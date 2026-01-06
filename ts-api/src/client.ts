/**
 * NovelAI Unified Image Generation Client
 * 統合されたVibe Transfer & Image2Image APIクライアント
 */
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import AdmZip from 'adm-zip';
import { Unpackr } from 'msgpackr';
import * as Constants from './constants';
import * as Schemas from './schemas';
import * as Utils from './utils';

// Helper types for return values
export interface AnlasBalance {
  fixed: number;
  purchased: number;
  total: number;
  tier: number;
}

export class NovelAIClient {
  private apiKey: string;

  constructor(apiKey?: string) {
    this.apiKey = apiKey || process.env.NOVELAI_API_KEY || "";
    if (!this.apiKey) {
      throw new Error(
        "API key is required. Set NOVELAI_API_KEY environment variable or pass apiKey parameter."
      );
    }
  }

  /**
   * 残りアンラス（Training Steps）を取得
   */
  async getAnlasBalance(): Promise<AnlasBalance> {
    const response = await fetch(Constants.SUBSCRIPTION_URL, {
      method: "GET",
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "Accept": "application/json",
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to get subscription data: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    const trainingSteps = data.trainingStepsLeft || {};
    const fixed = trainingSteps.fixedTrainingStepsLeft || 0;
    const purchased = trainingSteps.purchasedTrainingSteps || 0;

    return {
      fixed,
      purchased,
      total: fixed + purchased,
      tier: data.tier || 0,
    };
  }

  /**
   * 画像をVibe Transfer用にエンコード（2 Anlas消費）
   */
  async encodeVibe(params: Schemas.EncodeVibeParams): Promise<Schemas.VibeEncodeResult> {
    // Validate parameters
    const validatedParams = Schemas.EncodeVibeParamsSchema.parse(params);

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
      // Ignore error
    }

    const payload = {
      image: b64Image,
      information_extracted: validatedParams.information_extracted,
      model: validatedParams.model,
    };

    const response = await fetch(Constants.ENCODE_URL, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
        "Accept": "*/*",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      throw new Error(`Vibe encoding failed: ${response.status} ${response.statusText}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    const encoding = Buffer.from(arrayBuffer).toString('base64');

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
      // Ignore
    }

    const result: Schemas.VibeEncodeResult = {
      encoding,
      model: validatedParams.model as any,
      information_extracted: validatedParams.information_extracted,
      strength: validatedParams.strength,
      source_image_hash: sourceHash,
      created_at: new Date(),
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save if requested
    if (validatedParams.save_path) {
      this.saveVibe(result, validatedParams.save_path);
      result.saved_path = validatedParams.save_path;
    } else if (validatedParams.save_dir) {
      const dir = validatedParams.save_dir;
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      let filename: string;
      if (validatedParams.save_filename) {
        // Use custom filename, ensure .naiv4vibe extension
        const baseName = validatedParams.save_filename.replace(/\.naiv4vibe$/i, '');
        filename = `${baseName}.naiv4vibe`;
      } else {
        // Auto-generate filename
        const timestamp = new Date().toISOString().replace(/[-:T.]/g, '_').slice(0, 15);
        filename = `${sourceHash.slice(0, 12)}_${timestamp}.naiv4vibe`;
      }
      const savePath = path.join(dir, filename);

      this.saveVibe(result, savePath);
      result.saved_path = savePath;
    }

    return result;
  }

  private saveVibe(result: Schemas.VibeEncodeResult, savePath: string) {
    const dir = path.dirname(savePath);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

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

    fs.writeFileSync(savePath, JSON.stringify(vibeData, null, 2), 'utf-8');
  }

  /**
   * 統合画像生成メソッド
   */
  async generate(params: Schemas.GenerateParams): Promise<Schemas.GenerateResult> {
    // Validate parameters
    const validatedParams = Schemas.GenerateParamsSchema.parse(params);

    // Defaults
    const negativePrompt = validatedParams.negative_prompt ?? Constants.DEFAULT_NEGATIVE;
    const seed = validatedParams.seed ?? Math.floor(Math.random() * 4294967295);

    // Process Character Reference
    let charRefData: any = null;
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
    let charConfigs = validatedParams.characters || [];
    let charCaptions: any[] = [];
    let charNegativeCaptions: any[] = [];

    if (charConfigs.length > 0) {
      charCaptions = charConfigs.map(Schemas.characterToCaptionDict);
      charNegativeCaptions = charConfigs.map(Schemas.characterToNegativeCaptionDict);
    }

    // Build Payload
    const payload: any = {
      input: validatedParams.prompt,
      model: validatedParams.model,
      action: validatedParams.action,
      parameters: {
        params_version: 3,
        width: validatedParams.width,
        height: validatedParams.height,
        scale: validatedParams.scale,
        sampler: validatedParams.sampler,
        steps: validatedParams.steps,
        n_samples: 1,
        ucPreset: 0,
        qualityToggle: true,
        autoSmea: false,
        dynamic_thresholding: false,
        controlnet_strength: 1,
        legacy: false,
        add_original_image: true,
        cfg_rescale: 0,
        noise_schedule: validatedParams.noise_schedule,
        legacy_v3_extend: false,
        skip_cfg_above_sigma: null,
        use_coords: true,
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

    // Img2Img
    if (validatedParams.action === "img2img" && validatedParams.source_image) {
      payload.parameters.image = Utils.getImageBase64(validatedParams.source_image);
      payload.parameters.strength = validatedParams.img2img_strength;
      payload.parameters.noise = validatedParams.img2img_noise;
      payload.parameters.extra_noise_seed = seed - 1;
    }

    // Infill/Inpaint (Mask機能)
    if (validatedParams.action === "infill" && validatedParams.source_image && validatedParams.mask) {
      // モデル名に-inpaintingサフィックスを追加
      payload.model = validatedParams.model + "-inpainting";
      
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
      
      // Inpaint用パラメータを設定
      payload.parameters.image = sourceImageBase64;
      payload.parameters.mask = maskBase64;
      payload.parameters.strength = validatedParams.inpaint_strength;
      payload.parameters.noise = validatedParams.inpaint_noise;
      payload.parameters.add_original_image = false;
      payload.parameters.extra_noise_seed = seed - 1;
      payload.parameters.img2img = {
        strength: validatedParams.inpaint_strength,
        color_correct: validatedParams.inpaint_color_correct,
      };
      payload.parameters.image_cache_secret_key = imageCacheSecretKey;
      payload.parameters.mask_cache_secret_key = maskCacheSecretKey;
      payload.parameters.image_format = "png";
      payload.parameters.stream = "msgpack";
    }

    // Vibe
    if (vibeEncodings.length > 0) {
      payload.parameters.reference_image_multiple = vibeEncodings;
      payload.parameters.reference_strength_multiple = vibeStrengths;
      payload.parameters.reference_information_extracted_multiple = vibeInfoList;
      payload.parameters.normalize_reference_strength_multiple = true;
    }

    // Character Reference
    if (charRefData) {
      const { images, descriptions, info_extracted, strength_values, secondary_strength_values } = charRefData;
      payload.parameters.director_reference_images = images;
      payload.parameters.director_reference_descriptions = descriptions;
      payload.parameters.director_reference_information_extracted = info_extracted;
      payload.parameters.director_reference_strength_values = strength_values;
      payload.parameters.director_reference_secondary_strength_values = secondary_strength_values;
      payload.parameters.use_coords = true;
      payload.parameters.stream = "msgpack";
      payload.parameters.image_format = "png";

      // Ensure character prompts exist if using reference
      if (charConfigs.length === 0) {
        const dummyChar: Schemas.CharacterConfig = { prompt: validatedParams.prompt, center_x: 0.5, center_y: 0.5, negative_prompt: "" };
        charConfigs = [dummyChar];
        charCaptions = [Schemas.characterToCaptionDict(dummyChar)];
        charNegativeCaptions = [Schemas.characterToNegativeCaptionDict(dummyChar)];
      }
    }

    // V4 Prompt Structure
    payload.parameters.v4_prompt = {
      caption: {
        base_caption: validatedParams.prompt,
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

    // Character Prompts (use_coords)
    if (charConfigs.length > 0) {
      payload.parameters.use_coords = true;
      payload.parameters.characterPrompts = charConfigs.map(char => ({
        prompt: char.prompt,
        uc: char.negative_prompt,
        center: { x: char.center_x, y: char.center_y },
        enabled: true,
      }));
    }

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Ignore
    }

    // Make Request
    // Use stream endpoint for character reference OR infill action
    const useStream = (validatedParams.character_reference !== undefined && validatedParams.character_reference !== null) 
      || validatedParams.action === "infill";
    const apiUrl = useStream ? Constants.STREAM_URL : Constants.API_URL;

    const response = await fetch(apiUrl, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
        const text = await response.text();
        console.error(`Error response: ${text}`);
        throw new Error(`Generation failed: ${response.status} ${response.statusText}`);
    }

    const responseBuffer = Buffer.from(await response.arrayBuffer());
    let imageData: Buffer;

    if (useStream) {
        imageData = this.parseStreamResponse(responseBuffer);
    } else {
        imageData = this.parseZipResponse(responseBuffer);
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
      // Ignore
    }

    const result: Schemas.GenerateResult = {
      image_data: imageData,
      seed: seed,
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save
    if (validatedParams.save_path) {
      this.saveImage(result, validatedParams.save_path);
      result.saved_path = validatedParams.save_path;
    } else if (validatedParams.save_dir) {
      const dir = validatedParams.save_dir;
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

      let prefix = validatedParams.action === "img2img" ? "img2img" : "gen";
      if (charConfigs.length > 0) prefix += "_multi";

      const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
      const filename = `${prefix}_${timestamp}_${seed}.png`;
      const savePath = path.join(dir, filename);

      this.saveImage(result, savePath);
      result.saved_path = savePath;
    }

    return result;
  }

  private saveImage(result: { image_data: Buffer }, savePath: string) {
      const dir = path.dirname(savePath);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(savePath, result.image_data);
  }

  private parseZipResponse(content: Buffer): Buffer {
    const zip = new AdmZip(content);
    const zipEntries = zip.getEntries();

    for (const entry of zipEntries) {
      if (entry.entryName.match(/\.(png|webp|jpg|jpeg)$/i)) {
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
        // ignore and fallback
    }

    // Fallback: search for PNG magic bytes
    const pngStart = content.indexOf(pngSignature);
    if (pngStart !== -1) {
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
    // Validate parameters
    const validatedParams = Schemas.AugmentParamsSchema.parse(params);

    // Get image data and auto-detect dimensions
    const { width, height, buffer: imageBuffer } = await Utils.getImageDimensions(validatedParams.image);
    const b64Image = imageBuffer.toString('base64');

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Ignore error
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

    const response = await fetch(Constants.AUGMENT_URL, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const text = await response.text();
      console.error(`Augment error response: ${text}`);
      throw new Error(`Augment failed: ${response.status} ${response.statusText}`);
    }

    const responseBuffer = Buffer.from(await response.arrayBuffer());
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
      // Ignore
    }

    const result: Schemas.AugmentResult = {
      image_data: imageData,
      req_type: validatedParams.req_type,
      anlas_remaining: anlasRemaining,
      anlas_consumed: anlasConsumed,
      saved_path: null,
    };

    // Save if requested
    if (validatedParams.save_path) {
      this.saveImage(result, validatedParams.save_path);
      result.saved_path = validatedParams.save_path;
    } else if (validatedParams.save_dir) {
      const dir = validatedParams.save_dir;
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

      const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
      const filename = `${validatedParams.req_type}_${timestamp}.png`;
      const savePath = path.join(dir, filename);

      this.saveImage(result, savePath);
      result.saved_path = savePath;
    }

    return result;
  }

  /**
   * 画像アップスケール（拡大）
   * @param params Upscale parameters
   * @returns Upscaled image result
   */
  async upscaleImage(params: Schemas.UpscaleParams): Promise<Schemas.UpscaleResult> {
    // Validate parameters
    const validatedParams = Schemas.UpscaleParamsSchema.parse(params);

    // Get image data and auto-detect dimensions
    const { width, height, buffer: imageBuffer } = await Utils.getImageDimensions(validatedParams.image);
    const b64Image = imageBuffer.toString('base64');

    // Get initial balance
    let anlasBefore: number | null = null;
    try {
      const balance = await this.getAnlasBalance();
      anlasBefore = balance.total;
    } catch (e) {
      // Ignore error
    }

    const payload = {
      image: b64Image,
      width: width,
      height: height,
      scale: validatedParams.scale,
    };

    const response = await fetch(Constants.UPSCALE_URL, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const text = await response.text();
      console.error(`Upscale error response: ${text}`);
      throw new Error(`Upscale failed: ${response.status} ${response.statusText}`);
    }

    // Response can be ZIP or raw image
    const responseBuffer = Buffer.from(await response.arrayBuffer());
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
      // Ignore
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
    if (validatedParams.save_path) {
      this.saveImage(result, validatedParams.save_path);
      result.saved_path = validatedParams.save_path;
    } else if (validatedParams.save_dir) {
      const dir = validatedParams.save_dir;
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

      const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 15);
      const filename = `upscale_${validatedParams.scale}x_${timestamp}.png`;
      const savePath = path.join(dir, filename);

      this.saveImage(result, savePath);
      result.saved_path = savePath;
    }

    return result;
  }
}
