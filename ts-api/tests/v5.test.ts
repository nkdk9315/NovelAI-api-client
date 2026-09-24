/**
 * V5 support tests: model helpers, validation, cost, usage, payload and the Qwen tokenizer.
 */
import { describe, it, expect, vi, beforeAll, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import sharp from 'sharp';
import * as Constants from '../src/constants';
import * as Schemas from '../src/schemas';
import { calculateGenerationCost, summarizeOpusUsage } from '../src/anlas';
import { NovelAIClient } from '../src/client';
import { NovelAIQwenTokenizer } from '../src/tokenizer';

describe('V5 model helpers', () => {
  it('isV5Model', () => {
    expect(Constants.isV5Model('nai-diffusion-5-full')).toBe(true);
    expect(Constants.isV5Model('nai-diffusion-5-curated')).toBe(true);
    expect(Constants.isV5Model('nai-diffusion-4-5-full')).toBe(false);
  });

  it('getInpaintModel maps V5 curated to the 4.5 curated inpainting model', () => {
    expect(Constants.getInpaintModel('nai-diffusion-5-full')).toBe('nai-diffusion-5-full-inpainting');
    expect(Constants.getInpaintModel('nai-diffusion-5-curated')).toBe('nai-diffusion-4-5-curated-inpainting');
    expect(Constants.getInpaintModel('nai-diffusion-4-5-full')).toBe('nai-diffusion-4-5-full-inpainting');
    expect(Constants.getInpaintModel('nai-diffusion-4-5-full-inpainting')).toBe('nai-diffusion-4-5-full-inpainting');
  });

  it('getMaxTokens returns the per-model limit', () => {
    expect(Constants.getMaxTokens('nai-diffusion-5-full')).toBe(1471);
    expect(Constants.getMaxTokens('nai-diffusion-5-curated')).toBe(703);
    expect(Constants.getMaxTokens('nai-diffusion-4-5-full')).toBe(512);
  });
});

describe('V5 validation', () => {
  const parse = (params: Record<string, unknown>) => Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', ...params });

  it('rejects vibes on V5', async () => {
    const r = await parse({ model: 'nai-diffusion-5-full', vibes: ['vibes/input1.naiv4vibe'] });
    expect(r.success).toBe(false);
  });

  it('rejects character_reference on V5', async () => {
    const r = await parse({ model: 'nai-diffusion-5-full', character_reference: { image: 'reference/input.jpeg' } });
    expect(r.success).toBe(false);
  });

  it('allows up to 32 characters on V5 but only 6 on V4.5', async () => {
    const chars = (n: number) => Array.from({ length: n }, () => ({ prompt: 'girl' }));
    expect((await parse({ model: 'nai-diffusion-5-full', characters: chars(32) })).success).toBe(true);
    expect((await parse({ model: 'nai-diffusion-5-full', characters: chars(33) })).success).toBe(false);
    expect((await parse({ model: 'nai-diffusion-4-5-full', characters: chars(6) })).success).toBe(true);
    expect((await parse({ model: 'nai-diffusion-4-5-full', characters: chars(7) })).success).toBe(false);
  });

  it('allows transparent_background only on V5', async () => {
    expect((await parse({ model: 'nai-diffusion-5-full', transparent_background: true })).success).toBe(true);
    expect((await parse({ model: 'nai-diffusion-4-5-full', transparent_background: true })).success).toBe(false);
  });

  it('buildEffectivePrompt appends the transparent tag once', () => {
    expect(Schemas.buildEffectivePrompt('1girl', true)).toBe('1girl, transparent background');
    expect(Schemas.buildEffectivePrompt('', true)).toBe('transparent background');
    expect(Schemas.buildEffectivePrompt('transparent background, 1girl', true)).toBe('transparent background, 1girl');
    expect(Schemas.buildEffectivePrompt('1girl', false)).toBe('1girl');
  });
});

describe('V5 cost', () => {
  it('applies the 1.5x multiplier (measured: 1088x1024 steps 1 → V5 6 / V4.5 4)', () => {
    const base = { width: 1088, height: 1024, steps: 1 };
    expect(calculateGenerationCost({ ...base, isV5: true }).totalCost).toBe(6);
    expect(calculateGenerationCost({ ...base, isV5: false }).totalCost).toBe(4);
    expect(calculateGenerationCost({ ...base, isV5: true }).modelMultiplier).toBe(1.5);
  });

  it('disables Opus free for V5 when the usage is exhausted', () => {
    const base = { width: 832, height: 1216, steps: 23, tier: 3 as const };
    expect(calculateGenerationCost({ ...base, isV5: true }).totalCost).toBe(0);
    const exhausted = calculateGenerationCost({ ...base, isV5: true, opusUsageExhausted: true });
    expect(exhausted.isOpusFree).toBe(false);
    expect(exhausted.totalCost).toBe(Math.ceil(17 * 1.5));
    // V4.5 has no usage limit
    expect(calculateGenerationCost({ ...base, isV5: false, opusUsageExhausted: true }).totalCost).toBe(0);
  });

  it('summarizeOpusUsage follows the official site formulas', () => {
    expect(summarizeOpusUsage({ percent: 100, isNegative: false, timeUntilNextPercent: 7888 })).toEqual({
      remainingPercent: 100,
      refillPercentPerDay: 11,
      estimatedImagesRemaining: 1730,
      isLow: false,
      isExhausted: false,
    });
    const empty = summarizeOpusUsage({ percent: 3, isNegative: true, timeUntilNextPercent: 0 });
    expect(empty.remainingPercent).toBe(0);
    expect(empty.refillPercentPerDay).toBe(0);
    expect(empty.isLow).toBe(true);
    expect(empty.isExhausted).toBe(true);
  });
});

describe('V5 request payload', () => {
  let client: NovelAIClient;
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    client = new NovelAIClient('test-api-key', { logger: { warn: () => {}, error: () => {} } });
    vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('skip balance'));
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    const png = await sharp({ create: { width: 64, height: 64, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } }).png().toBuffer();
    fetchMock.mockResolvedValue(new Response(new Uint8Array(png), { status: 200 }));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const sentBody = () => JSON.parse(fetchMock.mock.calls[0][1].body);

  it('txt2img with transparent background', async () => {
    await client.generate({
      prompt: '1girl',
      model: 'nai-diffusion-5-full',
      width: 512,
      height: 768,
      noise_schedule: 'exponential',
      transparent_background: true,
    });
    const body = sentBody();
    expect(body.model).toBe('nai-diffusion-5-full');
    expect(body.input).toBe('1girl, transparent background');
    expect(body.parameters.v4_prompt.caption.base_caption).toBe('1girl, transparent background');
    expect(body.parameters.params_version).toBe(4);
    expect(body.parameters.noise_schedule).toBe('karras');
    expect(body.parameters.straight_alpha).toBe(true);
    expect(body.parameters.tag_hint_transparent_background).toBe(true);
    expect(body.parameters.negative_prompt).toBe(Constants.DEFAULT_NEGATIVE_V5);
  });

  it('infill with V5 curated uses the 4.5 curated inpainting model', async () => {
    const png = await sharp({ create: { width: 512, height: 768, channels: 3, background: 'black' } }).png().toBuffer();
    await client.generate({
      prompt: '1girl',
      model: 'nai-diffusion-5-curated',
      width: 512,
      height: 768,
      action: 'infill',
      source_image: png,
      mask: png,
      mask_strength: 0.7,
    });
    expect(sentBody().model).toBe('nai-diffusion-4-5-curated-inpainting');
  });

  it('image_format webp is sent and detected from the returned bytes', async () => {
    const webp = await sharp({ create: { width: 64, height: 64, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } }).webp({ lossless: true }).toBuffer();
    fetchMock.mockResolvedValue(new Response(new Uint8Array(webp), { status: 200 }));
    const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'nai-webp-'));
    const result = await client.generate({ prompt: '1girl', model: 'nai-diffusion-5-full', width: 512, height: 768, image_format: 'webp', save_dir: dir });
    expect(sentBody().parameters.image_format).toBe('webp');
    expect(result.image_format).toBe('webp');
    expect(result.saved_path?.endsWith('.webp')).toBe(true);
  });

  it('defaults to png', async () => {
    const result = await client.generate({ prompt: '1girl', width: 512, height: 768 });
    expect(sentBody().parameters.image_format).toBe('png');
    expect(result.image_format).toBe('png');
  });

  it('V4.5 payload is unchanged (params_version 3, no transparency hints)', async () => {
    await client.generate({ prompt: '1girl', width: 512, height: 768 });
    const body = sentBody();
    expect(body.parameters.params_version).toBe(3);
    expect(body.parameters.straight_alpha).toBeUndefined();
    expect(body.parameters.negative_prompt).toBe(Constants.DEFAULT_NEGATIVE);
  });
});

describe('NovelAIQwenTokenizer (matches the official site encoder)', () => {
  let tokenizer: NovelAIQwenTokenizer | null = null;
  const expected: Array<{ text: string; ids: number[] }> = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'fixtures', 'qwen_expected.json'), 'utf-8')
  );

  beforeAll(() => {
    // Uses the on-disk cache only (downloaded on first real use); skipped when absent.
    const cachePath = path.join(__dirname, '..', '.cache', 'tokenizers', 'qwen35_tokenizer_v2.json');
    if (fs.existsSync(cachePath)) {
      tokenizer = new NovelAIQwenTokenizer(JSON.parse(fs.readFileSync(cachePath, 'utf-8')));
    }
  }, 60_000);

  it.each(expected.map(e => [e.text, e.ids] as const))('encodes %j', (text, ids) => {
    if (!tokenizer) return;
    expect(tokenizer.encode(text)).toEqual(ids);
  });
});
