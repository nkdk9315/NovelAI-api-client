/**
 * NovelAI Anlas コスト計算モジュール
 * 純粋な計算ロジックのみ。API呼び出し・非同期処理・外部依存なし。
 */

import {
  OPUS_FREE_PIXELS,
  OPUS_FREE_MAX_STEPS,
  OPUS_MIN_TIER,
  MAX_COST_PER_IMAGE,
  MIN_COST_PER_IMAGE,
  GRID_SIZE,
  VIBE_BATCH_PRICE,
  VIBE_FREE_THRESHOLD,
  VIBE_ENCODE_PRICE,
  CHAR_REF_PRICE,
  INPAINT_THRESHOLD_RATIO,
  V4_COST_COEFF_LINEAR,
  V4_COST_COEFF_STEP,
  AUGMENT_FIXED_STEPS,
  AUGMENT_MIN_PIXELS,
  BG_REMOVAL_MULTIPLIER,
  BG_REMOVAL_ADDEND,
  UPSCALE_COST_TABLE,
  UPSCALE_OPUS_FREE_PIXELS,
  MAX_PIXELS,
} from './constants';


// =============================================================================
// 型定義
// =============================================================================

/** サブスクリプションティア（0=Free, 1=Tablet, 2=Scroll, 3=Opus） */
export type SubscriptionTier = 0 | 1 | 2 | 3;

/** SMEAモード */
export type SmeaMode = 'off' | 'smea' | 'smea_dyn';

/** 生成モード */
export type GenerationMode = 'txt2img' | 'img2img' | 'inpaint';

/** Augmentツールタイプ */
export type AugmentToolType = 'colorize' | 'declutter' | 'emotion' | 'sketch' | 'lineart' | 'bg-removal';

/** 画像生成コスト計算パラメータ */
export type GenerationCostParams = {
  width: number;
  height: number;
  steps: number;
  smea?: SmeaMode;              // デフォルト: 'off'
  mode?: GenerationMode;        // デフォルト: 'txt2img'
  strength?: number;            // デフォルト: 1.0
  nSamples?: number;            // デフォルト: 1
  tier?: SubscriptionTier;      // デフォルト: 0
  charRefCount?: number;        // デフォルト: 0
  vibeCount?: number;           // デフォルト: 0
  vibeUnencodedCount?: number;  // デフォルト: 0
  maskWidth?: number;           // Inpaintマスク寸法
  maskHeight?: number;
};

/** 画像生成コスト計算結果（内訳付き） */
export type GenerationCostResult = {
  baseCost: number;
  smeaMultiplier: number;
  perImageCost: number;
  strengthMultiplier: number;
  adjustedCost: number;
  isOpusFree: boolean;
  billableImages: number;
  generationCost: number;
  charRefCost: number;
  vibeEncodeCost: number;
  vibeBatchCost: number;
  totalCost: number;
  error: boolean;
  errorCode: number | null;
};

/** Augmentコスト計算パラメータ */
export type AugmentCostParams = {
  tool: AugmentToolType;
  width: number;
  height: number;
  tier?: SubscriptionTier;      // デフォルト: 0
};

/** Augmentコスト計算結果 */
export type AugmentCostResult = {
  originalPixels: number;
  adjustedWidth: number;
  adjustedHeight: number;
  adjustedPixels: number;
  baseCost: number;
  finalCost: number;
  isOpusFree: boolean;
  effectiveCost: number;
};

/** アップスケールコスト計算パラメータ */
export type UpscaleCostParams = {
  width: number;
  height: number;
  tier?: SubscriptionTier;      // デフォルト: 0
};

/** アップスケールコスト計算結果 */
export type UpscaleCostResult = {
  pixels: number;
  cost: number | null;
  isOpusFree: boolean;
  error: boolean;
  errorCode: number | null;
};

/** Inpaintサイズ補正結果 */
export type InpaintCorrectionResult = {
  corrected: boolean;
  width: number;
  height: number;
};


// =============================================================================
// 基本計算関数
// =============================================================================

/**
 * V4モデルの基本コストを計算
 * ピクセル数とステップ数に基づく線形コスト計算
 * @param width 画像幅
 * @param height 画像高さ
 * @param steps ステップ数
 * @returns 基本コスト（Anlas）
 */
export function calcV4BaseCost(width: number, height: number, steps: number): number {
  const pixels = width * height;
  return Math.ceil(V4_COST_COEFF_LINEAR * pixels + V4_COST_COEFF_STEP * pixels * steps);
}

/**
 * SMEAモードに対応するコスト乗数を返す
 * @param mode SMEAモード
 * @returns 乗数（1.0 / 1.2 / 1.4）
 */
export function getSmeaMultiplier(mode: SmeaMode): number {
  switch (mode) {
    case 'smea_dyn': return 1.4;
    case 'smea':     return 1.2;
    case 'off':      return 1.0;
  }
}

/**
 * Opus無料生成の条件を判定
 * @param width 画像幅（元のリクエストサイズ）
 * @param height 画像高さ（元のリクエストサイズ）
 * @param steps ステップ数
 * @param charRefCount キャラクター参照数
 * @param tier サブスクリプションティア
 * @returns Opus無料かどうか
 */
export function isOpusFreeGeneration(
  width: number,
  height: number,
  steps: number,
  charRefCount: number,
  tier: SubscriptionTier,
): boolean {
  return charRefCount === 0
    && width * height <= OPUS_FREE_PIXELS
    && steps <= OPUS_FREE_MAX_STEPS
    && tier >= OPUS_MIN_TIER;
}

/**
 * Vibeバッチコストを計算
 * 無料枠（VIBE_FREE_THRESHOLD）を超えた分のみ課金
 * @param enabledVibeCount 有効なVibe数
 * @returns Vibeバッチコスト（Anlas）
 */
export function calcVibeBatchCost(enabledVibeCount: number): number {
  return Math.max(0, enabledVibeCount - VIBE_FREE_THRESHOLD) * VIBE_BATCH_PRICE;
}

/**
 * キャラクター参照コストを計算
 * @param charRefCount キャラクター参照数
 * @param nSamples 生成枚数
 * @returns キャラクター参照コスト（Anlas）
 */
export function calcCharRefCost(charRefCount: number, nSamples: number): number {
  return CHAR_REF_PRICE * charRefCount * nSamples;
}


// =============================================================================
// ピクセル数調整関数
// =============================================================================

/**
 * 最小ピクセル数まで拡大（アスペクト比維持）
 * グリッドスナップなし、Math.floorのみ適用
 * @param width 元の幅
 * @param height 元の高さ
 * @param minPixels 最小ピクセル数
 * @returns 調整後の幅・高さ・ピクセル数
 */
export function expandToMinPixels(
  width: number,
  height: number,
  minPixels: number,
): { width: number; height: number; pixels: number } {
  const pixels = width * height;
  if (pixels >= minPixels) {
    return { width, height, pixels };
  }
  const scale = Math.sqrt(minPixels / pixels);
  const newW = Math.floor(width * scale);
  const newH = Math.floor(height * scale);
  return { width: newW, height: newH, pixels: newW * newH };
}

/**
 * 最大ピクセル数まで縮小（アスペクト比維持）
 * グリッドスナップなし、Math.floorのみ適用
 * @param width 元の幅
 * @param height 元の高さ
 * @param maxPixels 最大ピクセル数
 * @returns 調整後の幅・高さ・ピクセル数
 */
export function clampToMaxPixels(
  width: number,
  height: number,
  maxPixels: number,
): { width: number; height: number; pixels: number } {
  const pixels = width * height;
  if (pixels <= maxPixels) {
    return { width, height, pixels };
  }
  const scale = Math.sqrt(maxPixels / pixels);
  const newW = Math.floor(width * scale);
  const newH = Math.floor(height * scale);
  return { width: newW, height: newH, pixels: newW * newH };
}


// =============================================================================
// Inpaint補正
// =============================================================================

/**
 * Inpaintマスクサイズの補正を計算
 * マスクが閾値より小さい場合、OPUS_FREE_PIXELSまで拡大してグリッドスナップ
 * @param maskWidth マスク幅
 * @param maskHeight マスク高さ
 * @returns 補正結果（補正されたかどうか・幅・高さ）
 */
export function calcInpaintSizeCorrection(
  maskWidth: number,
  maskHeight: number,
): InpaintCorrectionResult {
  const pixels = maskWidth * maskHeight;
  const threshold = OPUS_FREE_PIXELS * INPAINT_THRESHOLD_RATIO;

  if (pixels >= threshold) {
    return { corrected: false, width: maskWidth, height: maskHeight };
  }

  const scale = Math.sqrt(OPUS_FREE_PIXELS / pixels);
  const newW = Math.floor(Math.floor(maskWidth * scale) / GRID_SIZE) * GRID_SIZE;
  const newH = Math.floor(Math.floor(maskHeight * scale) / GRID_SIZE) * GRID_SIZE;

  return { corrected: true, width: newW, height: newH };
}


// =============================================================================
// メイン：画像生成コスト計算
// =============================================================================

/**
 * 画像生成のAnlasコストを計算（メインオーケストレータ）
 * すべてのコスト要素（基本コスト、SMEA、strength、Opus無料、Vibe、キャラクター参照）を
 * 統合して最終コストと内訳を返す。
 * @param params 生成パラメータ
 * @returns コスト内訳を含む計算結果
 */
export function calculateGenerationCost(params: GenerationCostParams): GenerationCostResult {
  // デフォルト値の適用
  const smea = params.smea ?? 'off';
  const mode = params.mode ?? 'txt2img';
  const strength = params.strength ?? 1.0;
  const nSamples = params.nSamples ?? 1;
  const tier = params.tier ?? 0;
  const charRefCount = params.charRefCount ?? 0;
  const vibeCount = params.vibeCount ?? 0;
  const vibeUnencodedCount = params.vibeUnencodedCount ?? 0;

  // 有効な幅・高さの決定（Inpaintマスク補正）
  let effectiveWidth = params.width;
  let effectiveHeight = params.height;

  if (mode === 'inpaint' && params.maskWidth != null && params.maskHeight != null) {
    const correction = calcInpaintSizeCorrection(params.maskWidth, params.maskHeight);
    if (correction.corrected) {
      effectiveWidth = correction.width;
      effectiveHeight = correction.height;
    }
  }

  // 基本コスト計算
  const baseCost = calcV4BaseCost(effectiveWidth, effectiveHeight, params.steps);

  // SMEA乗数
  const smeaMultiplier = getSmeaMultiplier(smea);
  const perImageCost = baseCost * smeaMultiplier;

  // strength乗数（txt2imgは常に1.0）
  let strengthMultiplier: number;
  switch (mode) {
    case 'txt2img':
      strengthMultiplier = 1.0;
      break;
    case 'img2img':
    case 'inpaint':
      strengthMultiplier = strength;
      break;
  }

  // 調整後コスト（最低MIN_COST_PER_IMAGE保証）
  const adjustedCost = Math.max(Math.ceil(perImageCost * strengthMultiplier), MIN_COST_PER_IMAGE);

  // エラー判定（最大コスト超過）
  const error = adjustedCost > MAX_COST_PER_IMAGE;
  const errorCode = error ? -3 : null;

  // Opus無料判定（元のリクエストサイズで判定）
  const isOpusFree = isOpusFreeGeneration(
    params.width,
    params.height,
    params.steps,
    charRefCount,
    tier,
  );

  // 課金対象枚数
  const billableImages = Math.max(0, nSamples - (isOpusFree ? 1 : 0));
  const generationCost = adjustedCost * billableImages;

  // Vibeコスト（キャラクター参照使用時・Inpaint時は無効）
  let vibeEncodeCost = 0;
  let vibeBatchCost = 0;
  if (charRefCount === 0 && mode !== 'inpaint') {
    vibeEncodeCost = vibeUnencodedCount * VIBE_ENCODE_PRICE;
    vibeBatchCost = calcVibeBatchCost(vibeCount);
  }

  // キャラクター参照コスト
  const charRefCost = charRefCount > 0 ? calcCharRefCost(charRefCount, nSamples) : 0;

  // 合計コスト
  const totalCost = generationCost + charRefCost + vibeEncodeCost + vibeBatchCost;

  return {
    baseCost,
    smeaMultiplier,
    perImageCost,
    strengthMultiplier,
    adjustedCost,
    isOpusFree,
    billableImages,
    generationCost,
    charRefCost,
    vibeEncodeCost,
    vibeBatchCost,
    totalCost,
    error,
    errorCode,
  };
}


// =============================================================================
// Augmentコスト計算
// =============================================================================

/**
 * Augmentツールのコストを計算
 * MAX_PIXELSでクランプ → AUGMENT_MIN_PIXELSまで拡大 → V4基本コスト計算
 * bg-removalは追加の乗数と加算あり
 * @param params Augmentパラメータ
 * @returns コスト計算結果
 */
export function calculateAugmentCost(params: AugmentCostParams): AugmentCostResult {
  const tier = params.tier ?? 0;
  const originalPixels = params.width * params.height;

  // MAX_PIXELSでクランプ
  const clamped = clampToMaxPixels(params.width, params.height, MAX_PIXELS);

  // AUGMENT_MIN_PIXELSまで拡大
  const expanded = expandToMinPixels(clamped.width, clamped.height, AUGMENT_MIN_PIXELS);

  // 基本コスト計算（固定ステップ数）
  const baseCost = calcV4BaseCost(expanded.width, expanded.height, AUGMENT_FIXED_STEPS);

  // bg-removalは特別計算
  let finalCost: number;
  if (params.tool === 'bg-removal') {
    finalCost = BG_REMOVAL_MULTIPLIER * baseCost + BG_REMOVAL_ADDEND;
  } else {
    finalCost = baseCost;
  }

  // Opus無料判定（bg-removal以外、かつ拡大後ピクセル数がOPUS_FREE_PIXELS以下）
  const isOpusFree = params.tool !== 'bg-removal'
    && expanded.width * expanded.height <= OPUS_FREE_PIXELS
    && tier >= OPUS_MIN_TIER;

  const effectiveCost = isOpusFree ? 0 : finalCost;

  return {
    originalPixels,
    adjustedWidth: expanded.width,
    adjustedHeight: expanded.height,
    adjustedPixels: expanded.width * expanded.height,
    baseCost,
    finalCost,
    isOpusFree,
    effectiveCost,
  };
}


// =============================================================================
// アップスケールコスト計算
// =============================================================================

/**
 * アップスケールのコストを計算
 * ピクセル数に基づくテーブル引きで価格を決定
 * @param params アップスケールパラメータ
 * @returns コスト計算結果
 */
export function calculateUpscaleCost(params: UpscaleCostParams): UpscaleCostResult {
  const tier = params.tier ?? 0;
  const pixels = params.width * params.height;

  // Opus無料判定
  if (tier >= OPUS_MIN_TIER && pixels <= UPSCALE_OPUS_FREE_PIXELS) {
    return { pixels, cost: 0, isOpusFree: true, error: false, errorCode: null };
  }

  // コストテーブルから該当価格を検索（降順テーブル、最後にマッチしたものが最小閾値）
  let cost: number | null = null;
  for (const [threshold, price] of UPSCALE_COST_TABLE) {
    if (pixels <= threshold) {
      cost = price;
    }
  }

  return {
    pixels,
    cost,
    isOpusFree: false,
    error: cost === null,
    errorCode: cost === null ? -3 : null,
  };
}
