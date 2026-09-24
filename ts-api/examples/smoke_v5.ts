/**
 * V5 smoke test against the live API (small sizes to stay in the Opus free tier).
 * Usage: pnpm exec tsx examples/smoke_v5.ts [step...]
 */
import * as fs from "fs";
import * as path from "path";
import sharp from "sharp";
import { NovelAIClient } from "../src/client";
import { summarizeOpusUsage } from "../src/anlas";

const OUT = process.env.SMOKE_OUT ?? "/tmp/novelai_smoke/ts_v5";
fs.mkdirSync(OUT, { recursive: true });
const client = new NovelAIClient();
const steps = process.argv.slice(2);
const want = (s: string) => steps.length === 0 || steps.includes(s);
const base = { model: "nai-diffusion-5-full", width: 512, height: 768, steps: 23, prompt: "1girl, solo, red apple in hand, simple background" } as const;

async function step(name: string, fn: () => Promise<string>) {
  if (!want(name)) return;
  const t0 = Date.now();
  try {
    const info = await fn();
    console.log(`OK   ${name} (${((Date.now() - t0) / 1000).toFixed(1)}s) ${info}`);
  } catch (e: any) {
    console.log(`FAIL ${name} (${((Date.now() - t0) / 1000).toFixed(1)}s) ${e?.constructor?.name}: ${String(e?.message ?? e).slice(0, 300)}`);
  }
}

async function alphaRatio(file: string): Promise<string> {
  const { data, info } = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  let transparent = 0;
  for (let i = 3; i < data.length; i += 4) if (data[i] === 0) transparent++;
  return `${info.width}x${info.height} alpha0=${((transparent / (info.width * info.height)) * 100).toFixed(1)}%`;
}

const anlas = (r: { anlas_consumed?: number | null; anlas_remaining?: number | null }) => `anlas consumed=${r.anlas_consumed} remaining=${r.anlas_remaining}`;

(async () => {
  const src = path.join(OUT, "t2i.png");
  const mask = await sharp({ create: { width: 512, height: 768, channels: 3, background: "black" } })
    .composite([{ input: await sharp({ create: { width: 256, height: 256, channels: 3, background: "white" } }).png().toBuffer(), left: 128, top: 0 }])
    .png().toBuffer();

  await step("balance", async () => {
    const b = await client.getAnlasBalance();
    return `total=${b.total} tier=${b.tier} usage=${JSON.stringify(b.usage)} summary=${b.usage ? JSON.stringify(summarizeOpusUsage(b.usage)) : "-"}`;
  });
  await step("t2i_transparent", async () => {
    const r = await client.generate({ ...base, seed: 1, transparent_background: true, save_path: src });
    return `${await alphaRatio(src)} ${anlas(r)}`;
  });
  await step("t2i_webp", async () => {
    const out = path.join(OUT, "webp_dir");
    const r = await client.generate({ ...base, seed: 8, transparent_background: true, image_format: "webp", save_dir: out });
    const meta = await sharp(r.saved_path!).metadata();
    return `format=${r.image_format} file=${path.basename(r.saved_path!)} ${meta.format} ${meta.width}x${meta.height} alpha=${meta.hasAlpha} ${anlas(r)}`;
  });
  await step("i2i_from_webp", async () => {
    const webpSrc = fs.readdirSync(path.join(OUT, "webp_dir")).find(f => f.endsWith(".webp"));
    if (!webpSrc) return "skipped (no webp)";
    const r = await client.generate({ ...base, seed: 9, action: "img2img", source_image: path.join(OUT, "webp_dir", webpSrc), img2img_strength: 0.5, save_path: path.join(OUT, "i2i_from_webp.png") });
    return `format=${r.image_format} ${anlas(r)}`;
  });
  await step("i2i", async () => {
    const out = path.join(OUT, "i2i.png");
    const r = await client.generate({ ...base, seed: 2, action: "img2img", source_image: src, img2img_strength: 0.6, transparent_background: true, save_path: out });
    return `${await alphaRatio(out)} ${anlas(r)}`;
  });
  await step("infill_full", async () => {
    const out = path.join(OUT, "infill_full.png");
    const r = await client.generate({ ...base, prompt: base.prompt + ", cat ears", seed: 3, action: "infill", source_image: src, mask, mask_strength: 0.7, save_path: out });
    return `${await alphaRatio(out)} ${anlas(r)}`;
  });
  await step("t2i_curated", async () => {
    const r = await client.generate({ ...base, model: "nai-diffusion-5-curated", seed: 4, save_path: path.join(OUT, "curated.png") });
    return anlas(r);
  });
  await step("infill_curated", async () => {
    const r = await client.generate({ ...base, model: "nai-diffusion-5-curated", seed: 5, action: "infill", source_image: path.join(OUT, "curated.png"), mask, mask_strength: 0.7, save_path: path.join(OUT, "infill_curated.png") });
    return anlas(r);
  });
  await step("characters", async () => {
    const r = await client.generate({
      ...base, seed: 6, prompt: "2girls, standing, simple background",
      characters: [{ prompt: "girl, red hair", center_x: 0.3, center_y: 0.5 }, { prompt: "girl, blue hair", center_x: 0.7, center_y: 0.5 }],
      save_path: path.join(OUT, "characters.png"),
    });
    return anlas(r);
  });
  await step("vibe_rejected", async () => {
    try {
      await client.generate({ ...base, vibes: [path.resolve(__dirname, "../vibes/input1.naiv4vibe")] });
      return "UNEXPECTED: accepted";
    } catch (e: any) {
      return `rejected locally: ${String(e.message).includes("Vibe Transfer is not supported")}`;
    }
  });
  await step("declutter_keep_bubbles", async () => {
    const r = await client.augmentImage({ req_type: "declutter-keep-bubbles", image: src, save_path: path.join(OUT, "declutter.png") });
    return anlas(r);
  });
})();
