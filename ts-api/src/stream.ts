/**
 * Incremental reading of the streaming endpoint (`stream: "msgpack"`), so a
 * caller can show the denoising previews while the image is generated.
 *
 * The body is `[u32 BE length][msgpack map]` frames: one `intermediate`
 * frame per sampling step (JPEG preview), then `final` (or `error`). The
 * whole body is still collected and parsed as before for the final image.
 */
import { Unpackr } from 'msgpackr';
import * as Constants from './constants';

/** One `intermediate` frame of the generation stream. */
export interface GenerateProgress {
  /** `step_ix`: the sampling step this preview comes from */
  step: number;
  /** `sigma`: noise level left at this step (falls toward 0) */
  sigma: number | null;
  /** Preview image (JPEG) */
  image: Buffer;
}

/** Receives each preview while `generate` runs with `onProgress`. */
export type ProgressCallback = (progress: GenerateProgress) => void;

const unpackr = new Unpackr({ useRecords: false });

/** The preview in a frame, if it is an `intermediate` event with an image. */
export function intermediatePreview(frame: Uint8Array): GenerateProgress | null {
  let val: unknown;
  try {
    val = unpackr.unpack(frame);
  } catch {
    return null;
  }
  if (!val || typeof val !== 'object') return null;
  const map = val as Record<string, unknown>;
  if ((map['event_type'] ?? map['event']) !== 'intermediate') return null;
  const image = map['image'];
  if (!(image instanceof Uint8Array)) return null;
  return {
    step: typeof map['step_ix'] === 'number' ? map['step_ix'] : 0,
    sigma: typeof map['sigma'] === 'number' ? map['sigma'] : null,
    image: Buffer.from(image),
  };
}

/** Finds the frames of a growing body as they complete. */
export class FrameScanner {
  /** Offset of the first frame not yet looked at */
  private next = 0;

  /**
   * Previews in the frames of `buf` that completed since the last call.
   * A body that is not framed (ZIP, bare PNG) yields nothing: its first
   * bytes read as a length far beyond what ever arrives.
   */
  scan(buf: Uint8Array): GenerateProgress[] {
    const out: GenerateProgress[] = [];
    for (;;) {
      if (this.next + 4 > buf.length) break;
      const view = new DataView(buf.buffer, buf.byteOffset + this.next, 4);
      const len = view.getUint32(0);
      const end = this.next + 4 + len;
      if (len === 0 || end > buf.length) break;
      const preview = intermediatePreview(buf.subarray(this.next + 4, end));
      if (preview) out.push(preview);
      this.next = end;
    }
    return out;
  }
}

/**
 * Read the whole body like `getResponseBuffer`, calling `onProgress` for each
 * preview as soon as its frame has arrived.
 */
export async function readWithProgress(
  response: Response,
  onProgress: ProgressCallback,
  maxSize: number = Constants.MAX_RESPONSE_SIZE,
): Promise<Buffer> {
  const contentLength = response.headers.get('content-length');
  if (contentLength && parseInt(contentLength, 10) > maxSize) {
    throw new Error(`Response too large: ${contentLength} bytes (max ${maxSize})`);
  }
  if (!response.body) return Buffer.from(await response.arrayBuffer());

  // Grow by doubling so appending chunks stays linear
  let buf = Buffer.alloc(64 * 1024);
  let length = 0;
  const scanner = new FrameScanner();
  const reader = response.body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    if (length + value.length > maxSize) {
      await reader.cancel();
      throw new Error(`Response too large: ${length + value.length} bytes (max ${maxSize})`);
    }
    if (length + value.length > buf.length) {
      const grown = Buffer.alloc(Math.max(buf.length * 2, length + value.length));
      buf.copy(grown, 0, 0, length);
      buf = grown;
    }
    buf.set(value, length);
    length += value.length;
    for (const preview of scanner.scan(buf.subarray(0, length))) onProgress(preview);
  }
  return buf.subarray(0, length);
}
