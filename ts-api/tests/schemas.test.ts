/**
 * NovelAI Client Schema Validation Tests
 * Zodスキーマのバリデーションテスト
 */

/// <reference types="node" />

import { describe, it, expect } from 'vitest';
import { z } from 'zod';
import * as Schemas from '../src/schemas';
import * as Constants from '../src/constants';

// =============================================================================
// CharacterConfigSchema Tests
// =============================================================================
describe('CharacterConfigSchema', () => {
  it('should validate valid character config', () => {
    const config = {
      prompt: '1girl, beautiful',
      center_x: 0.5,
      center_y: 0.5,
      negative_prompt: 'lowres',
    };
    const result = Schemas.CharacterConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
  });

  it('should apply defaults for center_x, center_y, negative_prompt', () => {
    const config = { prompt: '1girl' };
    const result = Schemas.CharacterConfigSchema.parse(config);
    expect(result.center_x).toBe(0.5);
    expect(result.center_y).toBe(0.5);
    expect(result.negative_prompt).toBe('');
  });

  it('should reject empty prompt', () => {
    const config = { prompt: '' };
    const result = Schemas.CharacterConfigSchema.safeParse(config);
    expect(result.success).toBe(false);
  });

  it('should reject prompt exceeding max chars', () => {
    const config = { prompt: 'a'.repeat(Constants.MAX_PROMPT_CHARS + 1) };
    const result = Schemas.CharacterConfigSchema.safeParse(config);
    expect(result.success).toBe(false);
  });

  it('should reject center_x outside 0-1 range', () => {
    expect(Schemas.CharacterConfigSchema.safeParse({ prompt: '1girl', center_x: -0.1 }).success).toBe(false);
    expect(Schemas.CharacterConfigSchema.safeParse({ prompt: '1girl', center_x: 1.1 }).success).toBe(false);
  });

  it('should reject center_y outside 0-1 range', () => {
    expect(Schemas.CharacterConfigSchema.safeParse({ prompt: '1girl', center_y: -0.1 }).success).toBe(false);
    expect(Schemas.CharacterConfigSchema.safeParse({ prompt: '1girl', center_y: 1.1 }).success).toBe(false);
  });
});

// =============================================================================
// CharacterReferenceConfigSchema Tests
// =============================================================================
describe('CharacterReferenceConfigSchema', () => {
  it('should validate with string image input', () => {
    const config = { image: 'path/to/image.png' };
    const result = Schemas.CharacterReferenceConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
  });

  it('should validate with Buffer image input', () => {
    const config = { image: Buffer.from('test') };
    const result = Schemas.CharacterReferenceConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
  });

  it('should apply defaults for fidelity and include_style', () => {
    const config = { image: 'test.png' };
    const result = Schemas.CharacterReferenceConfigSchema.parse(config);
    expect(result.fidelity).toBe(1.0);
    expect(result.include_style).toBe(true);
  });

  it('should reject fidelity outside 0-1 range', () => {
    expect(Schemas.CharacterReferenceConfigSchema.safeParse({ image: 'test.png', fidelity: -0.1 }).success).toBe(false);
    expect(Schemas.CharacterReferenceConfigSchema.safeParse({ image: 'test.png', fidelity: 1.1 }).success).toBe(false);
  });
});

// =============================================================================
// GenerateParamsSchema Tests
// =============================================================================
describe('GenerateParamsSchema', () => {
  describe('基本バリデーション', () => {
    it('should validate minimal params (prompt only)', async () => {
      const params = { prompt: '1girl' };
      const result = await Schemas.GenerateParamsSchema.safeParseAsync(params);
      expect(result.success).toBe(true);
    });

    it('should apply all defaults correctly', async () => {
      const params = { prompt: '1girl' };
      const result = await Schemas.GenerateParamsSchema.parseAsync(params);
      expect(result.action).toBe('generate');
      expect(result.model).toBe(Constants.DEFAULT_MODEL);
      expect(result.width).toBe(Constants.DEFAULT_WIDTH);
      expect(result.height).toBe(Constants.DEFAULT_HEIGHT);
      expect(result.steps).toBe(Constants.DEFAULT_STEPS);
      expect(result.scale).toBe(Constants.DEFAULT_SCALE);
      expect(result.sampler).toBe(Constants.DEFAULT_SAMPLER);
      expect(result.noise_schedule).toBe(Constants.DEFAULT_NOISE_SCHEDULE);
    });

    it('should reject prompt exceeding max chars', async () => {
      const params = { prompt: 'a'.repeat(Constants.MAX_PROMPT_CHARS + 1) };
      const result = await Schemas.GenerateParamsSchema.safeParseAsync(params);
      expect(result.success).toBe(false);
    });
  });

  describe('width/height バリデーション', () => {
    it('should accept width/height as multiples of 64', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 512, height: 768 });
      expect(result.success).toBe(true);
    });

    it('should reject width not a multiple of 64', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 500 });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('multiple of 64'))).toBe(true);
      }
    });

    it('should reject height not a multiple of 64', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', height: 700 });
      expect(result.success).toBe(false);
    });

    it('should reject width below min dimension', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 32 });
      expect(result.success).toBe(false);
    });

    it('should reject non-integer width', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 512.5 });
      expect(result.success).toBe(false);
    });

    it('should reject total pixels exceeding MAX_PIXELS', async () => {
      // 1024 x 1024 = 1,048,576 which equals MAX_PIXELS, should pass
      const resultPass = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 1024, height: 1024 });
      expect(resultPass.success).toBe(true);
      // 1280 x 1024 = 1,310,720 which exceeds MAX_PIXELS, should fail
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', width: 1280, height: 1024 });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('exceeds limit'))).toBe(true);
      }
    });
  });

  describe('steps バリデーション', () => {
    it('should accept valid steps', async () => {
      const result1 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', steps: 1 });
      expect(result1.success).toBe(true);
      const result2 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', steps: 50 });
      expect(result2.success).toBe(true);
    });

    it('should reject steps below min', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', steps: 0 });
      expect(result.success).toBe(false);
    });

    it('should reject steps above max', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', steps: 51 });
      expect(result.success).toBe(false);
    });

    it('should reject non-integer steps', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', steps: 23.5 });
      expect(result.success).toBe(false);
    });
  });

  describe('scale バリデーション', () => {
    it('should accept valid scale', async () => {
      const result1 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', scale: 0 });
      expect(result1.success).toBe(true);
      const result2 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', scale: 10 });
      expect(result2.success).toBe(true);
      const result3 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', scale: 5.5 });
      expect(result3.success).toBe(true);
    });

    it('should reject scale above max', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', scale: 10.1 });
      expect(result.success).toBe(false);
    });
  });

  describe('seed バリデーション', () => {
    it('should accept valid seed', async () => {
      const result1 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', seed: 0 });
      expect(result1.success).toBe(true);
      const result2 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', seed: Constants.MAX_SEED });
      expect(result2.success).toBe(true);
    });

    it('should reject seed above max', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', seed: Constants.MAX_SEED + 1 });
      expect(result.success).toBe(false);
    });

    it('should reject negative seed', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', seed: -1 });
      expect(result.success).toBe(false);
    });

    it('should reject non-integer seed', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', seed: 123.4 });
      expect(result.success).toBe(false);
    });
  });

  describe('enum バリデーション', () => {
    it('should accept valid model', async () => {
      for (const model of Constants.VALID_MODELS) {
        const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', model });
        expect(result.success).toBe(true);
      }
    });

    it('should reject invalid model', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', model: 'invalid-model' });
      expect(result.success).toBe(false);
    });

    it('should accept valid sampler', async () => {
      for (const sampler of Constants.VALID_SAMPLERS) {
        const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', sampler });
        expect(result.success).toBe(true);
      }
    });

    it('should reject invalid sampler', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', sampler: 'invalid-sampler' });
      expect(result.success).toBe(false);
    });

    it('should accept valid noise_schedule', async () => {
      for (const schedule of Constants.VALID_NOISE_SCHEDULES) {
        const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', noise_schedule: schedule });
        expect(result.success).toBe(true);
      }
    });

    it('should reject invalid action', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', action: 'invalid' });
      expect(result.success).toBe(false);
    });
  });

  describe('img2img バリデーション', () => {
    it('should require source_image for img2img action', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', action: 'img2img' });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('source_image is required'))).toBe(true);
      }
    });

    it('should accept img2img with source_image', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        action: 'img2img',
        source_image: 'path/to/image.png',
      });
      expect(result.success).toBe(true);
    });

    it('should validate img2img_strength range', async () => {
      const result1 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', img2img_strength: -0.1 });
      expect(result1.success).toBe(false);
      const result2 = await Schemas.GenerateParamsSchema.safeParseAsync({ prompt: '1girl', img2img_strength: 1.1 });
      expect(result2.success).toBe(false);
    });
  });

  describe('vibes と character_reference の相互排他', () => {
    it('should reject vibes and character_reference used together', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibes: ['vibe1.naiv4vibe'],
        character_reference: { image: 'test.png' },
      });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('cannot be used together'))).toBe(true);
      }
    });

    it('should accept vibes without character_reference', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibes: ['vibe1.naiv4vibe'],
      });
      expect(result.success).toBe(true);
    });
  });

  describe('vibe_strengths / vibe_info_extracted 依存関係', () => {
    it('should reject vibe_strengths without vibes', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibe_strengths: [0.5],
      });
      expect(result.success).toBe(false);
    });

    it('should reject vibe_info_extracted without vibes', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibe_info_extracted: [0.7],
      });
      expect(result.success).toBe(false);
    });

    it('should reject mismatched vibes and vibe_strengths length', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibes: ['vibe1.naiv4vibe', 'vibe2.naiv4vibe'],
        vibe_strengths: [0.5],  // length mismatch
      });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('Mismatch'))).toBe(true);
      }
    });

    it('should reject mismatched vibes and vibe_info_extracted length', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibes: ['vibe1.naiv4vibe'],
        vibe_info_extracted: [0.5, 0.6],  // length mismatch
      });
      expect(result.success).toBe(false);
    });

    it('should accept matching vibes and vibe_strengths length', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        vibes: ['vibe1.naiv4vibe', 'vibe2.naiv4vibe'],
        vibe_strengths: [0.5, 0.6],
      });
      expect(result.success).toBe(true);
    });
  });

  describe('save_path / save_dir 相互排他', () => {
    it('should reject save_path and save_dir used together', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        save_path: '/path/to/file.png',
        save_dir: '/path/to/dir/',
      });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('cannot be specified together'))).toBe(true);
      }
    });

    it('should accept save_path alone', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        save_path: '/path/to/file.png',
      });
      expect(result.success).toBe(true);
    });

    it('should accept save_dir alone', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '1girl',
        save_dir: '/path/to/dir/',
      });
      expect(result.success).toBe(true);
    });
  });

  describe('characters バリデーション', () => {
    it('should accept valid characters array', async () => {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: '2girls',
        characters: [
          { prompt: 'girl A', center_x: 0.3, center_y: 0.5 },
          { prompt: 'girl B', center_x: 0.7, center_y: 0.5 },
        ],
      });
      expect(result.success).toBe(true);
    });

    it('should reject characters exceeding max count', async () => {
      const tooManyChars = Array(Constants.MAX_CHARACTERS + 1).fill({ prompt: 'test' });
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: 'many girls',
        characters: tooManyChars,
      });
      expect(result.success).toBe(false);
    });
  });

  describe('トークン数バリデーション (MAX_TOKENS = 512)', () => {
    it('should accept short prompts under 512 tokens', async () => {
      const shortPrompt = 'a beautiful landscape with mountains and rivers';
      const result = await Schemas.GenerateParamsSchema.parseAsync({
        prompt: shortPrompt,
      });
      expect(result).toBeDefined();
      expect(result.prompt).toBe(shortPrompt);
    });

    it('should reject prompts exceeding 512 tokens', async () => {
      // Create a prompt that definitely exceeds 512 tokens
      const longPrompt = Array(600).fill('masterpiece beautiful detailed anime girl').join(', ');
      
      try {
        await Schemas.GenerateParamsSchema.parseAsync({
          prompt: longPrompt,
        });
        // If we get here, the test should fail
        expect.fail('Expected validation to reject long prompt, but it passed');
      } catch (error) {
        expect(error).toBeInstanceOf(z.ZodError);
        if (error instanceof z.ZodError) {
          const tokenError = error.issues.find(i => i.message.includes('token count'));
          expect(tokenError).toBeDefined();
          expect(tokenError?.path).toEqual(['prompt']);
        }
      }
    });

    it('should handle empty prompts without token validation errors', async () => {
      const result = await Schemas.GenerateParamsSchema.parseAsync({
        prompt: '',
      });
      expect(result).toBeDefined();
      expect(result.prompt).toBe('');
    });

    it('should work with parseAsync (async validation)', async () => {
      // Verify that async validation is properly triggered
      const validPrompt = 'test prompt for async validation';
      const result = await Schemas.GenerateParamsSchema.parseAsync({
        prompt: validPrompt,
      });
      expect(result.prompt).toBe(validPrompt);
    });

    it('should fail with synchronous parse when async refinement is used', () => {
      // This test verifies that safeParse will fail since we have async validation
      const params = { prompt: 'test' };
      
      // safeParse should detect async refinement and fail
      expect(() => {
        Schemas.GenerateParamsSchema.parse(params);
      }).toThrow(/Async refinement/);
    });
  });
});

// =============================================================================
// EncodeVibeParamsSchema Tests
// =============================================================================
describe('EncodeVibeParamsSchema', () => {
  it('should validate minimal params (image only)', () => {
    const params = { image: 'test.png' };
    const result = Schemas.EncodeVibeParamsSchema.safeParse(params);
    expect(result.success).toBe(true);
  });

  it('should apply defaults correctly', () => {
    const params = { image: 'test.png' };
    const result = Schemas.EncodeVibeParamsSchema.parse(params);
    expect(result.model).toBe(Constants.DEFAULT_MODEL);
    expect(result.information_extracted).toBe(0.7);
    expect(result.strength).toBe(0.7);
  });

  it('should accept Buffer as image', () => {
    const result = Schemas.EncodeVibeParamsSchema.safeParse({ image: Buffer.from('test') });
    expect(result.success).toBe(true);
  });

  describe('information_extracted バリデーション', () => {
    it('should accept valid range', () => {
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', information_extracted: 0 }).success).toBe(true);
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', information_extracted: 1 }).success).toBe(true);
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', information_extracted: 0.5 }).success).toBe(true);
    });

    it('should reject out of range', () => {
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', information_extracted: -0.1 }).success).toBe(false);
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', information_extracted: 1.1 }).success).toBe(false);
    });
  });

  describe('strength バリデーション', () => {
    it('should accept valid range', () => {
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', strength: 0 }).success).toBe(true);
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', strength: 1 }).success).toBe(true);
    });

    it('should reject out of range', () => {
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', strength: -0.1 }).success).toBe(false);
      expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', strength: 1.1 }).success).toBe(false);
    });
  });

  describe('save_path / save_dir 相互排他', () => {
    it('should reject save_path and save_dir used together', () => {
      const result = Schemas.EncodeVibeParamsSchema.safeParse({
        image: 'test.png',
        save_path: '/path/to/file.naiv4vibe',
        save_dir: '/path/to/dir/',
      });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('cannot be specified together'))).toBe(true);
      }
    });
  });

  describe('save_filename 依存関係', () => {
    it('should reject save_filename without save_dir', () => {
      const result = Schemas.EncodeVibeParamsSchema.safeParse({
        image: 'test.png',
        save_filename: 'my_vibe',
      });
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues.some(i => i.message.includes('save_filename requires save_dir'))).toBe(true);
      }
    });

    it('should accept save_filename with save_dir', () => {
      const result = Schemas.EncodeVibeParamsSchema.safeParse({
        image: 'test.png',
        save_dir: './vibes/',
        save_filename: 'my_custom_vibe',
      });
      expect(result.success).toBe(true);
    });

    it('should accept save_dir without save_filename (auto-naming)', () => {
      const result = Schemas.EncodeVibeParamsSchema.safeParse({
        image: 'test.png',
        save_dir: './vibes/',
      });
      expect(result.success).toBe(true);
    });
  });

  describe('model バリデーション', () => {
    it('should accept valid models', () => {
      Constants.VALID_MODELS.forEach(model => {
        expect(Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', model }).success).toBe(true);
      });
    });

    it('should reject invalid model', () => {
      const result = Schemas.EncodeVibeParamsSchema.safeParse({ image: 'test.png', model: 'invalid-model' });
      expect(result.success).toBe(false);
    });
  });
});

// =============================================================================
// Helper Function Tests
// =============================================================================
describe('Helper Functions', () => {
  describe('characterToCaptionDict', () => {
    it('should convert CharacterConfig to caption dict', () => {
      const config: Schemas.CharacterConfig = {
        prompt: '1girl, red hair',
        center_x: 0.3,
        center_y: 0.7,
        negative_prompt: '',
      };
      const result = Schemas.characterToCaptionDict(config);
      expect(result.char_caption).toBe('1girl, red hair');
      expect(result.centers).toEqual([{ x: 0.3, y: 0.7 }]);
    });
  });

  describe('characterToNegativeCaptionDict', () => {
    it('should convert CharacterConfig to negative caption dict', () => {
      const config: Schemas.CharacterConfig = {
        prompt: '1girl',
        center_x: 0.5,
        center_y: 0.5,
        negative_prompt: 'lowres, bad anatomy',
      };
      const result = Schemas.characterToNegativeCaptionDict(config);
      expect(result.char_caption).toBe('lowres, bad anatomy');
      expect(result.centers).toEqual([{ x: 0.5, y: 0.5 }]);
    });
  });
});
