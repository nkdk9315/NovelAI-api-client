/**
 * NovelAI Client Utilities
 * 画像処理・ユーティリティ関数
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import sharp from 'sharp';
import * as Constants from './constants';
import { VibeEncodeResult, CharacterReferenceConfig } from './schemas';

// =============================================================================
// Custom Errors
// =============================================================================

export class ImageFileSizeError extends Error {
  public readonly fileSizeMB: number;
  public readonly maxSizeMB: number;
  constructor(fileSizeMB: number, maxSizeMB: number, source?: string) {
    const suffix = source ? `: ${source}` : '';
    super(`Image file size (${fileSizeMB.toFixed(2)} MB) exceeds maximum allowed size (${maxSizeMB} MB)${suffix}`);
    this.name = 'ImageFileSizeError';
    this.fileSizeMB = fileSizeMB;
    this.maxSizeMB = maxSizeMB;
  }
}

// =============================================================================
// Internal Helpers
// =============================================================================

const DATA_URL_PREFIX_REGEX = /^data:image\/[\w+.-]+;base64,/;

function sanitizeFilePath(filePath: string): string {
  const normalized = path.normalize(filePath);
  if (normalized.replace(/\\/g, '/').includes('..')) {
    throw new Error(`Invalid file path (path traversal detected): ${filePath}`);
  }
  return normalized;
}

function decodeBase64Image(base64Str: string): Buffer {
  const stripped = base64Str.replace(DATA_URL_PREFIX_REGEX, '');
  if (!/^[A-Za-z0-9+/]*=*$/.test(stripped) || stripped.length === 0) {
    throw new Error('Invalid Base64 string: contains characters outside the Base64 alphabet or is empty');
  }
  return Buffer.from(stripped, 'base64');
}

// =============================================================================
// Image Helpers
// =============================================================================

/**
 * 画像データのサイズを検証
 * @throws ImageFileSizeError if data exceeds MAX_REF_IMAGE_SIZE_MB
 */
export function validateImageDataSize(data: Buffer, source?: string): void {
  const sizeMB = data.length / (1024 * 1024);
  if (sizeMB > Constants.MAX_REF_IMAGE_SIZE_MB) {
    throw new ImageFileSizeError(sizeMB, Constants.MAX_REF_IMAGE_SIZE_MB, source);
  }
}

/**
 * 画像データをBufferに変換
 */
export function getImageBuffer(image: string | Buffer | Uint8Array): Buffer {
  if (Buffer.isBuffer(image)) {
    return image;
  }

  if (image instanceof Uint8Array) {
    return Buffer.from(image);
  }

  if (typeof image === 'string') {
    if (looksLikeFilePath(image)) {
      const safePath = sanitizeFilePath(image);
      try {
        return fs.readFileSync(safePath);
      } catch {
        throw new Error(`Image file not found or not readable: ${image}`);
      }
    }

    return decodeBase64Image(image);
  }

  throw new Error(`Invalid image type: ${typeof image}`);
}

/**
 * 画像の存在確認と寸法を取得
 * @throws Error if image doesn't exist, cannot be read, or exceeds size limit
 */
export async function getImageDimensions(image: string | Buffer | Uint8Array): Promise<{ width: number; height: number; buffer: Buffer }> {
  const buffer = getImageBuffer(image);
  validateImageDataSize(buffer, typeof image === 'string' && looksLikeFilePath(image) ? image : undefined);

  const metadata = await sharp(buffer).metadata();

  if (!metadata.width || !metadata.height) {
    throw new Error("Could not determine image dimensions. The file may be corrupted or not a valid image.");
  }

  return {
    width: metadata.width,
    height: metadata.height,
    buffer,
  };
}

/**
 * Heuristically determine if a string looks like a file path.
 * Base64 strings can contain '/' but typically don't look like paths.
 */
function looksLikeFilePath(str: string): boolean {
  // If it starts with data URL prefix, it's definitely not a path
  if (str.startsWith('data:')) {
    return false;
  }

  // Short-circuit: long Base64-only strings are not paths
  const base64Regex = /^[A-Za-z0-9+/\-_]+=*$/;
  if (base64Regex.test(str) && str.length > 64) {
    return false;
  }

  // Absolute paths (Unix or Windows) — require extension or directory structure
  if (str.startsWith('/')) {
    if (/\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$/i.test(str)) {
      return true;
    }
    // Has at least two path segments (e.g., /dir/file)
    if (/^\/[^/]+\//.test(str)) {
      return true;
    }
    return false;
  }
  if (/^[A-Za-z]:[\\/]/.test(str)) {
    return true;
  }

  // Relative paths with directory separators and file extension
  if ((str.includes('/') || str.includes('\\')) &&
    /\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$/i.test(str)) {
    return true;
  }

  // If it has a file extension, assume path
  if (/\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$/i.test(str)) {
    return true;
  }

  // Default: if it contains directory separator, try as path
  return str.includes('/') || str.includes('\\');
}

/**
 * 画像をBase64文字列に変換
 */
export function getImageBase64(image: string | Buffer | Uint8Array): string {
  return getImageBuffer(image).toString('base64');
}

/**
 * img2img用に画像を指定サイズにリサイズしてBase64に変換
 * サーバーは大きすぎる入力画像を処理できないため、出力寸法に合わせてリサイズする
 */
export async function resizeImageForImg2Img(
  image: string | Buffer | Uint8Array,
  targetWidth: number,
  targetHeight: number
): Promise<string> {
  const buffer = getImageBuffer(image);
  const resized = await sharp(buffer)
    .resize({
      width: targetWidth,
      height: targetHeight,
      fit: 'fill',
    })
    .png()
    .toBuffer();
  return resized.toString('base64');
}

/**
 * 画像Bufferを指定サイズにリサイズ（アスペクト比を無視して正確なサイズに変換）
 * Augmentの寸法クランプ用
 */
export async function resizeImageBuffer(
  buffer: Buffer,
  targetWidth: number,
  targetHeight: number
): Promise<Buffer> {
  return sharp(buffer)
    .resize({
      width: targetWidth,
      height: targetHeight,
      fit: 'fill',
    })
    .png()
    .toBuffer();
}


// =============================================================================
// Vibe Helpers
// =============================================================================

export function loadVibeFile(vibePath: string): any {
  const safePath = sanitizeFilePath(vibePath);
  const content = fs.readFileSync(safePath, 'utf-8');
  return JSON.parse(content);
}

export function extractEncoding(
  vibeData: any,
  model: string = "nai-diffusion-4-5-full"
): { encoding: string; information_extracted: number } {
  const modelKey = (Constants.MODEL_KEY_MAP as Record<string, string>)[model] || "v4-5full";

  const encodings = vibeData.encodings || {};
  const modelEncodings = encodings[modelKey] || {};

  const keys = Object.keys(modelEncodings);
  if (keys.length === 0) {
    throw new Error(`No encoding found for model key: ${modelKey}`);
  }

  const firstKey = keys[0];
  const encodingData = modelEncodings[firstKey];

  const encoding = encodingData.encoding;
  const params = encodingData.params || {};
  let information_extracted = params.information_extracted ?? 1.0;

  const importInfo = vibeData.importInfo || {};
  if (importInfo.information_extracted !== undefined) {
    information_extracted = importInfo.information_extracted;
  }

  return { encoding, information_extracted };
}

export async function processVibes(
  vibes: Array<string | VibeEncodeResult>,
  model: string
): Promise<{ encodings: string[]; info_extracted_list: number[] }> {
  const encodings: string[] = [];
  const info_extracted_list: number[] = [];

  for (const vibe of vibes) {
    if (vibe !== null && typeof vibe === 'object' && 'encoding' in vibe) {
      // VibeEncodeResult object
      encodings.push(vibe.encoding);
      info_extracted_list.push(vibe.information_extracted);
    } else if (typeof vibe === 'string') {
      if (vibe.endsWith('.naiv4vibe')) {
        // File path — loadVibeFile handles sanitization and throws on read failure
        const data = loadVibeFile(vibe);
        const { encoding, information_extracted } = extractEncoding(data, model);
        encodings.push(encoding);
        info_extracted_list.push(information_extracted);
      } else {
        // Base64 string (assumed)
        encodings.push(vibe);
        info_extracted_list.push(1.0);
      }
    } else {
      throw new Error(`Invalid vibe type: expected string or VibeEncodeResult, got ${typeof vibe}`);
    }
  }

  return { encodings, info_extracted_list };
}


// =============================================================================
// Character Reference Helpers
// =============================================================================

/**
 * キャラクター参照画像を適切なサイズに変換
 */
export async function prepareCharacterReferenceImage(imageBuffer: Buffer): Promise<Buffer> {
  const image = sharp(imageBuffer);
  const metadata = await image.metadata();

  if (!metadata.width || !metadata.height) {
    throw new Error("Could not get image dimensions");
  }

  const origWidth = metadata.width;
  const origHeight = metadata.height;
  const aspectRatio = origWidth / origHeight;

  let targetWidth: number;
  let targetHeight: number;

  // Choose target size based on aspect ratio
  if (aspectRatio < Constants.CHARREF_PORTRAIT_THRESHOLD) {
    // Portrait
    targetWidth = Constants.CHARREF_PORTRAIT_SIZE.width;
    targetHeight = Constants.CHARREF_PORTRAIT_SIZE.height;
  } else if (aspectRatio > Constants.CHARREF_LANDSCAPE_THRESHOLD) {
    // Landscape
    targetWidth = Constants.CHARREF_LANDSCAPE_SIZE.width;
    targetHeight = Constants.CHARREF_LANDSCAPE_SIZE.height;
  } else {
    // Square-ish
    targetWidth = Constants.CHARREF_SQUARE_SIZE.width;
    targetHeight = Constants.CHARREF_SQUARE_SIZE.height;
  }

  // Resize to fit within target dimensions while maintaining aspect ratio
  // sharp's 'contain' fit with background color works perfectly for padding
  const resized = await image
    .resize({
      width: targetWidth,
      height: targetHeight,
      fit: 'contain',
      background: { r: 0, g: 0, b: 0, alpha: 1 }
    })
    .png() // Ensure PNG output
    .toBuffer();

  return resized;
}

export async function processCharacterReferences(
  refs: CharacterReferenceConfig[]
): Promise<{
  images: string[];
  descriptions: any[];
  info_extracted: number[];
  strength_values: number[];
  secondary_strength_values: number[];
}> {
  const images: string[] = [];
  const descriptions: any[] = [];
  const info_extracted: number[] = [];
  const strength_values: number[] = [];
  const secondary_strength_values: number[] = [];

  for (const ref of refs) {
    const imageBuffer = getImageBuffer(ref.image);

    // Resize and pad
    const processedBuffer = await prepareCharacterReferenceImage(imageBuffer);
    const b64Image = processedBuffer.toString('base64');

    images.push(b64Image);

    // Style settings
    const refType = ref.mode;
    descriptions.push({
      caption: { base_caption: refType, char_captions: [] },
      legacy_uc: false
    });

    info_extracted.push(1.0);
    strength_values.push(ref.strength);
    secondary_strength_values.push(1.0 - ref.fidelity);
  }

  return {
    images,
    descriptions,
    info_extracted,
    strength_values,
    secondary_strength_values
  };
}


// =============================================================================
// Mask/Inpaint Helpers
// =============================================================================

/**
 * 画像データのSHA256ハッシュを計算（cache_secret_key用）
 */
export function calculateCacheSecretKey(imageData: Buffer): string {
  return crypto.createHash('sha256').update(imageData).digest('hex');
}

/**
 * マスク画像を1/8サイズにリサイズ（API仕様に合わせる）
 */
export async function resizeMaskImage(
  mask: Buffer,
  targetWidth: number,
  targetHeight: number
): Promise<Buffer> {
  // マスクは元画像の1/8サイズにリサイズ
  const maskWidth = Math.floor(targetWidth / 8);
  const maskHeight = Math.floor(targetHeight / 8);

  const resized = await sharp(mask)
    .resize({
      width: maskWidth,
      height: maskHeight,
      fit: 'fill', // Exact size
    })
    .grayscale() // Ensure grayscale
    .png()
    .toBuffer();

  return resized;
}

/**
 * 矩形領域のマスク画像をプログラマティックに生成
 * @param width 元画像の幅
 * @param height 元画像の高さ
 * @param region マスク領域（0.0-1.0の相対座標）
 * @returns マスク画像のBuffer（白=変更領域、黒=保持領域）
 */
export async function createRectangularMask(
  width: number,
  height: number,
  region: { x: number; y: number; w: number; h: number }
): Promise<Buffer> {
  if (width <= 0 || height <= 0) {
    throw new Error(`Invalid dimensions: width (${width}) and height (${height}) must be positive`);
  }
  for (const [key, val] of Object.entries(region) as [string, number][]) {
    if (val < 0.0 || val > 1.0) {
      throw new Error(`Invalid region.${key}: ${val} (must be between 0.0 and 1.0)`);
    }
  }

  // マスクサイズは元画像の1/8
  const maskWidth = Math.floor(width / 8);
  const maskHeight = Math.floor(height / 8);

  // 領域を絶対座標に変換
  const rectX = Math.floor(region.x * maskWidth);
  const rectY = Math.floor(region.y * maskHeight);
  const rectW = Math.floor(region.w * maskWidth);
  const rectH = Math.floor(region.h * maskHeight);

  // 黒背景のキャンバスを作成
  const canvas = Buffer.alloc(maskWidth * maskHeight, 0); // All black (0)

  // 指定領域を白（255）で塗りつぶし
  for (let y = rectY; y < Math.min(rectY + rectH, maskHeight); y++) {
    for (let x = rectX; x < Math.min(rectX + rectW, maskWidth); x++) {
      canvas[y * maskWidth + x] = 255;
    }
  }

  // sharpでPNGに変換
  const mask = await sharp(canvas, {
    raw: {
      width: maskWidth,
      height: maskHeight,
      channels: 1
    }
  })
    .png()
    .toBuffer();

  return mask;
}

/**
 * 円形領域のマスク画像をプログラマティックに生成
 * @param width 元画像の幅
 * @param height 元画像の高さ
 * @param center 中心座標（0.0-1.0の相対座標）
 * @param radius 半径（0.0-1.0、幅に対する相対値）
 * @returns マスク画像のBuffer
 */
export async function createCircularMask(
  width: number,
  height: number,
  center: { x: number; y: number },
  radius: number
): Promise<Buffer> {
  if (width <= 0 || height <= 0) {
    throw new Error(`Invalid dimensions: width (${width}) and height (${height}) must be positive`);
  }
  if (center.x < 0.0 || center.x > 1.0 || center.y < 0.0 || center.y > 1.0) {
    throw new Error(`Invalid center: (${center.x}, ${center.y}) (values must be between 0.0 and 1.0)`);
  }
  if (radius < 0.0 || radius > 1.0) {
    throw new Error(`Invalid radius: ${radius} (must be between 0.0 and 1.0)`);
  }

  const maskWidth = Math.floor(width / 8);
  const maskHeight = Math.floor(height / 8);

  const centerX = center.x * maskWidth;
  const centerY = center.y * maskHeight;
  const radiusPxSq = (radius * maskWidth) ** 2;

  const canvas = Buffer.alloc(maskWidth * maskHeight, 0);

  for (let y = 0; y < maskHeight; y++) {
    for (let x = 0; x < maskWidth; x++) {
      const dx = x - centerX;
      const dy = y - centerY;
      if (dx * dx + dy * dy <= radiusPxSq) {
        canvas[y * maskWidth + x] = 255;
      }
    }
  }

  const mask = await sharp(canvas, {
    raw: {
      width: maskWidth,
      height: maskHeight,
      channels: 1
    }
  })
    .png()
    .toBuffer();

  return mask;
}
