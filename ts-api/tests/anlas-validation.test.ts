/**
 * Anlas Pre-flight Balance Validation Tests
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import sharp from 'sharp';
import { NovelAIClient, InsufficientAnlasError, AnlasBalance } from '../src/client';

function makeBalance(total: number, tier: number = 0): AnlasBalance {
  return { fixed: total, purchased: 0, total, tier };
}

async function makePng(width: number, height: number): Promise<Buffer> {
  return sharp({ create: { width, height, channels: 3, background: { r: 0, g: 0, b: 0 } } }).png().toBuffer();
}

describe('Anlas pre-flight balance validation', () => {
  let client: NovelAIClient;

  beforeEach(() => {
    client = new NovelAIClient('test-api-key');
  });

  // -------------------------------------------------------------------------
  // generate()
  // -------------------------------------------------------------------------

  describe('generate()', () => {
    it('throws InsufficientAnlasError when balance is too low', async () => {
      // Default 832x1216 @ 23 steps costs 17 Anlas - balance of 5 is insufficient
      vi.spyOn(client, 'getAnlasBalance').mockResolvedValueOnce(makeBalance(5));

      await expect(
        client.generate({ prompt: '1girl' })
      ).rejects.toBeInstanceOf(InsufficientAnlasError);
    });

    it('does not throw when balance fetch fails (graceful degradation)', async () => {
      vi.spyOn(client, 'getAnlasBalance')
        .mockRejectedValue(new Error('Network error'));

      // Should throw something other than InsufficientAnlasError (e.g. network error from the API call)
      try {
        await client.generate({ prompt: '1girl' });
      } catch (e) {
        expect(e).not.toBeInstanceOf(InsufficientAnlasError);
      }
    });

    it('does not throw for Opus tier free generation (tier=3, small image)', async () => {
      // Opus free: 832x1216, 28 steps, tier=3 → totalCost = 0
      vi.spyOn(client, 'getAnlasBalance').mockResolvedValueOnce(makeBalance(0, 3));

      // API call itself will fail (no mock server), but the balance check should pass
      try {
        await client.generate({ prompt: '1girl', steps: 28 });
      } catch (e) {
        expect(e).not.toBeInstanceOf(InsufficientAnlasError);
      }
    });
  });

  // -------------------------------------------------------------------------
  // encodeVibe()
  // -------------------------------------------------------------------------

  describe('encodeVibe()', () => {
    it('throws InsufficientAnlasError when balance is 1 (cost is 2)', async () => {
      vi.spyOn(client, 'getAnlasBalance').mockResolvedValueOnce(makeBalance(1));

      const pngBytes = Buffer.from([
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
        0x00, 0x00, 0x04, 0x00, 0x01, 0xe2, 0x21, 0xbc,
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
      ]);

      await expect(
        client.encodeVibe({ image: pngBytes })
      ).rejects.toBeInstanceOf(InsufficientAnlasError);
    });

    it('does not throw when balance fetch fails', async () => {
      vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('Network error'));

      const pngBytes = Buffer.alloc(64).fill(0x89);

      try {
        await client.encodeVibe({ image: pngBytes });
      } catch (e) {
        expect(e).not.toBeInstanceOf(InsufficientAnlasError);
      }
    });
  });

  // -------------------------------------------------------------------------
  // augmentImage()
  // -------------------------------------------------------------------------

  describe('augmentImage()', () => {
    it('throws InsufficientAnlasError when balance is too low for augment', async () => {
      // A 1024x1024 colorize costs 20 Anlas - balance of 5 is insufficient
      vi.spyOn(client, 'getAnlasBalance').mockResolvedValueOnce(makeBalance(5));

      const pngBytes = await makePng(1024, 1024);

      await expect(
        client.augmentImage({ image: pngBytes, req_type: 'colorize', defry: 0 })
      ).rejects.toBeInstanceOf(InsufficientAnlasError);
    });

    it('does not throw when balance fetch fails', async () => {
      vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('Network error'));

      const pngBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47]);

      try {
        await client.augmentImage({ image: pngBytes, req_type: 'colorize' });
      } catch (e) {
        expect(e).not.toBeInstanceOf(InsufficientAnlasError);
      }
    });
  });

  // -------------------------------------------------------------------------
  // upscaleImage()
  // -------------------------------------------------------------------------

  describe('upscaleImage()', () => {
    it('throws InsufficientAnlasError when balance is 0 for upscale', async () => {
      vi.spyOn(client, 'getAnlasBalance').mockResolvedValueOnce(makeBalance(0));

      const pngBytes = await makePng(256, 256);

      await expect(
        client.upscaleImage({ image: pngBytes, scale: 4 })
      ).rejects.toBeInstanceOf(InsufficientAnlasError);
    });

    it('does not throw when balance fetch fails', async () => {
      vi.spyOn(client, 'getAnlasBalance').mockRejectedValue(new Error('Network error'));

      const pngBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47]);

      try {
        await client.upscaleImage({ image: pngBytes, scale: 4 });
      } catch (e) {
        expect(e).not.toBeInstanceOf(InsufficientAnlasError);
      }
    });
  });
});
