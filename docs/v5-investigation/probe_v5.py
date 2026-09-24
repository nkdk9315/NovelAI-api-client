"""Probe V5 via the documented JSON body on /ai/generate-image (stdlib only)."""
import base64, io, json, os, struct, sys, time, urllib.request, urllib.error, zipfile, zlib

KEY = os.environ["NOVELAI_API_KEY"]
OUT = os.environ.get("PROBE_OUT", "/tmp/novelai_probe_out")
os.makedirs(OUT, exist_ok=True)
GEN = "https://image.novelai.net/ai/generate-image"
SUB = "https://image.novelai.net/user/subscription"
W, H = 512, 768


def req(url, body=None, accept=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(url, data=data, method="POST" if data else "GET")
    r.add_header("Authorization", f"Bearer {KEY}")
    r.add_header("User-Agent", "novelai-api-client-probe/0.1")
    if data:
        r.add_header("Content-Type", "application/json")
    if accept:
        r.add_header("Accept", accept)
    try:
        with urllib.request.urlopen(r, timeout=180) as res:
            return res.status, dict(res.headers), res.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()


def anlas():
    _, _, b = req(SUB)
    t = json.loads(b)["trainingStepsLeft"]
    return t["fixedTrainingStepsLeft"] + t["purchasedTrainingSteps"]


def png(w, h, rgba_fn):
    raw = b"".join(b"\x00" + b"".join(bytes(rgba_fn(x, y)) for x in range(w)) for y in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def describe(img):
    if img[:8] == b"\x89PNG\r\n\x1a\n":
        w, h, bd, ct = struct.unpack(">IIBB", img[16:26])
        return f"PNG {w}x{h} depth={bd} colorType={ct} ({'RGBA' if ct == 6 else 'RGB' if ct == 2 else ct})"
    if img[:4] == b"RIFF":
        return f"WEBP chunk={img[12:16]} flags={img[20]:08b}"
    return f"unknown {img[:8].hex()}"


def base_params(**kw):
    p = {
        "params_version": 4, "width": W, "height": H, "scale": 7, "sampler": "k_euler_ancestral",
        "steps": 23, "n_samples": 1, "seed": 12345, "noise_schedule": "karras", "cfg_rescale": 0,
        "dynamic_thresholding": False, "legacy": False, "add_original_image": True, "legacy_v3_extend": False,
        "use_coords": False, "legacy_uc": False, "normalize_reference_strength_multiple": True,
        "deliberate_euler_ancestral_bug": False, "prefer_brownian": True, "image_format": "png",
        "characterPrompts": [], "negative_prompt": "lowres, worst quality",
        "v4_prompt": {"caption": {"base_caption": PROMPT, "char_captions": []}, "use_coords": False, "use_order": True},
        "v4_negative_prompt": {"caption": {"base_caption": "lowres, worst quality", "char_captions": []}, "legacy_uc": False},
    }
    p.update(kw)
    return p


PROMPT = "1girl, solo, red apple in hand, simple, transparent background, very aesthetic, masterpiece, no text"


def run(name, body, accept=None):
    before = anlas()
    t0 = time.time()
    status, hdr, data = req(GEN, body, accept)
    after = anlas()
    print(f"== {name}: HTTP {status} ct={hdr.get('Content-Type')} {len(data)}B {time.time()-t0:.1f}s anlas {before}->{after}")
    if status >= 300:
        print("   body:", data[:500])
        return None
    if data[:2] == b"PK":
        z = zipfile.ZipFile(io.BytesIO(data))
        print("   zip entries:", z.namelist())
        img = z.read(z.namelist()[0])
    elif data[:1] == b"{":
        j = json.loads(data)
        print("   json keys:", list(j), [{k: (v if k != "image" else f"<{len(v)} b64>") for k, v in i.items()} for i in j.get("images", [])])
        img = base64.b64decode(j["images"][0]["image"])
    else:
        img = data
    print("   image:", describe(img))
    open(f"{OUT}/{name}.{'png' if img[:4] == b'\x89PNG' else 'webp'}", "wb").write(img)
    return img


which = sys.argv[1:] or ["t2i", "t2i_json", "i2i", "infill"]
state = {}
if "t2i" in which:
    body = {"input": PROMPT, "model": "nai-diffusion-5-full", "action": "generate",
            "parameters": base_params(straight_alpha=True, tag_hint_transparent_background=True,
                                      qualityToggle=True, ucPreset=0)}
    state["t2i"] = run("t2i_transparent", body)
if "t2i_json" in which:
    body = {"input": PROMPT, "model": "nai-diffusion-5-full", "action": "generate",
            "parameters": base_params(seed=222, image_format="webp", straight_alpha=True, tag_hint_transparent_background=True)}
    run("t2i_accept_json_webp", body, accept="application/json")
src = state.get("t2i") or open(f"{OUT}/t2i_transparent.png", "rb").read()
if "i2i" in which:
    body = {"input": PROMPT, "model": "nai-diffusion-5-full", "action": "img2img",
            "parameters": base_params(seed=333, image=base64.b64encode(src).decode(), strength=0.7, noise=0,
                                      extra_noise_seed=333, color_correct=False, straight_alpha=True,
                                      tag_hint_transparent_background=True)}
    run("i2i", body)
if "infill" in which:
    mask = png(W, H, lambda x, y: (255, 255, 255, 255) if (W // 4 <= x < 3 * W // 4 and y < H // 3) else (0, 0, 0, 255))
    open(f"{OUT}/mask.png", "wb").write(mask)
    body = {"input": PROMPT.replace("red apple in hand", "cat ears"), "model": "nai-diffusion-5-full-inpainting", "action": "infill",
            "parameters": base_params(seed=444, image=base64.b64encode(src).decode(), mask=base64.b64encode(mask).decode(),
                                      strength=0.7, noise=0, extra_noise_seed=444, add_original_image=False,
                                      inpaintImg2ImgStrength=1, straight_alpha=True, tag_hint_transparent_background=True)}
    run("infill", body)
