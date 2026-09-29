/**
 * resizeMaskImage のテスト（実際の sharp を使用）
 */

import { describe, it, expect } from 'vitest';
import sharp from 'sharp';
import { resizeMaskImage, MASK_CELL } from '../src/utils';

/** PNG IHDR の color type (0 = grayscale) */
function pngColorType(png: Buffer): number {
  return png[25];
}

async function decodeGray(png: Buffer) {
  expect(pngColorType(png)).toBe(0);
  const { data, info } = await sharp(png)
    .extractChannel(0)
    .raw()
    .toBuffer({ resolveWithObject: true });
  return { data, info };
}

/** 白い円（アンチエイリアスあり）を黒背景に描いたマスク */
async function softCircleMask(width: number, height: number): Promise<Buffer> {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
    <rect width="100%" height="100%" fill="black"/>
    <circle cx="${width / 2 + 3}" cy="${height / 2 + 5}" r="${Math.min(width, height) / 3}" fill="white"/>
  </svg>`;
  return sharp(Buffer.from(svg)).png().toBuffer();
}

function assertBinaryUniformCells(data: Buffer, width: number, height: number, channels: number) {
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const v = data[(y * width + x) * channels];
      expect(v === 0 || v === 255).toBe(true);
      const cellOrigin =
        data[((y - (y % MASK_CELL)) * width + (x - (x % MASK_CELL))) * channels];
      if (v !== cellOrigin) {
        throw new Error(`cell not uniform at (${x}, ${y})`);
      }
    }
  }
}

describe('resizeMaskImage', () => {
  it('outputs a full-size, binary mask with uniform 8x8 cells from a soft full-size mask', async () => {
    const mask = await softCircleMask(832, 1216);
    const out = await resizeMaskImage(mask, 832, 1216);
    const { data, info } = await decodeGray(out);

    expect(info.width).toBe(832);
    expect(info.height).toBe(1216);
    expect(info.channels).toBe(1);
    assertBinaryUniformCells(data, info.width, info.height, info.channels);

    // 中心は白、四隅は黒
    expect(data[(608 * 832 + 416) * info.channels]).toBe(255);
    expect(data[0]).toBe(0);
  });

  it('upscales a 1/8-size mask to full size', async () => {
    const small = await softCircleMask(104, 152);
    const out = await resizeMaskImage(small, 832, 1216);
    const { data, info } = await decodeGray(out);

    expect(info.width).toBe(832);
    expect(info.height).toBe(1216);
    assertBinaryUniformCells(data, info.width, info.height, info.channels);
  });

  it('handles RGBA input with transparency', async () => {
    const rgba = await sharp({
      create: { width: 64, height: 64, channels: 4, background: { r: 255, g: 255, b: 255, alpha: 0.5 } },
    }).png().toBuffer();
    const out = await resizeMaskImage(rgba, 64, 64);
    const { data, info } = await decodeGray(out);

    expect(info.width).toBe(64);
    expect(info.height).toBe(64);
    expect(info.channels).toBe(1);
    assertBinaryUniformCells(data, 64, 64, 1);
  });
});
