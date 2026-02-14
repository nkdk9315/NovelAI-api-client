/**
 * NovelAI Unified Image Generation Client
 * 統合されたVibe Transfer & Image2Image APIクライアント
 */
import fs from 'fs';
import fsp from 'fs/promises';
import path from 'path';
import crypto from 'crypto';
import AdmZip from 'adm-zip';
import { Unpackr } from 'msgpackr';
import * as Constants from './constants';
import * as Schemas from './schemas';
import * as Utils from './utils';

export interface Logger {
  warn(message: string, ...args: unknown[]): void;
  error(message: string, ...args: unknown[]): void;
}

const defaultLogger: Logger = {
  warn: console.warn.bind(console),
  error: console.error.bind(console),
};

// Helper types for return values
export interface AnlasBalance {
  fixed: number;
  purchased: number;
  total: number;
  tier: number;
}

interface GenerationPayloadParameters {
  params_version: number;
  width: number;
  height: number;
  scale: number;
  sampler: string;
  steps: number;
  n_samples: number;
  ucPreset: number;
  qualityToggle: boolean;
  autoSmea: boolean;
  dynamic_thresholding: boolean;
  controlnet_strength: number;
  legacy: boolean;
  add_original_image: boolean;
  cfg_rescale: number;
  noise_schedule: string;
  legacy_v3_extend: boolean;
  skip_cfg_above_sigma: null;
  use_coords: boolean;
  legacy_uc: boolean;
  normalize_reference_strength_multiple: boolean;
  inpaintImg2ImgStrength: number;
  seed: number;
  negative_prompt: string;
  deliberate_euler_ancestral_bug: boolean;
  prefer_brownian: boolean;
  [key: string]: unknown;  // Allow additional properties during migration
}

interface GenerationPayload {
  input: string;
  model: string;
  action: string;
  parameters: GenerationPayloadParameters;
  use_new_shared_trial: boolean;
}

export class NovelAIClient {
  private apiKey: string;
  private logger: Logger;

  // Retry configuration for rate limiting
  private readonly maxRetries = 3;
  private readonly baseRetryDelayMs = 1000;

  constructor(apiKey?: string, options?: { logger?: Logger }) {
    this.apiKey = apiKey || process.env.NOVELAI_API_KEY || "";
    this.logger = options?.logger ?? defaultLogger;
    if (!this.apiKey) {
      throw new Error(
        "API key is required. Set NOVELAI_API_KEY environment variable or pass apiKey parameter."
      );
    }
  }

  /**
   * ユーティリティ: 指定時間待機
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * ユーティリティ: リトライ付きでリクエストを実行
   * 429エラー (Too Many Requests / Concurrent generation locked) およびネットワークエラーに対応
   */
  private async fetchWithRetry(
    url: string,
    options: RequestInit,
    operationName: string = 'Request'
  ): Promise<Response> {
    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), Constants.DEFAULT_REQUEST_TIMEOUT_MS);

      let response: Response;
      try {
        response = await fetch(url, { ...options, signal: controller.signal });
      } catch (error) {
        clearTimeout(timeoutId);
        // Retry on network errors (timeout, connection refused, DNS failure, etc.)
        const isRetryable = error instanceof Error && (
          error.name === 'AbortError' ||
          error.name === 'TypeError' ||
          error.message.includes('ECONNREFUSED') ||
          error.message.includes('ENOTFOUND')
        );
        if (isRetryable && attempt < this.maxRetries) {
          const baseDelay = this.baseRetryDelayMs * Math.pow(2, attempt);
          const retryDelay = Math.round(baseDelay * (1 + Math.random() * 0.3));
          this.logger.warn(
            `[NovelAI] ${operationName}: Network error (${error instanceof Error ? error.message : 'Unknown'}). Retrying in ${retryDelay}ms... (attempt ${attempt + 1}/${this.maxRetries})`
          );
          await this.sleep(retryDelay);
          continue;
        }
        throw error;
      } finally {
        clearTimeout(timeoutId);
      }

      if (response.ok) {
        return response;
      }

      // Handle 429 (rate limit / concurrent lock)
      if (response.status === 429) {
        if (attempt < this.maxRetries) {
          const baseDelay = this.baseRetryDelayMs * Math.pow(2, attempt);
          const retryDelay = Math.round(baseDelay * (1 + Math.random() * 0.3));
          this.logger.warn(
            `[NovelAI] ${operationName}: Rate limited (429). Retrying in ${retryDelay}ms... (attempt ${attempt + 1}/${this.maxRetries})`
          );
          await this.sleep(retryDelay);
          continue;
        }
        // Max retries reached
        const text = await response.text();
        const sanitizedText = text.length > 200 ? text.slice(0, 200) + '...[truncated]' : text;
        this.logger.error(`[NovelAI] ${operationName} error after ${this.maxRetries} retries (${response.status}): ${sanitizedText}`);
        throw new Error(`${operationName} failed after ${this.maxRetries} retries: ${response.status} ${response.statusText}`);
      }

      // Other HTTP errors - don't retry
      const text = await response.text();
      const sanitizedText = text.length > 200 ? text.slice(0, 200) + '...[truncated]' : text;
      this.logger.error(`[NovelAI] ${operationName} error (${response.status}): ${sanitizedText}`);
      throw new Error(`${operationName} failed: ${response.status} ${response.statusText}`);
    }

    throw new Error(`${operationName} failed: Unknown error after ${this.maxRetries} retries`);
  }

  /**
   * 残りアンラス（Training Steps）を取得
   */
  async getAnlasBalance(): Promise<AnlasBalance> {
    const response = await this.fetchWithRetry(
      Constants.SUBSCRIPTION_URL,
      {
        method: "GET",
        headers: {
          "Authorization": `Bearer ${this.apiKey}`,
          "Accept": "application/json",
        },
      },
      'GetAnlasBalance'
    );

    const raw = await response.json();
    const data = Schemas.AnlasBalanceResponseSchema.parse(raw);
    const fixed = data.trainingStepsLeft.fixedTrainingStepsLeft;
    const purchased = data.trainingStepsLeft.purchasedTrainingSteps;

    return {
      fixed,
      purchased,
      total: fixed + purchased,
      tier: data.tier,
    };
  }

  /**
   * 画像をVibe Transfer用にエンコード（2 Anlas消費）
   */
  async encodeVibe(params: Schemas.EncodeVibeParams): Promise<Schemas.VibeEncodeResult> {
    // Validate parameters (use parseAsync for consistency with other methods)
    const validatedParams = await Schemas.EncodeVibeParamsSchema.parseAsync(params);

    // Get image data
    const imageBuffer = Utils.getImageBuffer(validatedParams.image);
    const b64Image = imageBuffer.toString('base64');

    // Calculate hash
    const sourceHash = crypto.createHash('sha256').update(imageBuffer).digest('hex');

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get initial Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const payload = {
      image: b64Image,
      information_extracted: validatedParams.information_extracted,
      model: validatedParams.model,
    };

    const response = await this.fetchWithRetry(
      Constants.ENCODE_URL,
      {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
          "Accept": "*/*",
        },
        body: JSON.stringify(payload),
      },
      'VibeEncode'
    );

    const responseBuffer = await this.getResponseBuffer(response);
    const encoding = responseBuffer.toString('base64');

    // Get final balance
    let anlasRemaining: number | null = null;
    let anlasConsumed: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasRemaining = balance.total;
      if (anlasBefore !== null) {
        anlasConsumed = anlasBefore - anlasRemaining;
      }
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get final Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const result: Schemas.VibeEncodeResult = {
      encoding,
      model: validatedParams.model,
      information_extracted: validatedParams.information_extracted,
      strength: validatedParams.strength,
      source_image_hash: sourceHash,
      created_at: new Date(),
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save if requested
    try {
      if (validatedParams.save_path) {
        await this.saveVibe(result, validatedParams.save_path);
        result.saved_path = validatedParams.save_path;
      } else if (validatedParams.save_dir) {
        const dir = validatedParams.save_dir;
        await this.ensureDir(dir);

        let filename: string;
        if (validatedParams.save_filename) {
          // Use custom filename, ensure .naiv4vibe extension
          const baseName = validatedParams.save_filename.replace(/\.naiv4vibe$/i, '');
          filename = `${baseName}.naiv4vibe`;
        } else {
          // Auto-generate filename
          const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
          filename = `${sourceHash.slice(0, 12)}_${timestamp}.naiv4vibe`;
        }
        const savePath = path.join(dir, filename);

        await this.saveVibe(result, savePath);
        result.saved_path = savePath;
      }
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to save vibe file:', e instanceof Error ? e.message : 'Unknown error');
    }

    return result;
  }

  /**
   * パストラバーサル防止: normalize後に".."が含まれていないか検証
   */
  private validateSavePath(savePath: string): string {
    const normalized = path.normalize(savePath);
    if (normalized.includes('..')) {
      throw new Error(`Invalid save path (path traversal detected): ${savePath}`);
    }
    return normalized;
  }

  private async ensureDir(dir: string): Promise<void> {
    await fsp.mkdir(dir, { recursive: true });
  }

  private async saveToFile(savePath: string, data: string | Buffer | Uint8Array): Promise<void> {
    savePath = this.validateSavePath(savePath);
    await this.ensureDir(path.dirname(savePath));
    await fsp.writeFile(savePath, data);
  }

  private async getResponseBuffer(response: Response, maxSize: number = Constants.MAX_RESPONSE_SIZE): Promise<Buffer> {
    const contentLength = response.headers.get('content-length');
    if (contentLength && parseInt(contentLength, 10) > maxSize) {
      throw new Error(`Response too large: ${contentLength} bytes (max ${maxSize})`);
    }
    const buffer = Buffer.from(await response.arrayBuffer());
    if (buffer.length > maxSize) {
      throw new Error(`Response too large: ${buffer.length} bytes (max ${maxSize})`);
    }
    return buffer;
  }

  private async saveVibe(result: Schemas.VibeEncodeResult, savePath: string) {
    const modelKey = Constants.MODEL_KEY_MAP[result.model] || "v4-5full";

    const vibeData = {
      identifier: "novelai-vibe-transfer",
      version: 1,
      type: "encoding",
      id: result.source_image_hash,
      encodings: {
        [modelKey]: {
          unknown: {
            encoding: result.encoding,
            params: {
              information_extracted: result.information_extracted,
            },
          },
        },
      },
      name: `${result.source_image_hash.slice(0, 6)}-${result.source_image_hash.slice(-6)}`,
      createdAt: result.created_at.toISOString(),
      importInfo: {
        model: result.model,
        information_extracted: result.information_extracted,
        strength: result.strength,
      },
    };

    await this.saveToFile(savePath, JSON.stringify(vibeData, null, 2));
  }

  // ===========================================================================
  // Generate Method - Private Helpers
  // ===========================================================================

  /**
   * Build the base payload structure for image generation
   */
  private buildBasePayload(
    validatedParams: Schemas.GenerateParams & { width: number; height: number; model: string },
    seed: number,
    negativePrompt: string
  ): GenerationPayload {
    return {
      input: validatedParams.prompt,
      model: validatedParams.model,
      action: validatedParams.action!,
      parameters: {
        params_version: 3,
        width: validatedParams.width,
        height: validatedParams.height,
        scale: validatedParams.scale!,
        sampler: validatedParams.sampler!,
        steps: validatedParams.steps!,
        n_samples: 1,
        ucPreset: 0,
        qualityToggle: true,
        autoSmea: false,
        dynamic_thresholding: false,
        controlnet_strength: 1,
        legacy: false,
        add_original_image: true,
        cfg_rescale: validatedParams.cfg_rescale!,
        noise_schedule: validatedParams.noise_schedule!,
        legacy_v3_extend: false,
        skip_cfg_above_sigma: null,
        use_coords: false,
        legacy_uc: false,
        normalize_reference_strength_multiple: true,
        inpaintImg2ImgStrength: 1,
        seed: seed,
        negative_prompt: negativePrompt,
        deliberate_euler_ancestral_bug: false,
        prefer_brownian: true,
      },
      use_new_shared_trial: true,
    };
  }

  /**
   * Apply Img2Img parameters to the payload
   */
  private applyImg2ImgParams(
    payload: GenerationPayload,
    validatedParams: Schemas.GenerateParams,
    seed: number
  ): void {
    if (validatedParams.action === "img2img" && validatedParams.source_image) {
      payload.parameters.image = Utils.getImageBase64(validatedParams.source_image);
      payload.parameters.strength = validatedParams.img2img_strength;
      payload.parameters.noise = validatedParams.img2img_noise;
      payload.parameters.extra_noise_seed = seed === 0 ? Constants.MAX_SEED : seed - 1;
    }
  }

  /**
   * Apply Infill/Inpaint parameters to the payload
   */
  private async applyInfillParams(
    payload: GenerationPayload,
    validatedParams: Schemas.GenerateParams & { width: number; height: number; model: string },
    seed: number
  ): Promise<void> {
    if (validatedParams.action !== "infill" || !validatedParams.source_image || !validatedParams.mask) {
      return;
    }

    // モデル名に-inpaintingサフィックスを追加（重複防止）
    if (!validatedParams.model.endsWith('-inpainting')) {
      payload.model = validatedParams.model + "-inpainting";
    }
    
    // 元画像を取得
    const sourceImageBuffer = Utils.getImageBuffer(validatedParams.source_image);
    const sourceImageBase64 = sourceImageBuffer.toString('base64');
    
    // マスク画像を処理（1/8サイズにリサイズ）
    const maskBuffer = Utils.getImageBuffer(validatedParams.mask);
    const resizedMask = await Utils.resizeMaskImage(
      maskBuffer,
      validatedParams.width,
      validatedParams.height
    );
    const maskBase64 = resizedMask.toString('base64');
    
    // cache_secret_keyを生成
    const imageCacheSecretKey = Utils.calculateCacheSecretKey(sourceImageBuffer);
    const maskCacheSecretKey = Utils.calculateCacheSecretKey(resizedMask);
    
    // パラメータ設定
    if (validatedParams.mask_strength == null) {
      throw new Error('mask_strength is required for infill action');
    }
    const maskStrength = validatedParams.mask_strength;
    const hybridStrength = validatedParams.hybrid_img2img_strength ?? maskStrength;
    const hybridNoise = validatedParams.hybrid_img2img_noise ?? 0;
    
    // Inpaint用パラメータを設定
    payload.parameters.image = sourceImageBase64;
    payload.parameters.mask = maskBase64;
    payload.parameters.strength = hybridStrength;
    payload.parameters.noise = hybridNoise;
    payload.parameters.add_original_image = false;
    payload.parameters.extra_noise_seed = seed === 0 ? Constants.MAX_SEED : seed - 1;
    payload.parameters.inpaintImg2ImgStrength = maskStrength;
    payload.parameters.img2img = {
      strength: maskStrength,
      color_correct: validatedParams.inpaint_color_correct,
    };
    payload.parameters.image_cache_secret_key = imageCacheSecretKey;
    payload.parameters.mask_cache_secret_key = maskCacheSecretKey;
    payload.parameters.image_format = "png";
    payload.parameters.stream = "msgpack";
  }

  /**
   * Apply Vibe Transfer parameters to the payload
   */
  private applyVibeParams(
    payload: GenerationPayload,
    vibeEncodings: string[],
    vibeStrengths: number[] | null | undefined,
    vibeInfoList: number[]
  ): void {
    if (vibeEncodings.length > 0) {
      payload.parameters.reference_image_multiple = vibeEncodings;
      payload.parameters.reference_strength_multiple = vibeStrengths;
      payload.parameters.reference_information_extracted_multiple = vibeInfoList;
      payload.parameters.normalize_reference_strength_multiple = true;
    }
  }

  /**
   * Apply Character Reference parameters to the payload
   */
  private applyCharRefParams(
    payload: GenerationPayload,
    charRefData: Awaited<ReturnType<typeof Utils.processCharacterReferences>>
  ): void {
    const { images, descriptions, info_extracted, strength_values, secondary_strength_values } = charRefData;
    payload.parameters.director_reference_images = images;
    payload.parameters.director_reference_descriptions = descriptions;
    payload.parameters.director_reference_information_extracted = info_extracted;
    payload.parameters.director_reference_strength_values = strength_values;
    payload.parameters.director_reference_secondary_strength_values = secondary_strength_values;
    payload.parameters.stream = "msgpack";
    payload.parameters.image_format = "png";
  }

  /**
   * Build V4 prompt structure for the payload
   */
  private buildV4PromptStructure(
    payload: GenerationPayload,
    prompt: string,
    negativePrompt: string,
    charCaptions: ReturnType<typeof Schemas.characterToCaptionDict>[],
    charNegativeCaptions: ReturnType<typeof Schemas.characterToNegativeCaptionDict>[]
  ): void {
    payload.parameters.v4_prompt = {
      caption: {
        base_caption: prompt,
        char_captions: charCaptions,
      },
      use_coords: true,
      use_order: true,
    };
    payload.parameters.v4_negative_prompt = {
      caption: {
        base_caption: negativePrompt,
        char_captions: charNegativeCaptions,
      },
      legacy_uc: false,
    };
  }

  /**
   * Apply character prompts (use_coords) to the payload
   */
  private applyCharacterPrompts(
    payload: GenerationPayload,
    charConfigs: Schemas.CharacterConfig[]
  ): void {
    if (charConfigs.length > 0) {
      payload.parameters.use_coords = true;
      payload.parameters.characterPrompts = charConfigs.map(char => ({
        prompt: char.prompt,
        uc: char.negative_prompt,
        center: { x: char.center_x, y: char.center_y },
        enabled: true,
      }));
    }
  }

  // ===========================================================================
  // Generate Method - Main Implementation
  // ===========================================================================

  /**
   * 統合画像生成メソッド
   */
  async generate(params: Schemas.GenerateParams): Promise<Schemas.GenerateResult> {
    // Validate parameters
    const validatedParams = await Schemas.GenerateParamsSchema.parseAsync(params);

    // Defaults
    const negativePrompt = validatedParams.negative_prompt ?? Constants.DEFAULT_NEGATIVE;
    const seed = validatedParams.seed ?? Math.floor(Math.random() * Constants.MAX_SEED);

    // Process Character Reference
    type CharRefProcessResult = Awaited<ReturnType<typeof Utils.processCharacterReferences>>;
    let charRefData: CharRefProcessResult | null = null;
    if (validatedParams.character_reference) {
      charRefData = await Utils.processCharacterReferences([validatedParams.character_reference]);
    }

    // Process Vibes
    let vibeEncodings: string[] = [];
    let vibeInfoList: number[] = [];
    let vibeStrengths = validatedParams.vibe_strengths;

    if (validatedParams.vibes && validatedParams.vibes.length > 0) {
      const processed = await Utils.processVibes(validatedParams.vibes, validatedParams.model);
      vibeEncodings = processed.encodings;
      vibeInfoList = validatedParams.vibe_info_extracted || processed.info_extracted_list;

      if (!vibeStrengths) {
        vibeStrengths = new Array(vibeEncodings.length).fill(Constants.DEFAULT_VIBE_STRENGTH);
      }
    }

    // Character Configs
    type CharCaptionDict = ReturnType<typeof Schemas.characterToCaptionDict>;
    type CharNegativeCaptionDict = ReturnType<typeof Schemas.characterToNegativeCaptionDict>;
    let charConfigs = validatedParams.characters || [];
    let charCaptions: CharCaptionDict[] = [];
    let charNegativeCaptions: CharNegativeCaptionDict[] = [];

    if (charConfigs.length > 0) {
      charCaptions = charConfigs.map(Schemas.characterToCaptionDict);
      charNegativeCaptions = charConfigs.map(Schemas.characterToNegativeCaptionDict);
    }

    // Build payload using helper methods
    const payload = this.buildBasePayload(validatedParams, seed, negativePrompt);
    
    // Apply action-specific parameters
    this.applyImg2ImgParams(payload, validatedParams, seed);
    await this.applyInfillParams(payload, validatedParams, seed);
    
    // Apply additional features
    this.applyVibeParams(payload, vibeEncodings, vibeStrengths, vibeInfoList);
    
    if (charRefData) {
      this.applyCharRefParams(payload, charRefData);
    }

    // Build prompt structures
    this.buildV4PromptStructure(payload, validatedParams.prompt, negativePrompt, charCaptions, charNegativeCaptions);
    this.applyCharacterPrompts(payload, charConfigs);

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to get initial Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    // Make Request
    const useStream = (validatedParams.character_reference !== undefined && validatedParams.character_reference !== null) 
      || validatedParams.action === "infill";
    const apiUrl = useStream ? Constants.STREAM_URL : Constants.API_URL;

    const response = await this.fetchWithRetry(
      apiUrl,
      {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      },
      'Generation'
    );

    const responseBuffer = await this.getResponseBuffer(response);
    const imageData = useStream ? this.parseStreamResponse(responseBuffer) : this.parseZipResponse(responseBuffer);

    // Get final balance
    let anlasRemaining: number | null = null;
    let anlasConsumed: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasRemaining = balance.total;
      if (anlasBefore !== null) {
        anlasConsumed = anlasBefore - anlasRemaining;
      }
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to get final Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const result: Schemas.GenerateResult = {
      image_data: imageData,
      seed: seed,
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save
    try {
      if (validatedParams.save_path) {
        await this.saveImage(result, validatedParams.save_path);
        result.saved_path = validatedParams.save_path;
      } else if (validatedParams.save_dir) {
        const dir = validatedParams.save_dir;
        await this.ensureDir(dir);

        let prefix = validatedParams.action === "img2img" ? "img2img" : "gen";
        if (charConfigs.length > 0) prefix += "_multi";

        const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
        const filename = `${prefix}_${timestamp}_${seed}.png`;
        const savePath = path.join(dir, filename);

        await this.saveImage(result, savePath);
        result.saved_path = savePath;
      }
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to save image:', e instanceof Error ? e.message : 'Unknown error');
    }

    return result;
  }

  private async saveImage(result: { image_data: Buffer | Uint8Array }, savePath: string) {
      await this.saveToFile(savePath, result.image_data);
  }

  private parseZipResponse(content: Buffer): Buffer {
    const zip = new AdmZip(content);
    const zipEntries = zip.getEntries();

    if (zipEntries.length > Constants.MAX_ZIP_ENTRIES) {
      throw new Error(`Too many ZIP entries: ${zipEntries.length} (max ${Constants.MAX_ZIP_ENTRIES})`);
    }

    for (const entry of zipEntries) {
      if (entry.entryName.match(/\.(png|webp|jpg|jpeg)$/i)) {
        if (entry.header.size > Constants.MAX_DECOMPRESSED_IMAGE_SIZE) {
          throw new Error(`Decompressed image too large (${entry.header.size} bytes, max ${Constants.MAX_DECOMPRESSED_IMAGE_SIZE})`);
        }
        if (entry.header.compressedSize > 0 && entry.header.size / entry.header.compressedSize > Constants.MAX_COMPRESSION_RATIO) {
          throw new Error(`Suspicious compression ratio detected`);
        }
        return entry.getData();
      }
    }
    throw new Error("No image found in response ZIP");
  }

  private parseStreamResponse(content: Buffer): Buffer {
    // Check for ZIP signature (PK)
    if (content.length > 1 && content[0] === 0x50 && content[1] === 0x4b) {
        return this.parseZipResponse(content);
    }

    // Check for PNG signature
    const pngSignature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    if (content.length > 8 && content.subarray(0, 8).equals(pngSignature)) {
        return content;
    }

    // msgpack stream parsing
    try {
        const unpackr = new Unpackr({ useRecords: false });
        const values = unpackr.unpackMultiple(content);

        for (const val of values) {
             if (val && typeof val === 'object') {
                 if (val['data']) return val['data'];
                 if (val['image']) return val['image'];
             }
        }
    } catch (e) {
        this.logger.warn('[NovelAI] msgpack parse failed, falling back to PNG detection:', e instanceof Error ? e.message : 'Unknown error');
    }

    // Fallback: search for PNG magic bytes
    const pngStart = content.indexOf(pngSignature);
    if (pngStart !== -1) {
        // Search for IEND chunk to get exact PNG end
        const iendMarker = Buffer.from([0x49, 0x45, 0x4e, 0x44]);
        const iendPos = content.indexOf(iendMarker, pngStart);
        if (iendPos !== -1) {
            // IEND chunk: 4 bytes length + 4 bytes "IEND" + 4 bytes CRC
            return content.subarray(pngStart, iendPos + 8);
        }
        return content.subarray(pngStart);
    }

    throw new Error(`Cannot parse stream response (length: ${content.length})`);
  }

  /**
   * 画像加工ツール（カラー化、表情変換、スケッチ化など）
   * @param params Augment parameters
   * @returns Augmented image result
   */
  async augmentImage(params: Schemas.AugmentParams): Promise<Schemas.AugmentResult> {
    // Validate parameters (use parseAsync for consistency)
    const validatedParams = await Schemas.AugmentParamsSchema.parseAsync(params);

    // Get image data and auto-detect dimensions
    const { width, height, buffer: imageBuffer } = await Utils.getImageDimensions(validatedParams.image);
    const b64Image = imageBuffer.toString('base64');

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get initial Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    // Build payload with auto-detected dimensions
    const payload: any = {
      req_type: validatedParams.req_type,
      use_new_shared_trial: true,
      width: width,
      height: height,
      image: b64Image,
    };

    // Add prompt and defry only for colorize and emotion
    if (validatedParams.req_type === 'colorize') {
      // colorize: prompt はそのまま使用（オプション）
      if (validatedParams.prompt) {
        payload.prompt = validatedParams.prompt;
      }
      payload.defry = validatedParams.defry;
    } else if (validatedParams.req_type === 'emotion') {
      // emotion: prompt に ;; を自動付与
      if (validatedParams.prompt) {
        payload.prompt = `${validatedParams.prompt};;`;
      }
      payload.defry = validatedParams.defry;
    }

    const response = await this.fetchWithRetry(
      Constants.AUGMENT_URL,
      {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      },
      'Augment'
    );

    const responseBuffer = await this.getResponseBuffer(response);
    const imageData = this.parseZipResponse(responseBuffer);

    // Get final balance
    let anlasRemaining: number | null = null;
    let anlasConsumed: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasRemaining = balance.total;
      if (anlasBefore !== null) {
        anlasConsumed = anlasBefore - anlasRemaining;
      }
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get final Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const result: Schemas.AugmentResult = {
      image_data: imageData,
      req_type: validatedParams.req_type,
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save if requested
    try {
      if (validatedParams.save_path) {
        await this.saveImage(result, validatedParams.save_path);
        result.saved_path = validatedParams.save_path;
      } else if (validatedParams.save_dir) {
        const dir = validatedParams.save_dir;
        await this.ensureDir(dir);

        const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
        const rand = crypto.randomBytes(2).toString('hex');
        const filename = `${validatedParams.req_type}_${timestamp}_${rand}.png`;
        const savePath = path.join(dir, filename);

        await this.saveImage(result, savePath);
        result.saved_path = savePath;
      }
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to save augmented image:', e instanceof Error ? e.message : 'Unknown error');
    }

    return result;
  }

  /**
   * 画像アップスケール（拡大）
   * @param params Upscale parameters
   * @returns Upscaled image result
   */
  async upscaleImage(params: Schemas.UpscaleParams): Promise<Schemas.UpscaleResult> {
    // Validate parameters (use parseAsync for consistency)
    const validatedParams = await Schemas.UpscaleParamsSchema.parseAsync(params);

    // Get image data and auto-detect dimensions
    const { width, height, buffer: imageBuffer } = await Utils.getImageDimensions(validatedParams.image);
    const b64Image = imageBuffer.toString('base64');

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get initial Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const payload = {
      image: b64Image,
      width: width,
      height: height,
      scale: validatedParams.scale,
    };

    const response = await this.fetchWithRetry(
      Constants.UPSCALE_URL,
      {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      },
      'Upscale'
    );

    // Response can be ZIP or raw image
    const responseBuffer = await this.getResponseBuffer(response);
    let imageData: Buffer;

    // Check for ZIP signature (PK)
    if (responseBuffer.length > 1 && responseBuffer[0] === 0x50 && responseBuffer[1] === 0x4b) {
      imageData = this.parseZipResponse(responseBuffer);
    } else {
      imageData = responseBuffer;
    }

    // Get final balance
    let anlasRemaining: number | null = null;
    let anlasConsumed: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasRemaining = balance.total;
      if (anlasBefore !== null) {
        anlasConsumed = anlasBefore - anlasRemaining;
      }
    } catch (e) {
      // Log but continue - Anlas tracking is optional
      this.logger.warn('[NovelAI] Failed to get final Anlas balance:', e instanceof Error ? e.message : 'Unknown error');
    }

    const outputWidth = width * validatedParams.scale;
    const outputHeight = height * validatedParams.scale;

    const result: Schemas.UpscaleResult = {
      image_data: imageData,
      scale: validatedParams.scale,
      output_width: outputWidth,
      output_height: outputHeight,
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save if requested
    try {
      if (validatedParams.save_path) {
        await this.saveImage(result, validatedParams.save_path);
        result.saved_path = validatedParams.save_path;
      } else if (validatedParams.save_dir) {
        const dir = validatedParams.save_dir;
        await this.ensureDir(dir);

        const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
        const rand = crypto.randomBytes(2).toString('hex');
        const filename = `upscale_${validatedParams.scale}x_${timestamp}_${rand}.png`;
        const savePath = path.join(dir, filename);

        await this.saveImage(result, savePath);
        result.saved_path = savePath;
      }
    } catch (e) {
      this.logger.warn('[NovelAI] Failed to save upscaled image:', e instanceof Error ? e.message : 'Unknown error');
    }

    return result;
  }
}
