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
  model: z.enum(Constants.VALID_MODELS).refine((val) => Constants.VALID_MODELS.includes(val as any), {
    message: `Invalid model. Valid models are: ${Constants.VALID_MODELS.join(", ")}`,
  }),
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
// GenerateParams
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
  inpaint_strength: z.number().min(0.0).max(1.0).default(Constants.DEFAULT_INPAINT_STRENGTH),
  inpaint_noise: z.number().min(0.0).max(1.0).default(Constants.DEFAULT_INPAINT_NOISE),
  inpaint_color_correct: z.boolean().default(Constants.DEFAULT_INPAINT_COLOR_CORRECT),

  // === キャラクター設定 ===
  characters: z.array(CharacterConfigSchema).max(Constants.MAX_CHARACTERS).optional().nullable(),

  // === Vibe Transfer ===
  vibes: z.array(z.any()).max(Constants.MAX_VIBES).optional().nullable(), // Vibes can be complex types, validation handled in client
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
  seed: z.number().int().min(0).max(Constants.MAX_SEED).optional().nullable(),
  sampler: z.enum(Constants.VALID_SAMPLERS).default(Constants.DEFAULT_SAMPLER),
  noise_schedule: z.enum(Constants.VALID_NOISE_SCHEDULES).default(Constants.DEFAULT_NOISE_SCHEDULE),
})
.superRefine((data, ctx) => {
  // 1. vibes と character_reference は同時使用不可
  if (data.vibes && data.vibes.length > 0 && data.character_reference) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibes and character_reference cannot be used together.",
      path: ["character_reference"],
    });
  }

  // 2. action="img2img" の場合は source_image が必須
  if (data.action === "img2img" && !data.source_image) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "source_image is required for img2img action",
      path: ["source_image"],
    });
  }

  // 2b. action="infill" の場合は source_image と mask が必須
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
  }

  // 2c. mask が指定されている場合は action が infill でなければならない
  if (data.mask && data.action !== "infill") {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "mask can only be used with action='infill'",
      path: ["mask"],
    });
  }

  // 3. vibes なしで vibe_strengths が指定されている
  if (data.vibe_strengths && (!data.vibes || data.vibes.length === 0)) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibe_strengths cannot be specified without vibes",
      path: ["vibe_strengths"],
    });
  }

  // 4. vibes なしで vibe_info_extracted が指定されている
  if (data.vibe_info_extracted && (!data.vibes || data.vibes.length === 0)) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "vibe_info_extracted cannot be specified without vibes",
      path: ["vibe_info_extracted"],
    });
  }

  // 5. vibes と vibe_strengths の長さが一致しない
  if (data.vibes && data.vibe_strengths) {
    if (data.vibes.length !== data.vibe_strengths.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `Mismatch between vibes count (${data.vibes.length}) and vibe_strengths count (${data.vibe_strengths.length})`,
        path: ["vibe_strengths"],
      });
    }
  }

  // 6. vibes と vibe_info_extracted の長さが一致しない
  if (data.vibes && data.vibe_info_extracted) {
    if (data.vibes.length !== data.vibe_info_extracted.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `Mismatch between vibes count (${data.vibes.length}) and vibe_info_extracted count (${data.vibe_info_extracted.length})`,
        path: ["vibe_info_extracted"],
      });
    }
  }

  // 7. width * height が MAX_PIXELS を超える
  const totalPixels = data.width * data.height;
  if (totalPixels > Constants.MAX_PIXELS) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: `Total pixels (${totalPixels}) exceeds limit (${Constants.MAX_PIXELS}). Current: ${data.width}x${data.height}`,
      path: ["width"], // Attach to width
    });
  }

  // 8. save_path と save_dir は同時指定不可
  if (data.save_path && data.save_dir) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "save_path and save_dir cannot be specified together. Use one or the other.",
      path: ["save_path"],
    });
  }
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

