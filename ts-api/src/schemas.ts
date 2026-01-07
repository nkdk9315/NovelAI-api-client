/**
 * NovelAI Client Zod Schemas
 * バリデーション付きデータモデル
 */

import { z } from "zod";
import * as Constants from "./constants";

// =============================================================================
// CharacterConfig
// =============================================================================

export const CharacterConfigSchema = z.object({
  prompt: z.string().min(1).max(Constants.MAX_PROMPT_CHARS),
  center_x: z.number().min(0.0).max(1.0).default(0.5),
  center_y: z.number().min(0.0).max(1.0).default(0.5),
  negative_prompt: z.string().max(Constants.MAX_PROMPT_CHARS).default(""),
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

// Image input can be string (path or base64) or Buffer
const ImageInputSchema = z.union([z.string(), z.instanceof(Buffer)]);

export const CharacterReferenceConfigSchema = z.object({
  image: ImageInputSchema,
  fidelity: z.number().min(0.0).max(1.0).default(1.0),
  include_style: z.boolean().default(true),
});

export type CharacterReferenceConfig = z.infer<typeof CharacterReferenceConfigSchema>;


// =============================================================================
// VibeEncodeResult
// =============================================================================

export const VibeEncodeResultSchema = z.object({
  encoding: z.string().min(1),
  model: z.enum(Constants.VALID_MODELS),
  information_extracted: z.number().min(0.0).max(1.0),
  strength: z.number().min(0.0).max(1.0),
  source_image_hash: z.string().regex(/^[a-f0-9]{64}$/),
  created_at: z.date(),
  saved_path: z.string().optional().nullable(),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
});

export type VibeEncodeResult = z.infer<typeof VibeEncodeResultSchema>;


// =============================================================================
// GenerateResult
// =============================================================================

export const GenerateResultSchema = z.object({
  image_data: z.instanceof(Buffer),
  seed: z.number().min(0).max(Constants.MAX_SEED),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
  saved_path: z.string().optional().nullable(),
});

export type GenerateResult = z.infer<typeof GenerateResultSchema>;


// =============================================================================
// GenerateParams - Validation Helper Functions
// =============================================================================

// Type alias for refinement context
type RefinementCtx = z.RefinementCtx;

// Raw input type before validation (used by validation helpers)
type GenerateParamsRaw = {
  prompt: string;
  action: "generate" | "img2img" | "infill";
  source_image?: string | Buffer | null;
  mask?: string | Buffer | null;
  mask_strength?: number | null;
  vibes?: any[] | null;
  vibe_strengths?: number[] | null;
  vibe_info_extracted?: number[] | null;
  character_reference?: z.infer<typeof CharacterReferenceConfigSchema> | null;
  characters?: z.infer<typeof CharacterConfigSchema>[] | null;
  negative_prompt?: string | null;
  save_path?: string | null;
  save_dir?: string | null;
  width: number;
  height: number;
};

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
  if (data.vibe_strengths && !hasVibes) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibe_strengths cannot be specified without vibes",
      path: ["vibe_strengths"],
    });
  }

  // vibes なしで vibe_info_extracted が指定されている
  if (data.vibe_info_extracted && !hasVibes) {
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
 * Validate save options (save_path and save_dir are mutually exclusive)
 */
function validateSaveOptions(data: GenerateParamsRaw, ctx: RefinementCtx): void {
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together. Use one or the other.",
      path: ["save_path"],
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
    // Network errors or tokenizer errors are logged but don't block validation
    if (error.name !== 'TokenValidationError' && error.name !== 'TokenizerError') {
      console.warn('[NovelAI] Token validation skipped due to error:', error.message);
    }
  }
}

// =============================================================================
// GenerateParams Schema Definition
// =============================================================================

export const GenerateParamsSchema = z.object({
  // === 基本プロンプト ===
  prompt: z.string().min(0).max(Constants.MAX_PROMPT_CHARS),

  // === Action & Image2Image ===
  action: z.enum(["generate", "img2img", "infill"]).default("generate"),
  source_image: ImageInputSchema.optional().nullable(),
  img2img_strength: z.number().min(0.0).max(1.0).default(Constants.DEFAULT_IMG2IMG_STRENGTH),
  img2img_noise: z.number().min(0.0).max(1.0).default(0.0),

  // === Inpaint/Mask ===
  mask: ImageInputSchema.optional().nullable(),
  /** Mask application strength (0.01-1). Required for infill action. */
  mask_strength: z.number().min(0.01).max(1.0).optional().nullable(),
  inpaint_color_correct: z.boolean().default(Constants.DEFAULT_INPAINT_COLOR_CORRECT),

  // === Hybrid Mode (Mask + Img2Img) ===
  /** Img2Img strength when used with mask. Controls how much original image influences (0.01-0.99). */
  hybrid_img2img_strength: z.number().min(0.01).max(0.99).optional().nullable(),
  /** Img2Img noise when used with mask (0-0.99). */
  hybrid_img2img_noise: z.number().min(0.0).max(0.99).optional().nullable(),

  // === キャラクター設定 ===
  characters: z.array(CharacterConfigSchema).max(Constants.MAX_CHARACTERS).optional().nullable(),

  // === Vibe Transfer ===
  vibes: z.array(z.any()).max(Constants.MAX_VIBES).optional().nullable(),
  vibe_strengths: z.array(z.number().min(0.0).max(1.0)).optional().nullable(),
  vibe_info_extracted: z.array(z.number().min(0.0).max(1.0)).optional().nullable(),

  // === Character Reference ===
  character_reference: CharacterReferenceConfigSchema.optional().nullable(),

  // === プロンプト ===
  negative_prompt: z.string().max(Constants.MAX_PROMPT_CHARS).optional().nullable(),

  // === 出力オプション ===
  save_path: z.string().optional().nullable(),
  save_dir: z.string().optional().nullable(),

  // === 生成パラメータ ===
  model: z.enum(Constants.VALID_MODELS).default(Constants.DEFAULT_MODEL),
  width: z.number().int().min(Constants.MIN_DIMENSION).default(Constants.DEFAULT_WIDTH)
    .refine(val => val % 64 === 0, { message: "Width must be a multiple of 64" }),
  height: z.number().int().min(Constants.MIN_DIMENSION).default(Constants.DEFAULT_HEIGHT)
    .refine(val => val % 64 === 0, { message: "Height must be a multiple of 64" }),
  steps: z.number().int().min(Constants.MIN_STEPS).max(Constants.MAX_STEPS).default(Constants.DEFAULT_STEPS),
  scale: z.number().min(Constants.MIN_SCALE).max(Constants.MAX_SCALE).default(Constants.DEFAULT_SCALE),
  cfg_rescale: z.number().min(0).max(1).default(Constants.DEFAULT_CFG_RESCALE),
  seed: z.number().int().min(0).max(Constants.MAX_SEED).optional().nullable(),
  sampler: z.enum(Constants.VALID_SAMPLERS).default(Constants.DEFAULT_SAMPLER),
  noise_schedule: z.enum(Constants.VALID_NOISE_SCHEDULES).default(Constants.DEFAULT_NOISE_SCHEDULE),
})
.superRefine(async (data, ctx) => {
  // Delegate to focused validation functions
  validateActionDependencies(data as GenerateParamsRaw, ctx);
  validateVibeParams(data as GenerateParamsRaw, ctx);
  validatePixelConstraints(data as GenerateParamsRaw, ctx);
  validateSaveOptions(data as GenerateParamsRaw, ctx);
  await validateTokenCounts(data as GenerateParamsRaw, ctx);
});

// Helper type for validated params (all fields present after .parse())
type GenerateParamsValidated = z.infer<typeof GenerateParamsSchema>;

// Input type - fields with defaults are optional
export type GenerateParams = Pick<GenerateParamsValidated, 'prompt'> & 
  Partial<Omit<GenerateParamsValidated, 'prompt'>>;


// =============================================================================
// EncodeVibeParams
// =============================================================================

export const EncodeVibeParamsSchema = z.object({
  image: ImageInputSchema,
  model: z.enum(Constants.VALID_MODELS).default(Constants.DEFAULT_MODEL),
  information_extracted: z.number().min(0.0).max(1.0).default(0.7),
  strength: z.number().min(0.0).max(1.0).default(0.7),
  save_path: z.string().optional().nullable(),
  save_dir: z.string().optional().nullable(),
  /** Custom filename for the .naiv4vibe file (without extension). If not provided, auto-generated. */
  save_filename: z.string().optional().nullable(),
})
.superRefine((data, ctx) => {
  // save_path と save_dir は同時指定不可
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together. Use one or the other.",
      path: ["save_path"],
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

// Helper type for validated params
type EncodeVibeParamsValidated = z.infer<typeof EncodeVibeParamsSchema>;

// Input type - fields with defaults are optional  
export type EncodeVibeParams = Pick<EncodeVibeParamsValidated, 'image'> &
  Partial<Omit<EncodeVibeParamsValidated, 'image'>>;


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
  prompt: z.string().optional().nullable(),
  defry: z.number().int().min(Constants.MIN_DEFRY).max(Constants.MAX_DEFRY).optional().nullable(),
  
  // 出力オプション
  save_path: z.string().optional().nullable(),
  save_dir: z.string().optional().nullable(),
})
.superRefine((data, ctx) => {
  const reqTypesRequiringDefry = ["colorize", "emotion"] as const;
  const reqTypesWithNoExtraParams = ["declutter", "sketch", "lineart", "bg-removal"] as const;
  
  // === colorize / emotion の場合 ===
  if (reqTypesRequiringDefry.includes(data.req_type as any)) {
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
      if (!Constants.EMOTION_KEYWORDS.includes(data.prompt as any)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `Invalid emotion keyword '${data.prompt}'. Valid: ${Constants.EMOTION_KEYWORDS.join(", ")}`,
          path: ["prompt"],
        });
      }
    }
  }
  
  // === declutter, sketch, lineart, bg-removal の場合 ===
  if (reqTypesWithNoExtraParams.includes(data.req_type as any)) {
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
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together.",
      path: ["save_path"],
    });
  }
});

// Helper type for validated params
type AugmentParamsValidated = z.infer<typeof AugmentParamsSchema>;

// Input type - req_type と image は必須、他はオプション
// colorize/emotion の場合は defry が実質必須（superRefine でチェック）
export type AugmentParams = Pick<AugmentParamsValidated, 'req_type' | 'image'> &
  Partial<Omit<AugmentParamsValidated, 'req_type' | 'image'>>;


// =============================================================================
// AugmentResult
// =============================================================================

export const AugmentResultSchema = z.object({
  image_data: z.instanceof(Buffer),
  req_type: z.enum(Constants.AUGMENT_REQ_TYPES),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
  saved_path: z.string().optional().nullable(),
});

export type AugmentResult = z.infer<typeof AugmentResultSchema>;


// =============================================================================
// UpscaleParams (画像拡大)
// =============================================================================

export const UpscaleParamsSchema = z.object({
  image: ImageInputSchema,
  scale: z.number().int().refine(
    (val) => Constants.VALID_UPSCALE_SCALES.includes(val as any),
    { message: `scale must be one of: ${Constants.VALID_UPSCALE_SCALES.join(", ")}` }
  ).default(Constants.DEFAULT_UPSCALE_SCALE),
  
  // 出力オプション
  save_path: z.string().optional().nullable(),
  save_dir: z.string().optional().nullable(),
})
.superRefine((data, ctx) => {
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together.",
      path: ["save_path"],
    });
  }
});

// Helper type for validated params
type UpscaleParamsValidated = z.infer<typeof UpscaleParamsSchema>;

// Input type - fields with defaults are optional
export type UpscaleParams = Pick<UpscaleParamsValidated, 'image'> &
  Partial<Omit<UpscaleParamsValidated, 'image'>>;


// =============================================================================
// UpscaleResult
// =============================================================================

export const UpscaleResultSchema = z.object({
  image_data: z.instanceof(Buffer),
  scale: z.number(),
  output_width: z.number(),
  output_height: z.number(),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
  saved_path: z.string().optional().nullable(),
});

export type UpscaleResult = z.infer<typeof UpscaleResultSchema>;
