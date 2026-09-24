/**
 * Request format tests: generate / upscale must be sent as JSON bodies with base64 images inline.
 * (multipart makes the server treat image fields as part-name references.)
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import sharp from 'sharp';
import { NovelAIClient } from '../src/client';
import * as Constants from '../src/constants';

async function makePng(width: number, height: number): Promise<Buffer> {
  return sharp({ create: { width, height, channels: 3, background: { r: 0, g: 0, b: 0 } } }).png().toBuffer();
}

describe('request format', () => {
  let client: NovelAIClient;
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    client = new NovelAIClient('test-api-key', { logger: { warn: () => {}, error: () => {} } });
    vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('skip balance'));
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('generate (img2img) sends a JSON body to the stream endpoint with the image inline', async () => {
    const png = await makePng(512, 768);
    fetchMock.mockResolvedValue(new Response(new Uint8Array(png), { status: 200 }));

    await client.generate({
      prompt: '1girl',
      width: 512,
      height: 768,
      action: 'img2img',
      source_image: png,
      img2img_strength: 0.6,
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(Constants.STREAM_URL);
    expect(init.headers['Content-Type']).toBe('application/json');
    expect(init.headers['User-Agent']).toBe(Constants.USER_AGENT);
    expect(typeof init.body).toBe('string');

    const body = JSON.parse(init.body);
    expect(body.action).toBe('img2img');
    expect(body.parameters.image.startsWith('iVBOR')).toBe(true);
    expect(body.parameters.stream).toBe('msgpack');
    expect(body.parameters.image_cache_secret_key).toBeUndefined();
  });

  it('generate (infill) sends image and mask inline without cache keys', async () => {
    const png = await makePng(512, 768);
    fetchMock.mockResolvedValue(new Response(new Uint8Array(png), { status: 200 }));

    await client.generate({
      prompt: '1girl',
      width: 512,
      height: 768,
      action: 'infill',
      source_image: png,
      mask: await makePng(512, 768),
      mask_strength: 0.7,
    });

    const body = JSON.parse(fetchMock.mock.calls[0][1].body);
    expect(body.model).toBe('nai-diffusion-4-5-full-inpainting');
    expect(body.parameters.image.startsWith('iVBOR')).toBe(true);
    expect(body.parameters.mask.startsWith('iVBOR')).toBe(true);
    expect(body.parameters.image_cache_secret_key).toBeUndefined();
    expect(body.parameters.mask_cache_secret_key).toBeUndefined();
  });

  it('upscale sends {image, model, declared_blur_sigma} and reports the returned size', async () => {
    const input = await makePng(512, 768);
    const output = await makePng(1024, 1536);
    fetchMock.mockResolvedValue(new Response(new Uint8Array(output), { status: 200 }));

    const result = await client.upscaleImage({ image: input });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(Constants.UPSCALE_URL);
    const body = JSON.parse(init.body);
    expect(Object.keys(body).sort()).toEqual(['declared_blur_sigma', 'image', 'model']);
    expect(body.model).toBe(Constants.UPSCALE_MODEL);
    expect(body.declared_blur_sigma).toBe(0);
    expect(result.scale).toBe(2);
    expect(result.output_width).toBe(1024);
    expect(result.output_height).toBe(1536);
  });

  it('getAnlasBalance uses the image.novelai.net subscription endpoint', async () => {
    vi.mocked(client.getAnlasBalance).mockRestore();
    fetchMock.mockResolvedValue(new Response(JSON.stringify({
      tier: 3,
      trainingStepsLeft: { fixedTrainingStepsLeft: 0, purchasedTrainingSteps: 100 },
    }), { status: 200 }));

    const balance = await client.getAnlasBalance();

    expect(fetchMock.mock.calls[0][0]).toBe('https://image.novelai.net/user/subscription');
    expect(balance.total).toBe(100);
  });
});
