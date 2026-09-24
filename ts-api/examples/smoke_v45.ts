/**
 * V4.5 regression smoke test against the live API (small sizes to stay in the Opus free tier).
 * Usage: pnpm exec tsx examples/smoke_v45.ts [step...]
 */
import * as fs from "fs";
import * as path from "path";
import { NovelAIClient } from "../src/client";

const OUT = process.env.SMOKE_OUT ?? "/tmp/novelai_smoke/ts";
fs.mkdirSync(OUT, { recursive: true });
const MODEL = (process.env.SMOKE_MODEL ?? "nai-diffusion-4-5-full") as "nai-diffusion-4-5-full";
const client = new NovelAIClient();
const steps = process.argv.slice(2);
const want = (s: string) => steps.length === 0 || steps.includes(s);
const base = { model: MODEL, width: 512, height: 768, steps: 23, prompt: "1girl, solo, red apple in hand, simple background" } as const;

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

const anlas = (r: { anlas_consumed?: number | null; anlas_remaining?: number | null }) => `anlas consumed=${r.anlas_consumed} remaining=${r.anlas_remaining}`;

(async () => {
  const src = path.join(OUT, "t2i.png");
  await step("balance", async () => JSON.stringify(await client.getAnlasBalance()));
  await step("t2i", async () => {
    const r = await client.generate({ ...base, seed: 1, save_path: src });
    return `seed=${r.seed} ${anlas(r)}`;
  });
  await step("i2i", async () => {
    const r = await client.generate({ ...base, seed: 2, action: "img2img", source_image: src, img2img_strength: 0.6, save_path: path.join(OUT, "i2i.png") });
    return anlas(r);
  });
  await step("infill", async () => {
    const sharp = (await import("sharp")).default;
    const mask = await sharp({ create: { width: 512, height: 768, channels: 3, background: "black" } })
      .composite([{ input: await sharp({ create: { width: 256, height: 256, channels: 3, background: "white" } }).png().toBuffer(), left: 128, top: 0 }])
      .png().toBuffer();
    const r = await client.generate({ ...base, prompt: base.prompt + ", cat ears", seed: 3, action: "infill", source_image: src, mask, mask_strength: 0.7, save_path: path.join(OUT, "infill.png") });
    return anlas(r);
  });
  await step("vibe", async () => {
    const r = await client.generate({ ...base, seed: 4, vibes: [path.resolve(__dirname, "../vibes/input1.naiv4vibe")], save_path: path.join(OUT, "vibe.png") });
    return anlas(r);
  });
  await step("charref", async () => {
    const r = await client.generate({ ...base, seed: 5, character_reference: { image: path.resolve(__dirname, "../reference/input.jpeg") }, save_path: path.join(OUT, "charref.png") });
    return anlas(r);
  });
  await step("augment", async () => {
    const r = await client.augmentImage({ req_type: "sketch", image: src, save_path: path.join(OUT, "sketch.png") });
    return anlas(r);
  });
  await step("upscale", async () => {
    const r = await client.upscaleImage({ image: src, scale: 2, save_path: path.join(OUT, "upscale.png") });
    return `${r.output_width}x${r.output_height} ${anlas(r)}`;
  });
})();
