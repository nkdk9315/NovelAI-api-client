/**
 * NovelAI Client Utils Tests
 * ユーティリティ関数のテスト
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as Constants from '../src/constants';

// =============================================================================
// Mocks（vi.hoisted で vi.mock ファクトリ内から参照可能にする）
// =============================================================================

const { mockReadFileSync, mockToBuffer, mockMetadata, mockSharp } = vi.hoisted(() => {
  const mockReadFileSync = vi.fn();
  const mockToBuffer = vi.fn().mockResolvedValue(Buffer.from('mock-png'));
  const mockMetadata = vi.fn().mockResolvedValue({ width: 100, height: 100 });
  const mockSharp = vi.fn(() => ({
    metadata: mockMetadata,
    resize: vi.fn().mockReturnThis(),
    png: vi.fn().mockReturnThis(),
    grayscale: vi.fn().mockReturnThis(),
    toBuffer: mockToBuffer,
  }));
  return { mockReadFileSync, mockToBuffer, mockMetadata, mockSharp };
});

vi.mock('fs', () => ({
  default: {
    readFileSync: (...args: any[]) => mockReadFileSync(...args),
  },
}));

vi.mock('sharp', () => ({ default: mockSharp }));

// モック後にインポート
import {
  ImageFileSizeError,
  validateImageDataSize,
  getImageBuffer,
  getImageBase64,
  getImageDimensions,
  loadVibeFile,
  extractEncoding,
  processVibes,
  createRectangularMask,
  createCircularMask,
  prepareCharacterReferenceImage,
} from '../src/utils';

beforeEach(() => {
  vi.clearAllMocks();
  mockMetadata.mockResolvedValue({ width: 100, height: 100 });
  mockToBuffer.mockResolvedValue(Buffer.from('mock-png'));
});

// =============================================================================
// A. ImageFileSizeError（カスタムエラークラス）
// =============================================================================
describe('ImageFileSizeError', () => {
  it('A-1: includes file path in message when source is provided', () => {
    const err = new ImageFileSizeError(15, 10, '/path/to/image.png');
    expect(err.message).toContain('/path/to/image.png');
  });

  it('A-2: message does not contain colon suffix when source is omitted', () => {
    const err = new ImageFileSizeError(15, 10);
    expect(err.message).not.toMatch(/:\s*$/);
    expect(err.message).toContain('15.00 MB');
    expect(err.message).toContain('10 MB');
  });

  it('A-3: name is ImageFileSizeError', () => {
    const err = new ImageFileSizeError(15, 10);
    expect(err.name).toBe('ImageFileSizeError');
  });

  it('A-4: fileSizeMB and maxSizeMB are correctly stored', () => {
    const err = new ImageFileSizeError(15.5, 10);
    expect(err.fileSizeMB).toBe(15.5);
    expect(err.maxSizeMB).toBe(10);
  });

  it('A-5: inherits from Error', () => {
    const err = new ImageFileSizeError(15, 10);
    expect(err).toBeInstanceOf(Error);
  });
});

// =============================================================================
// B. validateImageDataSize
// =============================================================================
describe('validateImageDataSize', () => {
  it('B-1: does not throw for buffer within size limit', () => {
    const buf = Buffer.alloc(1024); // 1KB
    expect(() => validateImageDataSize(buf)).not.toThrow();
  });

  it('B-2: throws ImageFileSizeError for buffer exceeding limit', () => {
    const size = (Constants.MAX_REF_IMAGE_SIZE_MB + 1) * 1024 * 1024;
    const buf = Buffer.alloc(size);
    expect(() => validateImageDataSize(buf)).toThrow(ImageFileSizeError);
  });

  it('B-3: error message includes source when provided', () => {
    const size = (Constants.MAX_REF_IMAGE_SIZE_MB + 1) * 1024 * 1024;
    const buf = Buffer.alloc(size);
    expect(() => validateImageDataSize(buf, 'test.png')).toThrow(/test\.png/);
  });

  it('B-4: exactly MAX_REF_IMAGE_SIZE_MB passes', () => {
    const size = Constants.MAX_REF_IMAGE_SIZE_MB * 1024 * 1024;
    const buf = Buffer.alloc(size);
    expect(() => validateImageDataSize(buf)).not.toThrow();
  });

  it('B-5: empty buffer passes', () => {
    const buf = Buffer.alloc(0);
    expect(() => validateImageDataSize(buf)).not.toThrow();
  });
});

// =============================================================================
// C. getImageBuffer — Buffer/Uint8Array入力
// =============================================================================
describe('getImageBuffer — Buffer/Uint8Array input', () => {
  it('C-1: returns Buffer input as-is', () => {
    const buf = Buffer.from('test-image');
    const result = getImageBuffer(buf);
    expect(result).toBe(buf); // same reference
  });

  it('C-2: converts Uint8Array to Buffer', () => {
    const arr = new Uint8Array([1, 2, 3, 4]);
    const result = getImageBuffer(arr);
    expect(Buffer.isBuffer(result)).toBe(true);
    expect(result).toEqual(Buffer.from([1, 2, 3, 4]));
  });

  it('C-3: throws for invalid type (number)', () => {
    expect(() => getImageBuffer(42 as any)).toThrow(/Invalid image type/);
  });
});

// =============================================================================
// D. getImageBuffer — ファイルパス入力（sanitizeFilePath間接テスト）
// =============================================================================
describe('getImageBuffer — file path input (sanitizeFilePath)', () => {
  it('D-1: calls readFileSync for a normal file path', () => {
    const mockBuf = Buffer.from('image-data');
    mockReadFileSync.mockReturnValue(mockBuf);

    const result = getImageBuffer('/images/test.png');
    expect(mockReadFileSync).toHaveBeenCalled();
    expect(result).toEqual(mockBuf);
  });

  it('D-2: throws for path traversal ../../etc/passwd', () => {
    expect(() => getImageBuffer('../../etc/passwd')).toThrow(/path traversal detected/);
  });

  it('D-3: throws for path traversal images/../../../secret', () => {
    expect(() => getImageBuffer('images/../../../secret')).toThrow(/path traversal detected/);
  });

  it('D-4: throws "not found or not readable" when file does not exist', () => {
    mockReadFileSync.mockImplementation(() => { throw new Error('ENOENT'); });
    expect(() => getImageBuffer('/nonexistent/image.png')).toThrow(/not found or not readable/);
  });
});

// =============================================================================
// E. getImageBuffer — Base64入力（decodeBase64Image間接テスト）
// =============================================================================
describe('getImageBuffer — Base64 input (decodeBase64Image)', () => {
  it('E-1: decodes a valid Base64 string', () => {
    const original = Buffer.from('hello world');
    const b64 = original.toString('base64');
    // Make it long enough (>64) to avoid file-path heuristic, or use a data URL
    const result = getImageBuffer(`data:image/png;base64,${b64}`);
    expect(result).toEqual(original);
  });

  it('E-2: strips data:image/png;base64, prefix correctly', () => {
    const original = Buffer.from('test-data');
    const b64 = original.toString('base64');
    const result = getImageBuffer(`data:image/png;base64,${b64}`);
    expect(result).toEqual(original);
  });

  it('E-3: strips data:image/svg+xml;base64, prefix', () => {
    const original = Buffer.from('svg-content');
    const b64 = original.toString('base64');
    const result = getImageBuffer(`data:image/svg+xml;base64,${b64}`);
    expect(result).toEqual(original);
  });

  it('E-4: throws for invalid Base64 characters (!@#$)', () => {
    const invalidB64 = 'data:image/png;base64,abc!@#$def';
    expect(() => getImageBuffer(invalidB64)).toThrow(/Invalid Base64/);
  });

  it('E-5: throws for empty string', () => {
    // Empty string doesn't match path heuristics (no extension, no separator)
    // looksLikeFilePath returns false for empty, so it goes to decodeBase64Image
    // Actually, empty string: looksLikeFilePath('') -> false (no data:, no /, no \, no extension)
    // Then decodeBase64Image('') -> stripped is '', length === 0 -> throw
    expect(() => getImageBuffer('')).toThrow(/Invalid Base64/);
  });
});

// =============================================================================
// F. getImageBase64
// =============================================================================
describe('getImageBase64', () => {
  it('F-1: converts Buffer to Base64 string', () => {
    const buf = Buffer.from('hello');
    const result = getImageBase64(buf);
    expect(result).toBe(buf.toString('base64'));
  });

  it('F-2: delegates to getImageBuffer (result is Base64-encoded)', () => {
    const arr = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
    const result = getImageBase64(arr);
    expect(typeof result).toBe('string');
    expect(Buffer.from(result, 'base64').toString()).toBe('Hello');
  });
});

// =============================================================================
// G. getImageDimensions
// =============================================================================
describe('getImageDimensions', () => {
  it('G-1: returns width, height, and buffer for valid input', async () => {
    const buf = Buffer.from('mock-image');
    mockMetadata.mockResolvedValue({ width: 200, height: 300 });

    const result = await getImageDimensions(buf);
    expect(result.width).toBe(200);
    expect(result.height).toBe(300);
    expect(Buffer.isBuffer(result.buffer)).toBe(true);
  });

  it('G-2: throws ImageFileSizeError for oversized buffer', async () => {
    const size = (Constants.MAX_REF_IMAGE_SIZE_MB + 1) * 1024 * 1024;
    const buf = Buffer.alloc(size);
    await expect(getImageDimensions(buf)).rejects.toThrow(ImageFileSizeError);
  });

  it('G-3: throws when metadata has no width/height', async () => {
    const buf = Buffer.from('mock-image');
    mockMetadata.mockResolvedValue({ width: undefined, height: undefined });

    await expect(getImageDimensions(buf)).rejects.toThrow(/Could not determine image dimensions/);
  });
});

// =============================================================================
// H. looksLikeFilePath（getImageBuffer経由の間接テスト）
// =============================================================================
describe('looksLikeFilePath (indirect via getImageBuffer)', () => {
  it('H-1: data:image/png;base64,... is treated as Base64 (not path)', () => {
    const b64 = Buffer.from('test').toString('base64');
    const input = `data:image/png;base64,${b64}`;
    // Should not throw path-related errors, should decode Base64
    const result = getImageBuffer(input);
    expect(result).toEqual(Buffer.from('test'));
  });

  it('H-2: long (>64 chars) Base64-only string is treated as Base64', () => {
    // Create a >64 char Base64 string
    const longB64 = Buffer.from('x'.repeat(100)).toString('base64');
    expect(longB64.length).toBeGreaterThan(64);
    const result = getImageBuffer(longB64);
    expect(Buffer.isBuffer(result)).toBe(true);
  });

  it('H-3: /image.png is treated as file path', () => {
    mockReadFileSync.mockReturnValue(Buffer.from('file-data'));
    getImageBuffer('/image.png');
    expect(mockReadFileSync).toHaveBeenCalled();
  });

  it('H-4: /dir/file is treated as file path', () => {
    mockReadFileSync.mockReturnValue(Buffer.from('file-data'));
    getImageBuffer('/dir/file');
    expect(mockReadFileSync).toHaveBeenCalled();
  });

  it('H-5: C:\\images\\test.png is treated as file path', () => {
    mockReadFileSync.mockReturnValue(Buffer.from('file-data'));
    getImageBuffer('C:\\images\\test.png');
    expect(mockReadFileSync).toHaveBeenCalled();
  });

  it('H-6: images/test.png is treated as file path', () => {
    mockReadFileSync.mockReturnValue(Buffer.from('file-data'));
    getImageBuffer('images/test.png');
    expect(mockReadFileSync).toHaveBeenCalled();
  });

  it('H-7: test.png is treated as file path', () => {
    mockReadFileSync.mockReturnValue(Buffer.from('file-data'));
    getImageBuffer('test.png');
    expect(mockReadFileSync).toHaveBeenCalled();
  });
});

// =============================================================================
// I. loadVibeFile
// =============================================================================
describe('loadVibeFile', () => {
  it('I-1: parses JSON from a valid file path', () => {
    const vibeData = { encodings: { v4full: {} } };
    mockReadFileSync.mockReturnValue(JSON.stringify(vibeData));

    const result = loadVibeFile('/path/to/vibe.naiv4vibe');
    expect(result).toEqual(vibeData);
    expect(mockReadFileSync).toHaveBeenCalled();
  });

  it('I-2: throws for path traversal', () => {
    expect(() => loadVibeFile('../../etc/secret.naiv4vibe')).toThrow(/path traversal detected/);
  });

  it('I-3: throws for non-existent file', () => {
    mockReadFileSync.mockImplementation(() => { throw new Error('ENOENT'); });
    expect(() => loadVibeFile('/nonexistent/vibe.naiv4vibe')).toThrow();
  });
});

// =============================================================================
// J. extractEncoding
// =============================================================================
describe('extractEncoding', () => {
  const makeVibeData = (modelKey: string, encoding: string, params?: any, importInfo?: any) => ({
    encodings: {
      [modelKey]: {
        someKey: { encoding, params: params || {} },
      },
    },
    ...(importInfo ? { importInfo } : {}),
  });

  it('J-1: extracts encoding and information_extracted from vibe data', () => {
    const data = makeVibeData('v4-5full', 'abc123', { information_extracted: 0.8 });
    const result = extractEncoding(data);
    expect(result.encoding).toBe('abc123');
    expect(result.information_extracted).toBe(0.8);
  });

  it('J-2: importInfo.information_extracted takes priority', () => {
    const data = makeVibeData('v4-5full', 'abc123', { information_extracted: 0.5 }, { information_extracted: 0.9 });
    const result = extractEncoding(data);
    expect(result.information_extracted).toBe(0.9);
  });

  it('J-3: throws for non-existent model key', () => {
    const data = { encodings: {} };
    expect(() => extractEncoding(data, 'nai-diffusion-4-5-full')).toThrow(/No encoding found/);
  });

  it('J-4: defaults to nai-diffusion-4-5-full model', () => {
    const data = makeVibeData('v4-5full', 'default-enc');
    const result = extractEncoding(data); // no model arg
    expect(result.encoding).toBe('default-enc');
  });
});

// =============================================================================
// K. processVibes
// =============================================================================
describe('processVibes', () => {
  it('K-1: processes VibeEncodeResult objects correctly', async () => {
    const vibes = [
      {
        encoding: 'enc1',
        information_extracted: 0.7,
        model: 'nai-diffusion-4-5-full' as const,
        strength: 0.5,
        source_image_hash: 'a'.repeat(64),
        created_at: new Date(),
      },
    ];
    const result = await processVibes(vibes, 'nai-diffusion-4-5-full');
    expect(result.encodings).toEqual(['enc1']);
    expect(result.info_extracted_list).toEqual([0.7]);
  });

  it('K-2: processes .naiv4vibe file path via loadVibeFile', async () => {
    const vibeData = {
      encodings: {
        'v4-5full': {
          key1: { encoding: 'file-enc', params: { information_extracted: 0.6 } },
        },
      },
    };
    mockReadFileSync.mockReturnValue(JSON.stringify(vibeData));

    const result = await processVibes(['test.naiv4vibe'], 'nai-diffusion-4-5-full');
    expect(result.encodings).toEqual(['file-enc']);
    expect(result.info_extracted_list).toEqual([0.6]);
  });

  it('K-3: processes Base64 string with info_extracted=1.0', async () => {
    const result = await processVibes(['someBase64EncodedString'], 'nai-diffusion-4-5-full');
    expect(result.encodings).toEqual(['someBase64EncodedString']);
    expect(result.info_extracted_list).toEqual([1.0]);
  });

  it('K-4: throws for invalid type (number)', async () => {
    await expect(processVibes([42 as any], 'nai-diffusion-4-5-full')).rejects.toThrow(/Invalid vibe type/);
  });

  it('K-5: throws for null entry', async () => {
    await expect(processVibes([null as any], 'nai-diffusion-4-5-full')).rejects.toThrow(/Invalid vibe type/);
  });

  it('K-6: empty array returns empty results', async () => {
    const result = await processVibes([], 'nai-diffusion-4-5-full');
    expect(result.encodings).toEqual([]);
    expect(result.info_extracted_list).toEqual([]);
  });
});

// =============================================================================
// L. createRectangularMask — バリデーション
// =============================================================================
describe('createRectangularMask', () => {
  it('L-1: throws for width=0', async () => {
    await expect(createRectangularMask(0, 100, { x: 0, y: 0, w: 1, h: 1 }))
      .rejects.toThrow(/Invalid dimensions/);
  });

  it('L-2: throws for height=-1', async () => {
    await expect(createRectangularMask(100, -1, { x: 0, y: 0, w: 1, h: 1 }))
      .rejects.toThrow(/Invalid dimensions/);
  });

  it('L-3: throws for region.x > 1.0', async () => {
    await expect(createRectangularMask(100, 100, { x: 1.5, y: 0, w: 0.5, h: 0.5 }))
      .rejects.toThrow(/Invalid region\.x/);
  });

  it('L-4: throws for region.y < 0.0', async () => {
    await expect(createRectangularMask(100, 100, { x: 0, y: -0.1, w: 0.5, h: 0.5 }))
      .rejects.toThrow(/Invalid region\.y/);
  });

  it('L-5: returns Buffer for valid input', async () => {
    const result = await createRectangularMask(800, 600, { x: 0.1, y: 0.1, w: 0.5, h: 0.5 });
    expect(Buffer.isBuffer(result)).toBe(true);
  });
});

// =============================================================================
// M. createCircularMask — バリデーション
// =============================================================================
describe('createCircularMask', () => {
  it('M-1: throws for width=0', async () => {
    await expect(createCircularMask(0, 100, { x: 0.5, y: 0.5 }, 0.3))
      .rejects.toThrow(/Invalid dimensions/);
  });

  it('M-2: throws for center.x > 1.0', async () => {
    await expect(createCircularMask(100, 100, { x: 1.5, y: 0.5 }, 0.3))
      .rejects.toThrow(/Invalid center/);
  });

  it('M-3: throws for radius < 0', async () => {
    await expect(createCircularMask(100, 100, { x: 0.5, y: 0.5 }, -0.1))
      .rejects.toThrow(/Invalid radius/);
  });

  it('M-4: throws for radius > 1.0', async () => {
    await expect(createCircularMask(100, 100, { x: 0.5, y: 0.5 }, 1.5))
      .rejects.toThrow(/Invalid radius/);
  });

  it('M-5: returns Buffer for valid input', async () => {
    const result = await createCircularMask(800, 600, { x: 0.5, y: 0.5 }, 0.3);
    expect(Buffer.isBuffer(result)).toBe(true);
  });
});

// =============================================================================
// N. prepareCharacterReferenceImage — 定数閾値テスト
// =============================================================================
describe('prepareCharacterReferenceImage', () => {
  it('N-1: aspect ratio < 0.8 (Portrait) uses CHARREF_PORTRAIT_SIZE', async () => {
    // 400/600 = 0.667 < 0.8 → Portrait
    mockMetadata.mockResolvedValue({ width: 400, height: 600 });

    await prepareCharacterReferenceImage(Buffer.from('img'));

    const resizeCall = mockSharp.mock.results[0].value.resize;
    expect(resizeCall).toHaveBeenCalledWith(expect.objectContaining({
      width: Constants.CHARREF_PORTRAIT_SIZE.width,
      height: Constants.CHARREF_PORTRAIT_SIZE.height,
    }));
  });

  it('N-2: aspect ratio > 1.25 (Landscape) uses CHARREF_LANDSCAPE_SIZE', async () => {
    // 800/400 = 2.0 > 1.25 → Landscape
    mockMetadata.mockResolvedValue({ width: 800, height: 400 });

    await prepareCharacterReferenceImage(Buffer.from('img'));

    const resizeCall = mockSharp.mock.results[0].value.resize;
    expect(resizeCall).toHaveBeenCalledWith(expect.objectContaining({
      width: Constants.CHARREF_LANDSCAPE_SIZE.width,
      height: Constants.CHARREF_LANDSCAPE_SIZE.height,
    }));
  });

  it('N-3: aspect ratio 0.8-1.25 (Square) uses CHARREF_SQUARE_SIZE', async () => {
    // 500/500 = 1.0, between 0.8 and 1.25 → Square
    mockMetadata.mockResolvedValue({ width: 500, height: 500 });

    await prepareCharacterReferenceImage(Buffer.from('img'));

    const resizeCall = mockSharp.mock.results[0].value.resize;
    expect(resizeCall).toHaveBeenCalledWith(expect.objectContaining({
      width: Constants.CHARREF_SQUARE_SIZE.width,
      height: Constants.CHARREF_SQUARE_SIZE.height,
    }));
  });
});
