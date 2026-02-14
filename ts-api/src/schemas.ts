/**
 * NovelAI Client Zod Schemas
 * バリデーション付きデータモデル
 */

import { z } from "zod";
import * as Constants from "./constants";

// =============================================================================
// Shared Infrastructure
// =============================================================================

/** Path traversal prevention schema */
const SafePathSchema = z.string().refine(
  (val) => !val.replace(/\\/g, '/').includes('..'),
  { message: "Path must not contain '..' (path traversal)" }
);

/** Browser-compatible binary data schema (Buffer | Uint8Array) */
const BinaryDataSchema = z.union([z.instanceof(Buffer), z.instanceof(Uint8Array)]);

// Type alias for refinement context
type RefinementCtx = z.RefinementCtx;

/**
 * Validate save_path and save_dir are mutually exclusive.
 * Shared across GenerateParams, EncodeVibeParams, AugmentParams, UpscaleParams.
 */
function validateSaveOptionsExclusive(
  data: { save_path?: string | null; save_dir?: string | null },
  ctx: RefinementCtx
): void {
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together. Use one or the other.",
      path: ["save_path"],
    });
  }
}


// =============================================================================
// CharacterConfig
// =============================================================================

export const CharacterConfigSchema = z.object({
  prompt: z.string().min(1),
  center_x: z.number().min(0.0).max(1.0).default(0.5),
  center_y: z.number().min(0.0).max(1.0).default(0.5),
  negative_prompt: z.string().default(""),
});

export type CharacterConfig = z.infer<typeof CharacterConfigSchema>;

// Helper methods (simulated via functions since Zod schemas are just data validators)
export function characterToCaptionDict(config: CharacterConfig) {
  return {
    char_caption: config.prompt,
    centers: [{ x: config.center_x, y: config.center_y }],
  };
}

export function characterToNegativeCaptionDict(config: CharacterConfig) {
  return {
    char_caption: config.negative_prompt,
    centers: [{ x: config.center_x, y: config.center_y }],
  };
}


// =============================================================================
// CharacterReferenceConfig
// =============================================================================

// Image input can be string (path or base64) or Buffer/Uint8Array
const ImageInputSchema = z.union([
  z.string().min(1).refine(
    (val) => {
      // Skip traversal check for data URLs and Base64 strings
      if (val.startsWith('data:')) return true;
      if (/^[A-Za-z0-9+/\-_]+=*$/.test(val) && val.length > 64) return true;
      // For file-path-like strings, check for path traversal
      return !val.replace(/\\/g, '/').includes('..');
    },
    { message: "Path must not contain '..' (path traversal)" }
  ),
  BinaryDataSchema,
]);

export const CHARREF_MODES = ["character", "character&style", "style"] as const;

export const CharacterReferenceConfigSchema = z.object({
  image: ImageInputSchema,
  strength: z.number().min(0.0).max(1.0).default(0.6),
  fidelity: z.number().min(0.0).max(1.0).default(1.0),
  mode: z.enum(CHARREF_MODES).default("character&style"),
});

export type CharacterReferenceConfig = z.infer<typeof CharacterReferenceConfigSchema>;


// =============================================================================
// VibeEncodeResult
// =============================================================================

export const VibeEncodeResultSchema = z.object({
  encoding: z.string().min(1).max(Constants.MAX_VIBE_ENCODING_LENGTH).refine(
    (val) => /^[A-Za-z0-9+/]+=*$/.test(val),
    { message: "encoding must be valid base64" }
  ),
  model: z.enum(Constants.VALID_MODELS),
  information_extracted: z.number().min(0.0).max(1.0),
  strength: z.number().min(0.0).max(1.0),
  source_image_hash: z.string().regex(/^[a-fA-F0-9]{64}$/),
  created_at: z.date(),
  saved_path: z.string().nullish(),
  anlas_remaining: z.number().min(0).nullish(),
  anlas_consumed: z.number().min(0).nullish(),
});

export type VibeEncodeResult = z.infer<typeof VibeEncodeResultSchema>;

/** Typed vibe item: either a pre-encoded VibeEncodeResult or a file path string */
const VibeItemSchema = z.union([VibeEncodeResultSchema, z.string().min(1)]);


// =============================================================================
// GenerateResult
// =============================================================================

export const GenerateResultSchema = z.object({
  image_data: BinaryDataSchema,
  seed: z.number().int().min(0).max(Constants.MAX_SEED),
  anlas_remaining: z.number().min(0).nullish(),
  anlas_consumed: z.number().min(0).nullish(),
  saved_path: z.string().nullish(),
});

export type GenerateResult = z.infer<typeof GenerateResultSchema>;


// =============================================================================
// GenerateParams - Base Schema & Inferred Type
// =============================================================================

const GenerateParamsBaseSchema = z.object({
  // === 基本プロンプト ===
  // .min(0): 空プロンプトは意図的に許容（vibes/img2imgのみの使用ケース）
  prompt: z.string().min(0),

  // === Action & Image2Image ===
  action: z.enum(["generate", "img2img", "infill"]).default("generate"),
  source_image: ImageInputSchema.nullish(),
  img2img_strength: z.number().min(0.0).max(1.0).default(Constants.DEFAULT_IMG2IMG_STRENGTH),
  img2img_noise: z.number().min(0.0).max(1.0).default(0.0),

  // === Inpaint/Mask ===
  mask: ImageInputSchema.nullish(),
  /** Mask application strength (0.01-1). Required for infill action. */
  mask_strength: z.number().min(0.01).max(1.0).nullish(),
  inpaint_color_correct: z.boolean().default(Constants.DEFAULT_INPAINT_COLOR_CORRECT),

  // === Hybrid Mode (Mask + Img2Img) ===
  /** Img2Img strength when used with mask. Controls how much original image influences (0.01-0.99). */
  hybrid_img2img_strength: z.number().min(0.01).max(0.99).nullish(),
  /** Img2Img noise when used with mask (0-0.99). */
  hybrid_img2img_noise: z.number().min(0.0).max(0.99).nullish(),

  // === キャラクター設定 ===
  characters: z.array(CharacterConfigSchema).max(Constants.MAX_CHARACTERS).nullish(),

  // === Vibe Transfer ===
  vibes: z.array(VibeItemSchema).max(Constants.MAX_VIBES).nullish(),
  vibe_strengths: z.array(z.number().min(0.0).max(1.0)).nullish(),
  vibe_info_extracted: z.array(z.number().min(0.0).max(1.0)).nullish(),

  // === Character Reference ===
  character_reference: CharacterReferenceConfigSchema.nullish(),

  // === プロンプト ===
  negative_prompt: z.string().nullish(),

  // === 出力オプション ===
  save_path: SafePathSchema.nullish(),
  save_dir: SafePathSchema.nullish(),

  // === 生成パラメータ ===
  model: z.enum(Constants.VALID_MODELS).default(Constants.DEFAULT_MODEL),
  width: z.number().int().min(Constants.MIN_DIMENSION).max(Constants.MAX_GENERATION_DIMENSION).default(Constants.DEFAULT_WIDTH)
    .refine(val => val % 64 === 0, { message: "Width must be a multiple of 64" }),
  height: z.number().int().min(Constants.MIN_DIMENSION).max(Constants.MAX_GENERATION_DIMENSION).default(Constants.DEFAULT_HEIGHT)
    .refine(val => val % 64 === 0, { message: "Height must be a multiple of 64" }),
  steps: z.number().int().min(Constants.MIN_STEPS).max(Constants.MAX_STEPS).default(Constants.DEFAULT_STEPS),
  scale: z.number().min(Constants.MIN_SCALE).max(Constants.MAX_SCALE).default(Constants.DEFAULT_SCALE),
  cfg_rescale: z.number().min(0).max(1).default(Constants.DEFAULT_CFG_RESCALE),
  seed: z.number().int().min(0).max(Constants.MAX_SEED).nullish(),
  sampler: z.enum(Constants.VALID_SAMPLERS).default(Constants.DEFAULT_SAMPLER),
  noise_schedule: z.enum(Constants.VALID_NOISE_SCHEDULES).default(Constants.DEFAULT_NOISE_SCHEDULE),
});

/** Inferred type from base schema (used by validation helpers - no manual type needed) */
type GenerateParamsRaw = z.infer<typeof GenerateParamsBaseSchema>;


// =============================================================================
// GenerateParams - Validation Helper Functions
// =============================================================================

/**
 * Validate action-dependent requirements (img2img, infill)
 */
function validateActionDependencies(data: GenerateParamsRaw, ctx: RefinementCtx): void {
  // vibes と character_reference は同時使用不可
  if (data.vibes && data.vibes.length > 0 && data.character_reference) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibes and character_reference cannot be used together.",
      path: ["character_reference"],
    });
  }

  // action="generate" で source_image が指定されている場合は警告
  if (data.action === "generate" && data.source_image) {
    console.warn('[NovelAI] source_image is specified but action is "generate". Did you mean action="img2img"?');
  }

  // action="img2img" の場合は source_image が必須
  if (data.action === "img2img" && !data.source_image) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "source_image is required for img2img action",
      path: ["source_image"],
    });
  }

  // action="infill" の場合は source_image, mask, mask_strength が必須
  if (data.action === "infill") {
    if (!data.source_image) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "source_image is required for infill action",
        path: ["source_image"],
      });
    }
    if (!data.mask) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "mask is required for infill action",
        path: ["mask"],
      });
    }
    if (data.mask_strength === undefined || data.mask_strength === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "mask_strength is required for infill action",
        path: ["mask_strength"],
      });
    }
  }

  // mask が指定されている場合は action が infill でなければならない
  if (data.mask && data.action !== "infill") {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "mask can only be used with action='infill'",
      path: ["mask"],
    });
  }
}

/**
 * Validate vibe-related parameters and array length consistency
 */
function validateVibeParams(data: GenerateParamsRaw, ctx: RefinementCtx): void {
  const hasVibes = data.vibes && data.vibes.length > 0;

  // vibes なしで vibe_strengths が指定されている
  if (data.vibe_strengths && data.vibe_strengths.length > 0 && !hasVibes) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibe_strengths cannot be specified without vibes",
      path: ["vibe_strengths"],
    });
  }

  // vibes なしで vibe_info_extracted が指定されている
  if (data.vibe_info_extracted && data.vibe_info_extracted.length > 0 && !hasVibes) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibe_info_extracted cannot be specified without vibes",
      path: ["vibe_info_extracted"],
    });
  }

  // vibes と vibe_strengths の長さが一致しない
  if (data.vibes && data.vibe_strengths) {
    if (data.vibes.length !== data.vibe_strengths.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `Mismatch between vibes count (${data.vibes.length}) and vibe_strengths count (${data.vibe_strengths.length})`,
        path: ["vibe_strengths"],
      });
    }
  }

  // vibes と vibe_info_extracted の長さが一致しない
  if (data.vibes && data.vibe_info_extracted) {
    if (data.vibes.length !== data.vibe_info_extracted.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `Mismatch between vibes count (${data.vibes.length}) and vibe_info_extracted count (${data.vibe_info_extracted.length})`,
        path: ["vibe_info_extracted"],
      });
    }
  }
}

/**
 * Validate pixel constraints (total pixels <= MAX_PIXELS)
 */
function validatePixelConstraints(data: GenerateParamsRaw, ctx: RefinementCtx): void {
  const totalPixels = data.width * data.height;
  if (totalPixels > Constants.MAX_PIXELS) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: `Total pixels (${totalPixels}) exceeds limit (${Constants.MAX_PIXELS}). Current: ${data.width}x${data.height}`,
      path: ["width"],
    });
  }
}

/**
 * Validate token counts for positive and negative prompts (async)
 */
async function validateTokenCounts(data: GenerateParamsRaw, ctx: RefinementCtx): Promise<void> {
  try {
    const { getT5Tokenizer, MAX_TOKENS } = await import('./tokenizer');
    const tokenizer = await getT5Tokenizer();

    // === ポジティブプロンプトの合計トークン数 ===
    const positivePrompts: string[] = [];
    if (data.prompt && data.prompt.length > 0) {
      positivePrompts.push(data.prompt);
    }
    if (data.characters && data.characters.length > 0) {
      for (const char of data.characters) {
        if (char.prompt && char.prompt.length > 0) {
          positivePrompts.push(char.prompt);
        }
      }
    }

    if (positivePrompts.length > 0) {
      let totalPositiveTokens = 0;
      for (const prompt of positivePrompts) {
        totalPositiveTokens += await tokenizer.countTokens(prompt);
      }
      if (totalPositiveTokens > MAX_TOKENS) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `Total positive prompt token count (${totalPositiveTokens}) exceeds maximum allowed (${MAX_TOKENS}). Base prompt + all character prompts must be <= ${MAX_TOKENS} tokens.`,
          path: ["prompt"],
        });
      }
    }

    // === ネガティブプロンプトの合計トークン数 ===
    const negativePrompts: string[] = [];
    if (data.negative_prompt && data.negative_prompt.length > 0) {
      negativePrompts.push(data.negative_prompt);
    }
    if (data.characters && data.characters.length > 0) {
      for (const char of data.characters) {
        if (char.negative_prompt && char.negative_prompt.length > 0) {
          negativePrompts.push(char.negative_prompt);
        }
      }
    }

    if (negativePrompts.length > 0) {
      let totalNegativeTokens = 0;
      for (const prompt of negativePrompts) {
        totalNegativeTokens += await tokenizer.countTokens(prompt);
      }
      if (totalNegativeTokens > MAX_TOKENS) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `Total negative prompt token count (${totalNegativeTokens}) exceeds maximum allowed (${MAX_TOKENS}). Base negative prompt + all character negative prompts must be <= ${MAX_TOKENS} tokens.`,
          path: ["negative_prompt"],
        });
      }
    }
  } catch (error: any) {
    if (error.name === 'TokenValidationError') {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: error.message,
        path: ["prompt"],
      });
    } else if (error.name === 'TokenizerError') {
      console.warn('[NovelAI] Token validation skipped - tokenizer unavailable:', error.message);
    } else {
      console.warn('[NovelAI] Token validation skipped due to unexpected error:', error.message);
    }
  }
}

// =============================================================================
// GenerateParams Schema Definition (with validation)
// =============================================================================

export const GenerateParamsSchema = GenerateParamsBaseSchema
.superRefine(async (data, ctx) => {
  // Delegate to focused validation functions
  validateActionDependencies(data, ctx);
  validateVibeParams(data, ctx);
  validatePixelConstraints(data, ctx);
  validateSaveOptionsExclusive(data, ctx);
  await validateTokenCounts(data, ctx);
});

// Input type - reflects what callers pass in (before defaults are applied)
export type GenerateParams = z.input<typeof GenerateParamsSchema>;


// =============================================================================
// EncodeVibeParams
// =============================================================================

export const EncodeVibeParamsSchema = z.object({
  image: ImageInputSchema,
  model: z.enum(Constants.VALID_MODELS).default(Constants.DEFAULT_MODEL),
  information_extracted: z.number().min(0.0).max(1.0).default(0.7),
  strength: z.number().min(0.0).max(1.0).default(0.7),
  save_path: SafePathSchema.nullish(),
  save_dir: SafePathSchema.nullish(),
  /** Custom filename for the .naiv4vibe file (without extension). If not provided, auto-generated. */
  save_filename: z.string().nullish(),
})
.superRefine((data, ctx) => {
  // save_path と save_dir は同時指定不可
  validateSaveOptionsExclusive(data, ctx);

  // save_filename と save_path は同時指定不可
  if (data.save_filename && data.save_path) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_filename and save_path cannot be specified together. Use save_dir with save_filename instead.",
      path: ["save_filename"],
    });
  }

  // save_filename は save_dir と一緒に使う必要がある
  if (data.save_filename && !data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_filename requires save_dir to be specified.",
      path: ["save_filename"],
    });
  }
});

// Input type - reflects what callers pass in (before defaults are applied)
export type EncodeVibeParams = z.input<typeof EncodeVibeParamsSchema>;


// =============================================================================
// AugmentParams (画像加工ツール)
// =============================================================================

// req_type によって必要な引数が異なる：
// - colorize: defry必須(0-5), promptオプション
// - emotion: defry必須(0-5), prompt必須(指定キーワード)
// - その他(declutter, sketch, lineart, bg-removal): prompt/defry使用不可

export const AugmentParamsSchema = z.object({
  req_type: z.enum(Constants.AUGMENT_REQ_TYPES),
  image: ImageInputSchema,

  // colorize, emotion用のみ
  prompt: z.string().nullish(),
  defry: z.number().int().min(Constants.MIN_DEFRY).max(Constants.MAX_DEFRY).nullish(),

  // 出力オプション
  save_path: SafePathSchema.nullish(),
  save_dir: SafePathSchema.nullish(),
})
.superRefine((data, ctx) => {
  const reqTypesRequiringDefry = ["colorize", "emotion"] as const;
  const reqTypesWithNoExtraParams = ["declutter", "sketch", "lineart", "bg-removal"] as const;

  // === colorize / emotion の場合 ===
  if ((reqTypesRequiringDefry as readonly string[]).includes(data.req_type)) {
    // defry は必須
    if (data.defry === undefined || data.defry === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `defry (0-5) is required for ${data.req_type}`,
        path: ["defry"],
      });
    }
  }

  // === emotion の場合 ===
  if (data.req_type === "emotion") {
    // prompt は必須
    if (!data.prompt) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "prompt is required for emotion (e.g., 'happy;;', 'sad;;')",
        path: ["prompt"],
      });
    }

    // prompt は有効なキーワードである必要がある（キーワードのみ、;;は不要）
    if (data.prompt) {
      if (!(Constants.EMOTION_KEYWORDS as readonly string[]).includes(data.prompt)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `Invalid emotion keyword '${data.prompt}'. Valid: ${Constants.EMOTION_KEYWORDS.join(", ")}`,
          path: ["prompt"],
        });
      }
    }
  }

  // === declutter, sketch, lineart, bg-removal の場合 ===
  if ((reqTypesWithNoExtraParams as readonly string[]).includes(data.req_type)) {
    // prompt は使用不可
    if (data.prompt !== undefined && data.prompt !== null && data.prompt !== "") {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `prompt cannot be used with ${data.req_type}`,
        path: ["prompt"],
      });
    }

    // defry は使用不可
    if (data.defry !== undefined && data.defry !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `defry cannot be used with ${data.req_type}`,
        path: ["defry"],
      });
    }
  }

  // save_path と save_dir は同時指定不可
  validateSaveOptionsExclusive(data, ctx);
});

// Input type - reflects what callers pass in
export type AugmentParams = z.input<typeof AugmentParamsSchema>;


// =============================================================================
// AugmentResult
// =============================================================================

export const AugmentResultSchema = z.object({
  image_data: BinaryDataSchema,
  req_type: z.enum(Constants.AUGMENT_REQ_TYPES),
  anlas_remaining: z.number().min(0).nullish(),
  anlas_consumed: z.number().min(0).nullish(),
  saved_path: z.string().nullish(),
});

export type AugmentResult = z.infer<typeof AugmentResultSchema>;


// =============================================================================
// UpscaleParams (画像拡大)
// =============================================================================

export const UpscaleParamsSchema = z.object({
  image: ImageInputSchema,
  scale: z.number().int().refine(
    (val) => (Constants.VALID_UPSCALE_SCALES as readonly number[]).includes(val),
    { message: `scale must be one of: ${Constants.VALID_UPSCALE_SCALES.join(", ")}` }
  ).default(Constants.DEFAULT_UPSCALE_SCALE),

  // 出力オプション
  save_path: SafePathSchema.nullish(),
  save_dir: SafePathSchema.nullish(),
})
.superRefine((data, ctx) => {
  validateSaveOptionsExclusive(data, ctx);
});

// Input type - reflects what callers pass in (before defaults are applied)
export type UpscaleParams = z.input<typeof UpscaleParamsSchema>;


// =============================================================================
// UpscaleResult
// =============================================================================

export const UpscaleResultSchema = z.object({
  image_data: BinaryDataSchema,
  scale: z.number().int().refine(
    (val) => (Constants.VALID_UPSCALE_SCALES as readonly number[]).includes(val),
    { message: `scale must be one of: ${Constants.VALID_UPSCALE_SCALES.join(", ")}` }
  ),
  output_width: z.number().int().min(1).max(Constants.MAX_GENERATION_DIMENSION * 4),
  output_height: z.number().int().min(1).max(Constants.MAX_GENERATION_DIMENSION * 4),
  anlas_remaining: z.number().min(0).nullish(),
  anlas_consumed: z.number().min(0).nullish(),
  saved_path: z.string().nullish(),
});

export type UpscaleResult = z.infer<typeof UpscaleResultSchema>;

// =============================================================================
// AnlasBalanceResponseSchema (API response validation)
// =============================================================================

export const AnlasBalanceResponseSchema = z.object({
  trainingStepsLeft: z.object({
    fixedTrainingStepsLeft: z.number().default(0),
    purchasedTrainingSteps: z.number().default(0),
  }).default({}),
  tier: z.number().int().min(0).max(3).default(0),
});

export type AnlasBalanceResponse = z.infer<typeof AnlasBalanceResponseSchema>;
