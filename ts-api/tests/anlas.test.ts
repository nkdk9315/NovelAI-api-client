/**
 * NovelAI Anlas Cost Calculation Tests
 * Anlasコスト計算ロジックのユニットテスト
 *
 * NovelAI公式フロントエンドのコスト計算ロジックに基づく検証
 */

import { describe, it, expect } from 'vitest';
import {
  calcV4BaseCost,
  getSmeaMultiplier,
  isOpusFreeGeneration,
  calcVibeBatchCost,
  calcCharRefCost,
  calcInpaintSizeCorrection,
  expandToMinPixels,
  clampToMaxPixels,
  calculateGenerationCost,
  calculateAugmentCost,
  calculateUpscaleCost,
} from '../src/anlas';

// =============================================================================
// Category A: calcV4BaseCost(width, height, steps)
// Formula: ceil(2.951823174884865e-6 * W*H + 5.753298233447344e-7 * W*H * steps)
// =============================================================================
describe('calcV4BaseCost', () => {
  it('A-1: 832x1216, 23 steps → 17', () => {
    expect(calcV4BaseCost(832, 1216, 23)).toBe(17);
  });

  it('A-2: 1024x1024, 28 steps → 20', () => {
    expect(calcV4BaseCost(1024, 1024, 28)).toBe(20);
  });

  it('A-3: 2048x1536, 50 steps → 100', () => {
    expect(calcV4BaseCost(2048, 1536, 50)).toBe(100);
  });

  it('A-4: 64x64, 1 step → 1 (smallest possible)', () => {
    // ceil(2.951823174884865e-6*4096 + 5.753298233447344e-7*4096*1) = ceil(0.01444) = 1
    expect(calcV4BaseCost(64, 64, 1)).toBe(1);
  });

  it('A-5: 832x1216, 1 step → 4', () => {
    // ceil(2.951823174884865e-6*1011712 + 5.753298233447344e-7*1011712*1) = ceil(3.5689) = 4
    expect(calcV4BaseCost(832, 1216, 1)).toBe(4);
  });

  it('A-6: 832x1216, 50 steps → 33', () => {
    // ceil(2.951823174884865e-6*1011712 + 5.753298233447344e-7*1011712*50) = ceil(32.089) = 33
    expect(calcV4BaseCost(832, 1216, 50)).toBe(33);
  });

  it('A-7: 1024x1024, 1 step → 4', () => {
    // ceil(2.951823174884865e-6*1048576 + 5.753298233447344e-7*1048576*1) = ceil(3.6978) = 4
    expect(calcV4BaseCost(1024, 1024, 1)).toBe(4);
  });
});

// =============================================================================
// Category B: getSmeaMultiplier(mode)
// =============================================================================
describe('getSmeaMultiplier', () => {
  it('B-1: off → 1.0', () => {
    expect(getSmeaMultiplier('off')).toBe(1.0);
  });

  it('B-2: smea → 1.2', () => {
    expect(getSmeaMultiplier('smea')).toBe(1.2);
  });

  it('B-3: smea_dyn → 1.4', () => {
    expect(getSmeaMultiplier('smea_dyn')).toBe(1.4);
  });
});

// =============================================================================
// Category C: isOpusFreeGeneration(width, height, steps, charRefCount, tier)
// =============================================================================
describe('isOpusFreeGeneration', () => {
  it('C-1: 832x1216, 23 steps, 0 charRef, tier 3 → true (all conditions met)', () => {
    expect(isOpusFreeGeneration(832, 1216, 23, 0, 3)).toBe(true);
  });

  it('C-2: 1024x1024, 28 steps, 0 charRef, tier 3 → true (exact pixel boundary)', () => {
    expect(isOpusFreeGeneration(1024, 1024, 28, 0, 3)).toBe(true);
  });

  it('C-3: 1088x1024, 28 steps, 0 charRef, tier 3 → false (pixels exceed 1048576)', () => {
    // 1088*1024 = 1114112 > 1048576
    expect(isOpusFreeGeneration(1088, 1024, 28, 0, 3)).toBe(false);
  });

  it('C-4: 1024x1024, 29 steps, 0 charRef, tier 3 → false (steps exceed)', () => {
    expect(isOpusFreeGeneration(1024, 1024, 29, 0, 3)).toBe(false);
  });

  it('C-5: 1024x1024, 28 steps, 0 charRef, tier 2 → false (tier too low)', () => {
    expect(isOpusFreeGeneration(1024, 1024, 28, 0, 2)).toBe(false);
  });

  it('C-6: 1024x1024, 28 steps, 1 charRef, tier 3 → false (has charRef)', () => {
    expect(isOpusFreeGeneration(1024, 1024, 28, 1, 3)).toBe(false);
  });

  it('C-7: 1024x1024, 28 steps, 0 charRef, tier 0 → false (free tier)', () => {
    expect(isOpusFreeGeneration(1024, 1024, 28, 0, 0)).toBe(false);
  });

  it('C-8: 1088x1024, 29 steps, 1 charRef, tier 2 → false (multiple conditions fail)', () => {
    expect(isOpusFreeGeneration(1088, 1024, 29, 1, 2)).toBe(false);
  });
});

// =============================================================================
// Category D: Per-image cost with strength/SMEA (via calculateGenerationCost adjustedCost)
// All using tier=0 to avoid Opus free, nSamples=1
// =============================================================================
describe('Per-image cost with strength/SMEA', () => {
  it('D-1: base case → adjustedCost=17', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 0 });
    expect(result.adjustedCost).toBe(17);
  });

  it('D-2: smea → adjustedCost=21 (ceil(17*1.2))', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, smea: 'smea', tier: 0 });
    expect(result.adjustedCost).toBe(21);
  });

  it('D-3: smea_dyn → adjustedCost=24 (ceil(17*1.4))', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, smea: 'smea_dyn', tier: 0 });
    expect(result.adjustedCost).toBe(24);
  });

  it('D-4: img2img strength=0.62 → adjustedCost=11 (max(ceil(17*0.62),2))', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 0.62, tier: 0,
    });
    expect(result.adjustedCost).toBe(11);
  });

  it('D-5: img2img strength=0.01 → adjustedCost=2 (MIN_COST)', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 0.01, tier: 0,
    });
    expect(result.adjustedCost).toBe(2);
  });

  it('D-6: img2img strength=1.0 → adjustedCost=17', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 1.0, tier: 0,
    });
    expect(result.adjustedCost).toBe(17);
  });

  it('D-7: smea + img2img strength=0.62 → adjustedCost=13 (max(ceil(20.4*0.62),2))', () => {
    // baseCost=17, smea=1.2, perImageCost=17*1.2=20.4 (not ceiled)
    // adjustedCost = max(ceil(20.4*0.62), 2) = max(ceil(12.648), 2) = 13
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, smea: 'smea', mode: 'img2img', strength: 0.62, tier: 0,
    });
    expect(result.adjustedCost).toBe(13);
  });

  it('D-8: 2048x1536, 50 steps, smea_dyn → adjustedCost=140, error=false', () => {
    const result = calculateGenerationCost({
      width: 2048, height: 1536, steps: 50, smea: 'smea_dyn', tier: 0,
    });
    expect(result.adjustedCost).toBe(140);
    expect(result.error).toBe(false);
  });

  it('D-9: max case (2048x1536, 50 steps, smea_dyn) → adjustedCost=140, error=false', () => {
    // baseCost=100, smea_dyn → 100*1.4=140 exactly at MAX_COST, not error
    const result = calculateGenerationCost({
      width: 2048, height: 1536, steps: 50, smea: 'smea_dyn', tier: 0,
    });
    expect(result.adjustedCost).toBe(140);
    expect(result.error).toBe(false);
  });

  it('D-10: img2img strength=0.12 → adjustedCost=3 (max(ceil(17*0.12),2))', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 0.12, tier: 0,
    });
    expect(result.adjustedCost).toBe(3);
  });
});

// =============================================================================
// Category E: Billable images and Opus discount (via calculateGenerationCost)
// =============================================================================
describe('Billable images and Opus discount', () => {
  it('E-1: Opus tier, 1 sample → isOpusFree=true, billableImages=0, generationCost=0', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1 });
    expect(result.isOpusFree).toBe(true);
    expect(result.billableImages).toBe(0);
    expect(result.generationCost).toBe(0);
  });

  it('E-2: Opus tier, 2 samples → isOpusFree=true, billableImages=1, generationCost=17', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 3, nSamples: 2 });
    expect(result.isOpusFree).toBe(true);
    expect(result.billableImages).toBe(1);
    expect(result.generationCost).toBe(17);
  });

  it('E-3: Opus tier, 4 samples → isOpusFree=true, billableImages=3, generationCost=51', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 3, nSamples: 4 });
    expect(result.isOpusFree).toBe(true);
    expect(result.billableImages).toBe(3);
    expect(result.generationCost).toBe(51);
  });

  it('E-4: non-Opus, 1 sample → isOpusFree=false, billableImages=1, generationCost=17', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 0, nSamples: 1 });
    expect(result.isOpusFree).toBe(false);
    expect(result.billableImages).toBe(1);
    expect(result.generationCost).toBe(17);
  });

  it('E-5: non-Opus, 4 samples → isOpusFree=false, billableImages=4, generationCost=68', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 0, nSamples: 4 });
    expect(result.isOpusFree).toBe(false);
    expect(result.billableImages).toBe(4);
    expect(result.generationCost).toBe(68);
  });
});

// =============================================================================
// Category F: Vibe costs
// =============================================================================
describe('Vibe costs', () => {
  describe('calcVibeBatchCost', () => {
    it('F-1: 0 vibes → 0', () => {
      expect(calcVibeBatchCost(0)).toBe(0);
    });

    it('F-2: 1 vibe → 0', () => {
      expect(calcVibeBatchCost(1)).toBe(0);
    });

    it('F-3: 4 vibes → 0 (at threshold)', () => {
      expect(calcVibeBatchCost(4)).toBe(0);
    });

    it('F-4: 5 vibes → 2', () => {
      expect(calcVibeBatchCost(5)).toBe(2);
    });

    it('F-5: 6 vibes → 4', () => {
      expect(calcVibeBatchCost(6)).toBe(4);
    });

    it('F-6: 10 vibes → 12', () => {
      expect(calcVibeBatchCost(10)).toBe(12);
    });
  });

  describe('Vibe costs in calculateGenerationCost', () => {
    it('F-7: 3 vibes (all encoded), Opus → vibeEncodeCost=0, vibeBatchCost=0', () => {
      const result = calculateGenerationCost({
        width: 832, height: 1216, steps: 23,
        vibeCount: 3, vibeUnencodedCount: 0, tier: 3,
      });
      expect(result.vibeEncodeCost).toBe(0);
      expect(result.vibeBatchCost).toBe(0);
    });

    it('F-8: 6 vibes (2 unencoded), Opus → vibeEncodeCost=4, vibeBatchCost=4, totalCost=8', () => {
      const result = calculateGenerationCost({
        width: 832, height: 1216, steps: 23,
        vibeCount: 6, vibeUnencodedCount: 2, tier: 3,
      });
      expect(result.vibeEncodeCost).toBe(4);
      expect(result.vibeBatchCost).toBe(4);
      expect(result.totalCost).toBe(8);
    });

    it('F-9: 5 vibes + charRef → vibeBatchCost=0, vibeEncodeCost=0 (charRef disables vibe)', () => {
      const result = calculateGenerationCost({
        width: 832, height: 1216, steps: 23,
        vibeCount: 5, charRefCount: 1, tier: 0,
      });
      expect(result.vibeBatchCost).toBe(0);
      expect(result.vibeEncodeCost).toBe(0);
    });

    it('F-10: 5 vibes + inpaint → vibeBatchCost=0, vibeEncodeCost=0 (inpaint disables vibe)', () => {
      const result = calculateGenerationCost({
        width: 832, height: 1216, steps: 23,
        vibeCount: 5, mode: 'inpaint', tier: 0,
      });
      expect(result.vibeBatchCost).toBe(0);
      expect(result.vibeEncodeCost).toBe(0);
    });
  });
});

// =============================================================================
// Category G: Character reference cost
// =============================================================================
describe('calcCharRefCost', () => {
  it('G-1: 0 charRefs, 1 sample → 0', () => {
    expect(calcCharRefCost(0, 1)).toBe(0);
  });

  it('G-2: 1 charRef, 1 sample → 5', () => {
    expect(calcCharRefCost(1, 1)).toBe(5);
  });

  it('G-3: 2 charRefs, 1 sample → 10', () => {
    expect(calcCharRefCost(2, 1)).toBe(10);
  });

  it('G-4: 1 charRef, 4 samples → 20', () => {
    expect(calcCharRefCost(1, 4)).toBe(20);
  });

  it('G-5: 6 charRefs, 4 samples → 120', () => {
    expect(calcCharRefCost(6, 4)).toBe(120);
  });
});

// =============================================================================
// Category H: Inpaint size correction
// =============================================================================
describe('calcInpaintSizeCorrection', () => {
  it('H-1: 1024x1024 → corrected=false (maskPixels >= threshold)', () => {
    // maskPixels=1048576, threshold=0.8*1048576=838861
    const result = calcInpaintSizeCorrection(1024, 1024);
    expect(result.corrected).toBe(false);
    expect(result.width).toBe(1024);
    expect(result.height).toBe(1024);
  });

  it('H-2: 928x928 → corrected=false (861184 >= 838861)', () => {
    const result = calcInpaintSizeCorrection(928, 928);
    expect(result.corrected).toBe(false);
    expect(result.width).toBe(928);
    expect(result.height).toBe(928);
  });

  it('H-3: 512x512 → corrected=true, 1024x1024', () => {
    // scale = sqrt(1048576/262144) = 2.0
    // width = floor(floor(512*2.0)/64)*64 = 1024
    const result = calcInpaintSizeCorrection(512, 512);
    expect(result.corrected).toBe(true);
    expect(result.width).toBe(1024);
    expect(result.height).toBe(1024);
  });

  it('H-4: 256x256 → corrected=true, 1024x1024', () => {
    // scale = sqrt(1048576/65536) = 4.0
    const result = calcInpaintSizeCorrection(256, 256);
    expect(result.corrected).toBe(true);
    expect(result.width).toBe(1024);
    expect(result.height).toBe(1024);
  });

  it('H-5: 300x400 → corrected=true, 832x1152', () => {
    // scale = sqrt(1048576/120000) ≈ 2.9559
    // width = floor(floor(300*2.9559)/64)*64 = floor(886/64)*64 = 13*64 = 832
    // height = floor(floor(400*2.9559)/64)*64 = floor(1182/64)*64 = 18*64 = 1152
    const result = calcInpaintSizeCorrection(300, 400);
    expect(result.corrected).toBe(true);
    expect(result.width).toBe(832);
    expect(result.height).toBe(1152);
  });
});

// =============================================================================
// Category I: Full integration tests (calculateGenerationCost)
// =============================================================================
describe('calculateGenerationCost integration', () => {
  it('I-1: Opus free → totalCost=0', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1 });
    expect(result.totalCost).toBe(0);
  });

  it('I-2: non-Opus → totalCost=17', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 0, nSamples: 1 });
    expect(result.totalCost).toBe(17);
  });

  it('I-3: 1024x1024 Opus → totalCost=0', () => {
    const result = calculateGenerationCost({ width: 1024, height: 1024, steps: 28, tier: 3, nSamples: 1 });
    expect(result.totalCost).toBe(0);
  });

  it('I-4: 2048x1536, 50 steps → totalCost=100', () => {
    const result = calculateGenerationCost({ width: 2048, height: 1536, steps: 50, tier: 0, nSamples: 1 });
    expect(result.totalCost).toBe(100);
  });

  it('I-5: Opus, 2 samples → totalCost=17 (1 billable)', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, tier: 3, nSamples: 2 });
    expect(result.totalCost).toBe(17);
  });

  it('I-6: smea → totalCost=21', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, smea: 'smea', tier: 0, nSamples: 1 });
    expect(result.totalCost).toBe(21);
  });

  it('I-7: smea_dyn → totalCost=24', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23, smea: 'smea_dyn', tier: 0, nSamples: 1 });
    expect(result.totalCost).toBe(24);
  });

  it('I-8: img2img strength=0.62 → totalCost=11', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 0.62, tier: 0, nSamples: 1,
    });
    expect(result.totalCost).toBe(11);
  });

  it('I-9: 1024x1024, 28 steps, non-Opus → totalCost=20', () => {
    const result = calculateGenerationCost({ width: 1024, height: 1024, steps: 28, tier: 0, nSamples: 1 });
    expect(result.totalCost).toBe(20);
  });

  it('I-10: charRef disables Opus free → charRefCost=10, generationCost=17, totalCost=27', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1, charRefCount: 2,
    });
    expect(result.charRefCost).toBe(10);
    expect(result.generationCost).toBe(17);
    expect(result.totalCost).toBe(27);
  });

  it('I-11: Opus free + vibes → generationCost=0, vibeEncodeCost=4, vibeBatchCost=4, totalCost=8', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1,
      vibeCount: 6, vibeUnencodedCount: 2,
    });
    expect(result.generationCost).toBe(0);
    expect(result.vibeEncodeCost).toBe(4);
    expect(result.vibeBatchCost).toBe(4);
    expect(result.totalCost).toBe(8);
  });

  it('I-12: Opus 2 samples + vibes → generationCost=17, vibeEncodeCost=2, vibeBatchCost=2, totalCost=21', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 3, nSamples: 2,
      vibeCount: 5, vibeUnencodedCount: 1,
    });
    expect(result.generationCost).toBe(17);
    expect(result.vibeEncodeCost).toBe(2);
    expect(result.vibeBatchCost).toBe(2);
    expect(result.totalCost).toBe(21);
  });

  it('I-13: error=false for max configuration (2048x1536, 50 steps, smea_dyn)', () => {
    const result = calculateGenerationCost({
      width: 2048, height: 1536, steps: 50, smea: 'smea_dyn', tier: 0, nSamples: 1,
    });
    // baseCost=100, smea_dyn*1.4=140 exactly → at MAX_COST, not exceeding
    expect(result.error).toBe(false);
    expect(result.adjustedCost).toBe(140);
  });
});

// =============================================================================
// Category J: calculateAugmentCost
// =============================================================================
describe('calculateAugmentCost', () => {
  it('J-1: lineart 512x512 tier 3 → expand to 1024x1024, baseCost=20, finalCost=20, isOpusFree=true, effectiveCost=0', () => {
    const result = calculateAugmentCost({ tool: 'lineart', width: 512, height: 512, tier: 3 });
    expect(result.adjustedWidth).toBe(1024);
    expect(result.adjustedHeight).toBe(1024);
    expect(result.adjustedPixels).toBe(1048576);
    expect(result.baseCost).toBe(20);
    expect(result.finalCost).toBe(20);
    expect(result.isOpusFree).toBe(true);
    expect(result.effectiveCost).toBe(0);
  });

  it('J-2: bg-removal 512x512 tier 3 → baseCost=20, finalCost=65, isOpusFree=false, effectiveCost=65', () => {
    const result = calculateAugmentCost({ tool: 'bg-removal', width: 512, height: 512, tier: 3 });
    expect(result.baseCost).toBe(20);
    expect(result.finalCost).toBe(65);
    expect(result.isOpusFree).toBe(false);
    expect(result.effectiveCost).toBe(65);
  });

  it('J-3: lineart 1024x1024 tier 3 → no expansion, baseCost=20, isOpusFree=true, effectiveCost=0', () => {
    const result = calculateAugmentCost({ tool: 'lineart', width: 1024, height: 1024, tier: 3 });
    expect(result.adjustedWidth).toBe(1024);
    expect(result.adjustedHeight).toBe(1024);
    expect(result.baseCost).toBe(20);
    expect(result.finalCost).toBe(20);
    expect(result.isOpusFree).toBe(true);
    expect(result.effectiveCost).toBe(0);
  });

  it('J-4: colorize 2048x1536 tier 0 → baseCost=60, finalCost=60, isOpusFree=false, effectiveCost=60', () => {
    const result = calculateAugmentCost({ tool: 'colorize', width: 2048, height: 1536, tier: 0 });
    expect(result.originalPixels).toBe(3145728);
    expect(result.baseCost).toBe(60);
    expect(result.finalCost).toBe(60);
    expect(result.isOpusFree).toBe(false);
    expect(result.effectiveCost).toBe(60);
  });

  it('J-5: bg-removal 1024x1024 tier 0 → baseCost=20, finalCost=65, isOpusFree=false, effectiveCost=65', () => {
    const result = calculateAugmentCost({ tool: 'bg-removal', width: 1024, height: 1024, tier: 0 });
    expect(result.baseCost).toBe(20);
    expect(result.finalCost).toBe(65);
    expect(result.isOpusFree).toBe(false);
    expect(result.effectiveCost).toBe(65);
  });

  it('J-6: sketch 1200x900 tier 0 → baseCost=21, finalCost=21, effectiveCost=21', () => {
    const result = calculateAugmentCost({ tool: 'sketch', width: 1200, height: 900, tier: 0 });
    expect(result.originalPixels).toBe(1080000);
    expect(result.baseCost).toBe(21);
    expect(result.finalCost).toBe(21);
    expect(result.isOpusFree).toBe(false);
    expect(result.effectiveCost).toBe(21);
  });

  it('J-7: all 6 tool types with 1024x1024, tier 0 → baseCost=20, bg-removal=65, others=20', () => {
    const tools = ['colorize', 'declutter', 'emotion', 'sketch', 'lineart', 'bg-removal'] as const;
    for (const tool of tools) {
      const result = calculateAugmentCost({ tool, width: 1024, height: 1024, tier: 0 });
      expect(result.baseCost).toBe(20);
      if (tool === 'bg-removal') {
        expect(result.finalCost).toBe(65);
      } else {
        expect(result.finalCost).toBe(20);
      }
    }
  });

  it('J-8: emotion 100x100 tier 0 → expand with Math.floor (no grid snap)', () => {
    // scale=sqrt(1048576/10000)=10.24 exactly
    // w=floor(100*10.24)=floor(1024)=1024, h=1024
    // adjustedPixels=1024*1024=1048576
    const result = calculateAugmentCost({ tool: 'emotion', width: 100, height: 100, tier: 0 });
    expect(result.adjustedWidth).toBe(1024);
    expect(result.adjustedHeight).toBe(1024);
    expect(result.adjustedPixels).toBe(1024 * 1024);
    expect(result.baseCost).toBe(20);
  });
});

// =============================================================================
// Category K: calculateUpscaleCost
// =============================================================================
describe('calculateUpscaleCost', () => {
  it('K-1: 512x512 tier 0 → pixels=262144, cost=1, isOpusFree=false, error=false', () => {
    const result = calculateUpscaleCost({ width: 512, height: 512, tier: 0 });
    expect(result.pixels).toBe(262144);
    expect(result.cost).toBe(1);
    expect(result.isOpusFree).toBe(false);
    expect(result.error).toBe(false);
  });

  it('K-2: 640x640 tier 0 → pixels=409600, cost=2, isOpusFree=false, error=false', () => {
    const result = calculateUpscaleCost({ width: 640, height: 640, tier: 0 });
    expect(result.pixels).toBe(409600);
    expect(result.cost).toBe(2);
    expect(result.isOpusFree).toBe(false);
    expect(result.error).toBe(false);
  });

  it('K-3: 512x1024 tier 0 → pixels=524288, cost=3, error=false', () => {
    const result = calculateUpscaleCost({ width: 512, height: 1024, tier: 0 });
    expect(result.pixels).toBe(524288);
    expect(result.cost).toBe(3);
    expect(result.error).toBe(false);
  });

  it('K-4: 1024x768 tier 0 → pixels=786432, cost=5, error=false', () => {
    const result = calculateUpscaleCost({ width: 1024, height: 768, tier: 0 });
    expect(result.pixels).toBe(786432);
    expect(result.cost).toBe(5);
    expect(result.error).toBe(false);
  });

  it('K-5: 1024x1024 tier 0 → pixels=1048576, cost=7, error=false', () => {
    const result = calculateUpscaleCost({ width: 1024, height: 1024, tier: 0 });
    expect(result.pixels).toBe(1048576);
    expect(result.cost).toBe(7);
    expect(result.error).toBe(false);
  });

  it('K-6: 1025x1024 tier 0 → pixels > 1048576, error=true, errorCode=-3', () => {
    const result = calculateUpscaleCost({ width: 1025, height: 1024, tier: 0 });
    expect(result.pixels).toBe(1049600);
    expect(result.cost).toBeNull();
    expect(result.error).toBe(true);
    expect(result.errorCode).toBe(-3);
  });

  it('K-7: 512x512 tier 3 → isOpusFree=true, cost=0 (effective)', () => {
    const result = calculateUpscaleCost({ width: 512, height: 512, tier: 3 });
    expect(result.pixels).toBe(262144);
    expect(result.isOpusFree).toBe(true);
    expect(result.cost).toBe(0);
  });

  it('K-8: 640x640 tier 3 → isOpusFree=true, cost=0', () => {
    const result = calculateUpscaleCost({ width: 640, height: 640, tier: 3 });
    expect(result.pixels).toBe(409600);
    expect(result.isOpusFree).toBe(true);
    expect(result.cost).toBe(0);
  });

  it('K-9: 512x1024 tier 3 → isOpusFree=false, cost=3', () => {
    const result = calculateUpscaleCost({ width: 512, height: 1024, tier: 3 });
    expect(result.pixels).toBe(524288);
    expect(result.isOpusFree).toBe(false);
    expect(result.cost).toBe(3);
  });

  it('K-10: exact boundary 512x512 = 262144 → cost=1', () => {
    const result = calculateUpscaleCost({ width: 512, height: 512, tier: 0 });
    expect(result.pixels).toBe(262144);
    expect(result.cost).toBe(1);
  });
});

// =============================================================================
// Category L: Size helpers
// =============================================================================
describe('Size helpers', () => {
  describe('expandToMinPixels', () => {
    it('L-1: 1024x1024, minPixels=1048576 → no change', () => {
      const result = expandToMinPixels(1024, 1024, 1048576);
      expect(result.width).toBe(1024);
      expect(result.height).toBe(1024);
    });

    it('L-2: 512x512, minPixels=1048576 → scale=2, 1024x1024', () => {
      const result = expandToMinPixels(512, 512, 1048576);
      expect(result.width).toBe(1024);
      expect(result.height).toBe(1024);
    });

    it('L-3: 100x100, minPixels=1048576 → 1024x1024 (Math.floor, no grid snap)', () => {
      // scale=sqrt(1048576/10000)=10.24 exactly
      // width=floor(100*10.24)=floor(1024)=1024
      const result = expandToMinPixels(100, 100, 1048576);
      expect(result.width).toBe(1024);
      expect(result.height).toBe(1024);
    });
  });

  describe('clampToMaxPixels', () => {
    it('L-4: 1024x1024, maxPixels=3145728 → no change', () => {
      const result = clampToMaxPixels(1024, 1024, 3145728);
      expect(result.width).toBe(1024);
      expect(result.height).toBe(1024);
    });

    it('L-5: 2048x2048, maxPixels=3145728 → scale down', () => {
      // 4194304 > 3145728, scale=sqrt(3145728/4194304)≈0.86602
      // width=floor(2048*0.86602)=floor(1773.96)=1773
      const result = clampToMaxPixels(2048, 2048, 3145728);
      expect(result.width).toBe(1773);
      expect(result.height).toBe(1773);
    });

    it('L-6: 3000x2000, maxPixels=3145728 → scale down', () => {
      // 6000000 > 3145728, scale=sqrt(3145728/6000000)≈0.72408
      // width=floor(3000*0.72408)=floor(2172.23)=2172
      // height=floor(2000*0.72408)=floor(1448.15)=1448
      const result = clampToMaxPixels(3000, 2000, 3145728);
      expect(result.width).toBe(2172);
      expect(result.height).toBe(1448);
    });
  });
});

// =============================================================================
// Category M: Edge cases
// =============================================================================
describe('Edge cases', () => {
  it('M-1: img2img strength=0 → adjustedCost=2 (MIN_COST)', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, mode: 'img2img', strength: 0, tier: 0,
    });
    expect(result.adjustedCost).toBe(2);
  });

  it('M-2: nSamples=0 → totalCost=0', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 0, nSamples: 0,
    });
    expect(result.totalCost).toBe(0);
  });

  it('M-3: default params only → totalCost=17 (tier defaults to 0)', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23 });
    expect(result.totalCost).toBe(17);
  });

  it('M-4: result object has all expected keys', () => {
    const result = calculateGenerationCost({ width: 832, height: 1216, steps: 23 });
    expect(result).toHaveProperty('baseCost');
    expect(result).toHaveProperty('adjustedCost');
    expect(result).toHaveProperty('isOpusFree');
    expect(result).toHaveProperty('billableImages');
    expect(result).toHaveProperty('generationCost');
    expect(result).toHaveProperty('vibeEncodeCost');
    expect(result).toHaveProperty('vibeBatchCost');
    expect(result).toHaveProperty('charRefCost');
    expect(result).toHaveProperty('totalCost');
    expect(result).toHaveProperty('error');
  });

  it('M-5: Opus free + vibes → generationCost=0, vibeEncodeCost=2, vibeBatchCost=2, totalCost=4', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1,
      vibeCount: 5, vibeUnencodedCount: 1,
    });
    expect(result.generationCost).toBe(0);
    expect(result.vibeEncodeCost).toBe(2);
    expect(result.vibeBatchCost).toBe(2);
    expect(result.totalCost).toBe(4);
  });

  it('M-6: charRef → isOpusFree=false AND vibeBatchCost=0', () => {
    const result = calculateGenerationCost({
      width: 832, height: 1216, steps: 23, tier: 3, nSamples: 1,
      charRefCount: 1, vibeCount: 5,
    });
    expect(result.isOpusFree).toBe(false);
    expect(result.vibeBatchCost).toBe(0);
    expect(result.vibeEncodeCost).toBe(0);
    expect(result.charRefCost).toBe(5);
  });
});
