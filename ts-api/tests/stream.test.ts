/**
 * Streamed generation previews: frames are reported as soon as they are
 * complete, and the final image is still parsed from the whole body.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { pack } from 'msgpackr';
import sharp from 'sharp';
import { NovelAIClient, type GenerateProgress } from '../src/client';
import { FrameScanner, intermediatePreview, readWithProgress } from '../src/stream';

/** One `[u32 BE length][msgpack map]` frame of the streaming endpoint. */
function frame(fields: Record<string, unknown>): Buffer {
  const payload = pack(fields);
  const header = Buffer.alloc(4);
  header.writeUInt32BE(payload.length);
  return Buffer.concat([header, payload]);
}

const intermediate = (step: number, jpeg: string) =>
  frame({ event_type: 'intermediate', samp_ix: 0, step_ix: step, gen_id: 'g', sigma: 14.6 / (step + 1), image: Buffer.from(jpeg) });
const final = (image: Buffer) => frame({ event_type: 'final', samp_ix: 0, gen_id: 'g', image });

/** A response whose body arrives in the given pieces. */
function chunkedResponse(pieces: Buffer[]): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      for (const p of pieces) controller.enqueue(new Uint8Array(p));
      controller.close();
    },
  });
  return new Response(body, { status: 200 });
}

async function makePng(): Promise<Buffer> {
  return sharp({ create: { width: 16, height: 16, channels: 3, background: { r: 0, g: 0, b: 0 } } }).png().toBuffer();
}

describe('FrameScanner', () => {
  it('waits for whole frames', () => {
    const first = intermediate(0, 'a');
    const second = intermediate(1, 'b');
    const body = Buffer.concat([first, second, final(Buffer.from('png'))]);

    const scanner = new FrameScanner();
    // Only part of the first frame: nothing yet
    expect(scanner.scan(body.subarray(0, first.length - 1))).toEqual([]);
    // First frame complete, second cut short
    expect(scanner.scan(body.subarray(0, first.length + 3)).map((p) => p.step)).toEqual([0]);
    // The rest: the second preview once, and the final frame is not a preview
    const got = scanner.scan(body);
    expect(got).toHaveLength(1);
    expect(got[0].step).toBe(1);
    expect(got[0].image.toString()).toBe('b');
    expect(got[0].sigma).not.toBeNull();
    expect(scanner.scan(body)).toEqual([]);
  });

  it('ignores bodies that are not framed', async () => {
    const png = await makePng();
    expect(new FrameScanner().scan(png)).toEqual([]);
    expect(new FrameScanner().scan(Buffer.from('PK\x03\x04 zip data'))).toEqual([]);
  });

  it('only reports intermediate frames', () => {
    const payload = (f: Buffer) => f.subarray(4);
    expect(intermediatePreview(payload(intermediate(3, 'x')))).not.toBeNull();
    expect(intermediatePreview(payload(final(Buffer.from('x'))))).toBeNull();
    expect(intermediatePreview(payload(frame({ event_type: 'error', message: 'boom' })))).toBeNull();
    expect(intermediatePreview(Buffer.from('not msgpack'))).toBeNull();
  });
});

describe('readWithProgress', () => {
  it('reports previews across chunk boundaries and returns the whole body', async () => {
    const body = Buffer.concat([intermediate(0, 'a'), intermediate(1, 'b'), final(Buffer.from('png'))]);
    // Split at awkward places: inside a header and inside a payload
    const pieces = [body.subarray(0, 2), body.subarray(2, 20), body.subarray(20)];
    const seen: number[] = [];
    const out = await readWithProgress(chunkedResponse(pieces), (p) => seen.push(p.step));
    expect(seen).toEqual([0, 1]);
    expect(out.equals(body)).toBe(true);
  });

  it('rejects bodies over the size limit', async () => {
    const pieces = [Buffer.alloc(10), Buffer.alloc(10)];
    await expect(readWithProgress(chunkedResponse(pieces), () => {}, 15)).rejects.toThrow('Response too large');
  });
});

describe('generate with onProgress', () => {
  let client: NovelAIClient;

  beforeEach(() => {
    client = new NovelAIClient('test-api-key', { logger: { warn: () => {}, error: () => {} } });
    vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('skip balance'));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('passes each preview to onProgress and returns the final image', async () => {
    const png = await makePng();
    const body = Buffer.concat([intermediate(0, 'jpeg-0'), intermediate(1, 'jpeg-1'), final(png)]);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(chunkedResponse([body.subarray(0, 30), body.subarray(30)])));

    const seen: GenerateProgress[] = [];
    const result = await client.generate({ prompt: '1girl', seed: 7 }, { onProgress: (p) => seen.push(p) });

    expect(Buffer.from(result.image_data).equals(png)).toBe(true);
    expect(seen.map((p) => [p.step, p.image.toString()])).toEqual([[0, 'jpeg-0'], [1, 'jpeg-1']]);
  });

  it('without onProgress still returns the final image', async () => {
    const png = await makePng();
    const body = Buffer.concat([intermediate(0, 'jpeg-0'), final(png)]);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(chunkedResponse([body])));

    const result = await client.generate({ prompt: '1girl', seed: 7 });
    expect(Buffer.from(result.image_data).equals(png)).toBe(true);
  });
});
