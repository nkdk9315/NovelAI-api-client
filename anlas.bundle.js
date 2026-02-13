"use strict";
var Anlas = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // src/anlas-browser.ts
  var anlas_browser_exports = {};
  __export(anlas_browser_exports, {
    AUGMENT_FIXED_STEPS: () => AUGMENT_FIXED_STEPS,
    AUGMENT_MIN_PIXELS: () => AUGMENT_MIN_PIXELS,
    BG_REMOVAL_ADDEND: () => BG_REMOVAL_ADDEND,
    BG_REMOVAL_MULTIPLIER: () => BG_REMOVAL_MULTIPLIER,
    CHAR_REF_PRICE: () => CHAR_REF_PRICE,
    GRID_SIZE: () => GRID_SIZE,
    INPAINT_THRESHOLD_RATIO: () => INPAINT_THRESHOLD_RATIO,
    MAX_COST_PER_IMAGE: () => MAX_COST_PER_IMAGE,
    MAX_PIXELS: () => MAX_PIXELS,
    MIN_COST_PER_IMAGE: () => MIN_COST_PER_IMAGE,
    OPUS_FREE_PIXELS: () => OPUS_FREE_PIXELS,
    UPSCALE_OPUS_FREE_PIXELS: () => UPSCALE_OPUS_FREE_PIXELS,
    V4_COST_COEFF_LINEAR: () => V4_COST_COEFF_LINEAR,
    V4_COST_COEFF_STEP: () => V4_COST_COEFF_STEP,
    VIBE_ENCODE_PRICE: () => VIBE_ENCODE_PRICE,
    VIBE_FREE_THRESHOLD: () => VIBE_FREE_THRESHOLD,
    calcCharRefCost: () => calcCharRefCost,
    calcInpaintSizeCorrection: () => calcInpaintSizeCorrection,
    calcV4BaseCost: () => calcV4BaseCost,
    calcVibeBatchCost: () => calcVibeBatchCost,
    calculateAugmentCost: () => calculateAugmentCost,
    calculateGenerationCost: () => calculateGenerationCost,
    calculateUpscaleCost: () => calculateUpscaleCost,
    clampToMaxPixels: () => clampToMaxPixels,
    expandToMinPixels: () => expandToMinPixels,
    getSmeaMultiplier: () => getSmeaMultiplier,
    isOpusFreeGeneration: () => isOpusFreeGeneration
  });

  // src/constants.ts
  var DEFAULT_NEGATIVE = [
    "nsfw, lowres, artistic error, film grain, scan artifacts, ",
    "worst quality, bad quality, jpeg artifacts, very displeasing, ",
    "chromatic aberration, dithering, halftone, screentone"
  ].join("");
  var MAX_PIXELS = 3145728;
  var OPUS_FREE_PIXELS = 1048576;
  var OPUS_FREE_MAX_STEPS = 28;
  var OPUS_MIN_TIER = 3;
  var MAX_COST_PER_IMAGE = 140;
  var MIN_COST_PER_IMAGE = 2;
  var GRID_SIZE = 64;
  var VIBE_BATCH_PRICE = 2;
  var VIBE_FREE_THRESHOLD = 4;
  var VIBE_ENCODE_PRICE = 2;
  var CHAR_REF_PRICE = 5;
  var INPAINT_THRESHOLD_RATIO = 0.8;
  var V4_COST_COEFF_LINEAR = 2951823174884865e-21;
  var V4_COST_COEFF_STEP = 5753298233447344e-22;
  var AUGMENT_FIXED_STEPS = 28;
  var AUGMENT_MIN_PIXELS = 1048576;
  var BG_REMOVAL_MULTIPLIER = 3;
  var BG_REMOVAL_ADDEND = 5;
  var UPSCALE_COST_TABLE = [
    [1048576, 7],
    [786432, 5],
    [524288, 3],
    [409600, 2],
    [262144, 1]
  ];
  var UPSCALE_OPUS_FREE_PIXELS = 409600;

  // src/anlas.ts
  function calcV4BaseCost(width, height, steps) {
    const pixels = width * height;
    return Math.ceil(V4_COST_COEFF_LINEAR * pixels + V4_COST_COEFF_STEP * pixels * steps);
  }
  function getSmeaMultiplier(mode) {
    switch (mode) {
      case "smea_dyn":
        return 1.4;
      case "smea":
        return 1.2;
      case "off":
        return 1;
    }
  }
  function isOpusFreeGeneration(width, height, steps, charRefCount, tier) {
    return charRefCount === 0 && width * height <= OPUS_FREE_PIXELS && steps <= OPUS_FREE_MAX_STEPS && tier >= OPUS_MIN_TIER;
  }
  function calcVibeBatchCost(enabledVibeCount) {
    return Math.max(0, enabledVibeCount - VIBE_FREE_THRESHOLD) * VIBE_BATCH_PRICE;
  }
  function calcCharRefCost(charRefCount, nSamples) {
    return CHAR_REF_PRICE * charRefCount * nSamples;
  }
  function expandToMinPixels(width, height, minPixels) {
    const pixels = width * height;
    if (pixels >= minPixels) {
      return { width, height, pixels };
    }
    const scale = Math.sqrt(minPixels / pixels);
    const newW = Math.floor(width * scale);
    const newH = Math.floor(height * scale);
    return { width: newW, height: newH, pixels: newW * newH };
  }
  function clampToMaxPixels(width, height, maxPixels) {
    const pixels = width * height;
    if (pixels <= maxPixels) {
      return { width, height, pixels };
    }
    const scale = Math.sqrt(maxPixels / pixels);
    const newW = Math.floor(width * scale);
    const newH = Math.floor(height * scale);
    return { width: newW, height: newH, pixels: newW * newH };
  }
  function calcInpaintSizeCorrection(maskWidth, maskHeight) {
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
  function calculateGenerationCost(params) {
    const smea = params.smea ?? "off";
    const mode = params.mode ?? "txt2img";
    const strength = params.strength ?? 1;
    const nSamples = params.nSamples ?? 1;
    const tier = params.tier ?? 0;
    const charRefCount = params.charRefCount ?? 0;
    const vibeCount = params.vibeCount ?? 0;
    const vibeUnencodedCount = params.vibeUnencodedCount ?? 0;
    let effectiveWidth = params.width;
    let effectiveHeight = params.height;
    if (mode === "inpaint" && params.maskWidth != null && params.maskHeight != null) {
      const correction = calcInpaintSizeCorrection(params.maskWidth, params.maskHeight);
      if (correction.corrected) {
        effectiveWidth = correction.width;
        effectiveHeight = correction.height;
      }
    }
    const baseCost = calcV4BaseCost(effectiveWidth, effectiveHeight, params.steps);
    const smeaMultiplier = getSmeaMultiplier(smea);
    const perImageCost = baseCost * smeaMultiplier;
    let strengthMultiplier;
    switch (mode) {
      case "txt2img":
        strengthMultiplier = 1;
        break;
      case "img2img":
      case "inpaint":
        strengthMultiplier = strength;
        break;
    }
    const adjustedCost = Math.max(Math.ceil(perImageCost * strengthMultiplier), MIN_COST_PER_IMAGE);
    const error = adjustedCost > MAX_COST_PER_IMAGE;
    const errorCode = error ? -3 : null;
    const isOpusFree = isOpusFreeGeneration(
      params.width,
      params.height,
      params.steps,
      charRefCount,
      tier
    );
    const billableImages = Math.max(0, nSamples - (isOpusFree ? 1 : 0));
    const generationCost = adjustedCost * billableImages;
    let vibeEncodeCost = 0;
    let vibeBatchCost = 0;
    if (charRefCount === 0 && mode !== "inpaint") {
      vibeEncodeCost = vibeUnencodedCount * VIBE_ENCODE_PRICE;
      vibeBatchCost = calcVibeBatchCost(vibeCount);
    }
    const charRefCost = charRefCount > 0 ? calcCharRefCost(charRefCount, nSamples) : 0;
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
      errorCode
    };
  }
  function calculateAugmentCost(params) {
    const tier = params.tier ?? 0;
    const originalPixels = params.width * params.height;
    const clamped = clampToMaxPixels(params.width, params.height, MAX_PIXELS);
    const expanded = expandToMinPixels(clamped.width, clamped.height, AUGMENT_MIN_PIXELS);
    const baseCost = calcV4BaseCost(expanded.width, expanded.height, AUGMENT_FIXED_STEPS);
    let finalCost;
    if (params.tool === "bg-removal") {
      finalCost = BG_REMOVAL_MULTIPLIER * baseCost + BG_REMOVAL_ADDEND;
    } else {
      finalCost = baseCost;
    }
    const isOpusFree = params.tool !== "bg-removal" && expanded.width * expanded.height <= OPUS_FREE_PIXELS && tier >= OPUS_MIN_TIER;
    const effectiveCost = isOpusFree ? 0 : finalCost;
    return {
      originalPixels,
      adjustedWidth: expanded.width,
      adjustedHeight: expanded.height,
      adjustedPixels: expanded.width * expanded.height,
      baseCost,
      finalCost,
      isOpusFree,
      effectiveCost
    };
  }
  function calculateUpscaleCost(params) {
    const tier = params.tier ?? 0;
    const pixels = params.width * params.height;
    if (tier >= OPUS_MIN_TIER && pixels <= UPSCALE_OPUS_FREE_PIXELS) {
      return { pixels, cost: 0, isOpusFree: true, error: false, errorCode: null };
    }
    let cost = null;
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
      errorCode: cost === null ? -3 : null
    };
  }
  return __toCommonJS(anlas_browser_exports);
})();
