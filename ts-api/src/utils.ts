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
// Image Helpers
// =============================================================================

/**
 * 画像データをBufferに変換
 */
export function getImageBuffer(image: string | Buffer): Buffer {
  if (Buffer.isBuffer(image)) {
    return image;
  }

  if (typeof image === 'string') {
    // Check if it's a file path
    if (fs.existsSync(image) && fs.lstatSync(image).isFile()) {
      return fs.readFileSync(image);
    }

    // Treat as Base64 string
    // Remove data URL prefix if present
    const base64Data = image.replace(/^data:image\/\w+;base64,/, "");
    return Buffer.from(base64Data, 'base64');
  }

  throw new Error(`Invalid image type: ${typeof image}`);
}

/**
 * 画像ファイルのサイズをチェック
 * @throws Error if file size exceeds MAX_REF_IMAGE_SIZE_MB
 */
export function validateImageFileSize(filePath: string): void {
  try {
    const stats = fs.statSync(filePath);
    const sizeMB = stats.size / (1024 * 1024);
    
    if (sizeMB > Constants.MAX_REF_IMAGE_SIZE_MB) {
      throw new Error(
        `Image file size (${sizeMB.toFixed(2)} MB) exceeds maximum allowed size (${Constants.MAX_REF_IMAGE_SIZE_MB} MB): ${filePath}`
      );
    }
  } catch (err: any) {
    if (err.message.includes('exceeds maximum allowed size')) {
      throw err; // Re-throw our validation error
    }
    // File doesn't exist or can't be read - will be handled by readFileSync later
  }
}

/**
 * 画像の存在確認と寸法を取得
 * @throws Error if image doesn't exist, cannot be read, or exceeds size limit
 */
export async function getImageDimensions(image: string | Buffer): Promise<{ width: number; height: number; buffer: Buffer }> {
  let buffer: Buffer;

  if (Buffer.isBuffer(image)) {
    // Check buffer size for in-memory images
    const sizeMB = image.length / (1024 * 1024);
    if (sizeMB > Constants.MAX_REF_IMAGE_SIZE_MB) {
      throw new Error(
        `Image buffer size (${sizeMB.toFixed(2)} MB) exceeds maximum allowed size (${Constants.MAX_REF_IMAGE_SIZE_MB} MB)`
      );
    }
    buffer = image;
  } else if (typeof image === 'string') {
    // Use helper to determine if this looks like a file path
    const isLikelyFilePath = looksLikeFilePath(image);
    
    if (isLikelyFilePath) {
      // Validate file size before reading
      validateImageFileSize(image);
      
      try {
        // Single atomic file read operation (avoids TOCTOU)
        buffer = fs.readFileSync(image);
      } catch (err) {
        throw new Error(`Image file not found or not readable: ${image}`);
      }
    } else {
      // Treat as Base64 string
      const base64Data = image.replace(/^data:image\/\w+;base64,/, "");
      buffer = Buffer.from(base64Data, 'base64');
      
      // Check decoded size
      const sizeMB = buffer.length / (1024 * 1024);
      if (sizeMB > Constants.MAX_REF_IMAGE_SIZE_MB) {
        throw new Error(
          `Decoded image size (${sizeMB.toFixed(2)} MB) exceeds maximum allowed size (${Constants.MAX_REF_IMAGE_SIZE_MB} MB)`
        );
      }
    }
  } else {
    throw new Error(`Invalid image type: ${typeof image}`);
  }

  // Get dimensions using sharp
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
  
  // Absolute paths (Unix or Windows)
  if (str.startsWith('/') || /^[A-Za-z]:[\\/]/.test(str)) {
    return true;
  }
  
  // Relative paths with directory separators and file extension
  // Check for common image extensions to reduce false positives
  if ((str.includes('/') || str.includes('\\')) && 
      /\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$/i.test(str)) {
    return true;
  }
  
  // Check if valid Base64: only contains Base64 chars and optional padding
  // A pure Base64 string without path-like characteristics
  const base64Regex = /^[A-Za-z0-9+/]+=*$/;
  if (base64Regex.test(str) && str.length > 100) {
    // Long string with only Base64 chars is likely Base64, not a path
    return false;
  }
  
  // If it has a file extension and no Base64-invalid chars for paths, assume path
  if (/\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$/i.test(str)) {
    return true;
  }
  
  // Default: if it contains directory separator, try as path
  return str.includes('/') || str.includes('\\');
}

/**
 * 画像をBase64文字列に変換
 */
export function getImageBase64(image: string | Buffer): string {
  if (Buffer.isBuffer(image)) {
    return image.toString('base64');
  }

  if (typeof image === 'string') {
    if (fs.existsSync(image) && fs.lstatSync(image).isFile()) {
      return fs.readFileSync(image).toString('base64');
    }
    // Assuming it's already base64, clean it just in case?
    // Or just return as is if it looks like base64
    return image.replace(/^data:image\/\w+;base64,/, "");
  }

  throw new Error(`Invalid image type: ${typeof image}`);
}


// =============================================================================
// Vibe Helpers
// =============================================================================

export function loadVibeFile(vibePath: string): any {
  const content = fs.readFileSync(vibePath, 'utf-8');
  return JSON.parse(content);
}

export function extractEncoding(
  vibeData: any,
  model: string = "nai-diffusion-4-5-full"
): { encoding: string; information_extracted: number } {
  const modelKey = Constants.MODEL_KEY_MAP[model] || "v4-5full";

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
    if (typeof vibe === 'object' && 'encoding' in vibe) {
      // VibeEncodeResult object
      encodings.push(vibe.encoding);
      info_extracted_list.push(vibe.information_extracted);
    } else if (typeof vibe === 'string') {
      if (vibe.endsWith('.naiv4vibe') && fs.existsSync(vibe)) {
        // File path
        const data = loadVibeFile(vibe);
        const { encoding, information_extracted } = extractEncoding(data, model);
        encodings.push(encoding);
        info_extracted_list.push(information_extracted);
      } else {
        // Base64 string (assumed)
        encodings.push(vibe);
        info_extracted_list.push(1.0);
      }
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
  if (aspectRatio < 0.8) {
    // Portrait
    targetWidth = Constants.CHARREF_PORTRAIT_SIZE.width;
    targetHeight = Constants.CHARREF_PORTRAIT_SIZE.height;
  } else if (aspectRatio > 1.25) {
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
  const maskWidth = Math.floor(width / 8);
  const maskHeight = Math.floor(height / 8);

  const centerX = center.x * maskWidth;
  const centerY = center.y * maskHeight;
  const radiusPx = radius * maskWidth;

  const canvas = Buffer.alloc(maskWidth * maskHeight, 0);

  for (let y = 0; y < maskHeight; y++) {
    for (let x = 0; x < maskWidth; x++) {
      const dist = Math.sqrt(Math.pow(x - centerX, 2) + Math.pow(y - centerY, 2));
      if (dist <= radiusPx) {
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
