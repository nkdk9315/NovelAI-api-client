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
    const refType = ref.include_style ? "character&style" : "character";
    descriptions.push({
      caption: { base_caption: refType, char_captions: [] },
      legacy_uc: false
    });

    info_extracted.push(1.0);
    strength_values.push(1.0);
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
