/**
 * NovelAI Client Constants Tests
 * 定数のテスト
 */

import { describe, it, expect } from 'vitest';
import * as Constants from '../src/constants';

// =============================================================================
// URL Constants Tests
// =============================================================================
describe('URL Constants', () => {
  it('should have valid API URLs', () => {
    expect(Constants.API_URL).toBe('https://image.novelai.net/ai/generate-image');
    expect(Constants.STREAM_URL).toBe('https://image.novelai.net/ai/generate-image-stream');
    expect(Constants.ENCODE_URL).toBe('https://image.novelai.net/ai/encode-vibe');
    expect(Constants.SUBSCRIPTION_URL).toBe('https://api.novelai.net/user/subscription');
  });

  it('should have valid Augment and Upscale URLs', () => {
    expect(Constants.AUGMENT_URL).toBe('https://image.novelai.net/ai/augment-image');
    expect(Constants.UPSCALE_URL).toBe('https://api.novelai.net/ai/upscale');
  });
});

// =============================================================================
// Default Values Tests
// =============================================================================
describe('Default Values', () => {
  it('should have valid default model', () => {
    expect(Constants.DEFAULT_MODEL).toBe('nai-diffusion-4-5-full');
    expect(Constants.VALID_MODELS).toContain(Constants.DEFAULT_MODEL);
  });

  it('should have valid default dimensions', () => {
    expect(Constants.DEFAULT_WIDTH).toBe(832);
    expect(Constants.DEFAULT_HEIGHT).toBe(1216);
    expect(Constants.DEFAULT_WIDTH % 64).toBe(0);
    expect(Constants.DEFAULT_HEIGHT % 64).toBe(0);
    expect(Constants.DEFAULT_WIDTH * Constants.DEFAULT_HEIGHT).toBeLessThanOrEqual(Constants.MAX_PIXELS);
  });

  it('should have valid default generation params', () => {
    expect(Constants.DEFAULT_STEPS).toBe(23);
    expect(Constants.DEFAULT_SCALE).toBe(5.0);
    expect(Constants.DEFAULT_SAMPLER).toBe('k_euler_ancestral');
    expect(Constants.DEFAULT_NOISE_SCHEDULE).toBe('karras');
  });

  it('should have valid defry defaults', () => {
    expect(Constants.DEFAULT_DEFRY).toBe(3);
    expect(Constants.DEFAULT_DEFRY).toBeGreaterThanOrEqual(Constants.MIN_DEFRY);
    expect(Constants.DEFAULT_DEFRY).toBeLessThanOrEqual(Constants.MAX_DEFRY);
  });

  it('should have valid upscale defaults', () => {
    expect(Constants.DEFAULT_UPSCALE_SCALE).toBe(4);
    expect(Constants.VALID_UPSCALE_SCALES).toContain(Constants.DEFAULT_UPSCALE_SCALE);
  });
});

// =============================================================================
// Validation Constants Tests
// =============================================================================
describe('Validation Constants', () => {
  it('should have valid samplers array', () => {
    expect(Constants.VALID_SAMPLERS).toContain('k_euler');
    expect(Constants.VALID_SAMPLERS).toContain('k_euler_ancestral');
    expect(Constants.VALID_SAMPLERS).toContain('k_dpmpp_2s_ancestral');
    expect(Constants.VALID_SAMPLERS).toContain('k_dpmpp_2m_sde');
    expect(Constants.VALID_SAMPLERS).toContain('k_dpmpp_2m');
    expect(Constants.VALID_SAMPLERS).toContain('k_dpmpp_sde');
    expect(Constants.VALID_SAMPLERS.length).toBe(6);
  });

  it('should have valid models array', () => {
    expect(Constants.VALID_MODELS).toContain('nai-diffusion-4-curated-preview');
    expect(Constants.VALID_MODELS).toContain('nai-diffusion-4-full');
    expect(Constants.VALID_MODELS).toContain('nai-diffusion-4-5-curated');
    expect(Constants.VALID_MODELS).toContain('nai-diffusion-4-5-full');
    expect(Constants.VALID_MODELS.length).toBe(4);
  });

  it('should have valid noise schedules array', () => {
    expect(Constants.VALID_NOISE_SCHEDULES).toContain('native');
    expect(Constants.VALID_NOISE_SCHEDULES).toContain('karras');
    expect(Constants.VALID_NOISE_SCHEDULES).toContain('exponential');
    expect(Constants.VALID_NOISE_SCHEDULES).toContain('polyexponential');
    expect(Constants.VALID_NOISE_SCHEDULES.length).toBe(4);
  });
});

// =============================================================================
// Augment Tool Constants Tests
// =============================================================================
describe('Augment Tool Constants', () => {
  it('should have all augment req_types', () => {
    expect(Constants.AUGMENT_REQ_TYPES).toContain('colorize');
    expect(Constants.AUGMENT_REQ_TYPES).toContain('declutter');
    expect(Constants.AUGMENT_REQ_TYPES).toContain('emotion');
    expect(Constants.AUGMENT_REQ_TYPES).toContain('sketch');
    expect(Constants.AUGMENT_REQ_TYPES).toContain('lineart');
    expect(Constants.AUGMENT_REQ_TYPES).toContain('bg-removal');
    expect(Constants.AUGMENT_REQ_TYPES.length).toBe(6);
  });

  it('should have all emotion keywords', () => {
    const expectedKeywords = [
      'neutral', 'happy', 'sad', 'angry', 'scared', 'surprised',
      'tired', 'excited', 'nervous', 'thinking', 'confused', 'shy',
      'disgusted', 'smug', 'bored', 'laughing', 'irritated', 'aroused',
      'embarrassed', 'love', 'worried', 'determined', 'hurt', 'playful',
    ];
    
    expectedKeywords.forEach(keyword => {
      expect(Constants.EMOTION_KEYWORDS).toContain(keyword);
    });
    expect(Constants.EMOTION_KEYWORDS.length).toBe(24);
  });

  it('should have valid defry range', () => {
    expect(Constants.MIN_DEFRY).toBe(0);
    expect(Constants.MAX_DEFRY).toBe(5);
    expect(Constants.MIN_DEFRY).toBeLessThan(Constants.MAX_DEFRY);
  });

  it('should have valid upscale scales', () => {
    expect(Constants.VALID_UPSCALE_SCALES).toContain(2);
    expect(Constants.VALID_UPSCALE_SCALES).toContain(4);
    expect(Constants.VALID_UPSCALE_SCALES.length).toBe(2);
  });
});

// =============================================================================
// Limit Constants Tests
// =============================================================================
describe('Limit Constants', () => {
  it('should have valid pixel limits', () => {
    expect(Constants.MAX_PIXELS).toBe(1_048_576);  // 1024 * 1024
    expect(Constants.MIN_DIMENSION).toBe(64);
    expect(Constants.MAX_DIMENSION).toBe(1024);
  });

  it('should have valid step limits', () => {
    expect(Constants.MIN_STEPS).toBe(1);
    expect(Constants.MAX_STEPS).toBe(50);
  });

  it('should have valid scale limits', () => {
    expect(Constants.MIN_SCALE).toBe(0.0);
    expect(Constants.MAX_SCALE).toBe(10.0);
  });

  it('should have valid seed limit', () => {
    expect(Constants.MAX_SEED).toBe(4294967295);  // 2^32 - 1
  });

  it('should have valid token limits', () => {
    expect(Constants.MAX_PROMPT_CHARS).toBe(2000);
    expect(Constants.MAX_TOKENS).toBe(512);
  });

  it('should have valid character and vibe limits', () => {
    expect(Constants.MAX_CHARACTERS).toBe(6);
    expect(Constants.MAX_VIBES).toBe(10);
  });
});

// =============================================================================
// Model Key Map Tests
// =============================================================================
describe('Model Key Map', () => {
  it('should have mappings for all valid models', () => {
    Constants.VALID_MODELS.forEach(model => {
      expect(Constants.MODEL_KEY_MAP[model]).toBeDefined();
    });
  });

  it('should have correct model key mappings', () => {
    expect(Constants.MODEL_KEY_MAP['nai-diffusion-4-curated-preview']).toBe('v4curated');
    expect(Constants.MODEL_KEY_MAP['nai-diffusion-4-full']).toBe('v4full');
    expect(Constants.MODEL_KEY_MAP['nai-diffusion-4-5-curated']).toBe('v4-5curated');
    expect(Constants.MODEL_KEY_MAP['nai-diffusion-4-5-full']).toBe('v4-5full');
  });
});
