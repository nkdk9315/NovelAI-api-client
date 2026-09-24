import Foundation

// MARK: - Payload Construction Helpers
//
// Internal functions for building JSON payloads to send to the NovelAI API.
// Uses [String: Any] dictionaries for flexible conditional field inclusion,
// serialized via JSONSerialization.

// MARK: - Base Payload

/// Build the base payload structure for image generation.
///
/// Constructs the top-level payload with `input`, `model`, `action`, and the
/// `parameters` dictionary containing all standard generation settings.
func buildBasePayload(
    _ params: GenerateParams,
    seed: UInt32,
    negativePrompt: String
) -> [String: Any] {
    let extraNoiseSeed = Int(seed == 0 ? MAX_SEED : seed - 1)
    let v5 = params.model.isV5
    var parameters: [String: Any] = [
        // V5: params_version 4 (same as the official site)
        "params_version": v5 ? 4 : 3,
        "width": params.width,
        "height": params.height,
        "scale": params.scale,
        "sampler": params.sampler.rawValue,
        "steps": params.steps,
        "n_samples": 1,
        "ucPreset": 2,
        "qualityToggle": true,
        "autoSmea": false,
        "dynamic_thresholding": false,
        "controlnet_strength": 1,
        "legacy": false,
        "add_original_image": true,
        "cfg_rescale": params.cfgRescale,
        // V5: noise_schedule is always karras on the official site
        "noise_schedule": v5 ? NoiseSchedule.karras.rawValue : params.noiseSchedule.rawValue,
        "legacy_v3_extend": false,
        "skip_cfg_above_sigma": NSNull(), // Explicit JSON null — API expects this key present with null value
        "use_coords": true,
        "legacy_uc": false,
        "normalize_reference_strength_multiple": true,
        "inpaintImg2ImgStrength": 0.85,
        "seed": Int(seed),
        "extra_noise_seed": extraNoiseSeed,
        "negative_prompt": negativePrompt,
        "deliberate_euler_ancestral_bug": false,
        "prefer_brownian": true,
        "stream": "msgpack",
        "image_format": "png",
    ]
    if params.transparentBackground {
        // Transparency itself comes from the prompt tag; these are the site's hints
        parameters["straight_alpha"] = true
        parameters["tag_hint_transparent_background"] = true
    }

    return [
        "input": params.effectivePrompt,
        "model": params.model.rawValue,
        "action": params.action.rawValue,
        "parameters": parameters,
        "use_new_shared_trial": true,
    ]
}

// MARK: - Img2Img Parameters

/// Apply img2img-specific parameters to the payload.
///
/// When action is `img2img` and a source image is provided, adds the base64
/// image, strength, noise, and extra_noise_seed fields.
func applyImg2ImgParams(
    _ payload: inout [String: Any],
    params: GenerateParams
) throws {
    guard params.action == .img2img, let sourceImage = params.sourceImage else {
        return
    }

    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    // Resize source image to match output dimensions to avoid server errors with oversized images
    let imageBase64 = try resizeImageForImg2Img(sourceImage, targetWidth: params.width, targetHeight: params.height)
    parameters["image"] = imageBase64
    parameters["strength"] = params.img2imgStrength
    parameters["noise"] = params.img2imgNoise

    payload["parameters"] = parameters
}

// MARK: - Infill/Inpaint Parameters

/// Apply infill (inpaint) parameters to the payload.
///
/// Handles model name suffixing, mask resizing, cache key generation,
/// and all inpaint-specific parameter fields.
func applyInfillParams(
    _ payload: inout [String: Any],
    params: GenerateParams
) throws {
    guard params.action == .infill,
          let sourceImage = params.sourceImage,
          params.mask != nil else {
        return
    }

    // Switch to the inpainting model (V5 curated uses the 4.5 curated one)
    payload["model"] = params.model.inpaintModel

    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    // Resize source image to target dimensions (same as img2img)
    let sourceImageBase64 = try resizeImageForImg2Img(
        sourceImage, targetWidth: params.width, targetHeight: params.height
    )

    // Resize mask to 1/8 dimensions
    let maskBuffer = try getImageBuffer(params.mask!)
    let resizedMask = try resizeMaskImage(maskBuffer, targetWidth: params.width, targetHeight: params.height)
    let maskBase64 = resizedMask.base64EncodedString()

    // Validate mask_strength is present
    guard let maskStrength = params.maskStrength else {
        throw NovelAIError.validation("mask_strength is required for infill action")
    }

    let hybridStrength = params.hybridImg2imgStrength ?? maskStrength
    let hybridNoise = params.hybridImg2imgNoise ?? 0

    // Set inpaint parameters
    parameters["image"] = sourceImageBase64
    parameters["mask"] = maskBase64
    parameters["strength"] = hybridStrength
    parameters["noise"] = hybridNoise
    parameters["add_original_image"] = false
    parameters["inpaintImg2ImgStrength"] = maskStrength
    parameters["img2img"] = [
        "strength": maskStrength,
        "color_correct": params.inpaintColorCorrect,
    ] as [String: Any]

    payload["parameters"] = parameters
}

// MARK: - Vibe Transfer Parameters

/// Apply vibe transfer parameters to the payload.
///
/// Adds reference image encodings, strength values, and information extracted
/// values when vibes are present.
func applyVibeParams(
    _ payload: inout [String: Any],
    vibeEncodings: [String],
    vibeStrengths: [Double]?,
    vibeInfoList: [Double]
) {
    guard !vibeEncodings.isEmpty else {
        return
    }

    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    parameters["reference_image_multiple"] = vibeEncodings
    parameters["reference_strength_multiple"] = vibeStrengths
    parameters["reference_information_extracted_multiple"] = vibeInfoList
    parameters["normalize_reference_strength_multiple"] = true

    payload["parameters"] = parameters
}

// MARK: - Character Reference Parameters

/// Apply character reference parameters to the payload.
///
/// Adds director reference images, descriptions, strength values, and
/// streaming configuration for character reference generation.
func applyCharRefParams(
    _ payload: inout [String: Any],
    charRefs: ProcessedCharacterReferences
) {
    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    parameters["director_reference_images"] = charRefs.images
    parameters["director_reference_descriptions"] = charRefs.descriptions.map { $0.toDictionary() }
    parameters["director_reference_information_extracted"] = charRefs.infoExtracted
    parameters["director_reference_strength_values"] = charRefs.strengthValues
    parameters["director_reference_secondary_strength_values"] = charRefs.secondaryStrengthValues

    payload["parameters"] = parameters
}

// MARK: - V4 Prompt Structure

/// Build the v4_prompt structure for the payload.
///
/// Constructs the nested caption dictionary with base_caption and char_captions,
/// along with use_coords and use_order flags.
func buildV4PromptStructure(
    prompt: String,
    charCaptions: [[String: Any]]
) -> [String: Any] {
    return [
        "caption": [
            "base_caption": prompt,
            "char_captions": charCaptions,
        ] as [String: Any],
        "use_coords": true,
        "use_order": true,
    ]
}

/// Build the v4_negative_prompt structure for the payload.
///
/// Constructs the nested caption dictionary for negative prompts with
/// base_caption, char_captions, and legacy_uc flag.
func buildV4NegativePromptStructure(
    negativePrompt: String,
    charNegativeCaptions: [[String: Any]]
) -> [String: Any] {
    return [
        "caption": [
            "base_caption": negativePrompt,
            "char_captions": charNegativeCaptions,
        ] as [String: Any],
        "legacy_uc": false,
    ]
}

// MARK: - Character Prompts

/// Apply character-specific prompts (use_coords) to the payload.
///
/// When characters are configured, enables coordinate mode and adds the
/// characterPrompts array with prompt, negative prompt, center position,
/// and enabled flag for each character.
func applyCharacterPrompts(
    _ payload: inout [String: Any],
    params: GenerateParams
) {
    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    let characters = params.characters ?? []

    if !characters.isEmpty {
        parameters["use_coords"] = true
    }
    parameters["characterPrompts"] = characters.map { char -> [String: Any] in
        [
            "prompt": char.prompt,
            "uc": char.negativePrompt,
            "center": ["x": char.centerX, "y": char.centerY] as [String: Double],
            "enabled": true,
        ]
    }

    payload["parameters"] = parameters
}

// MARK: - Full V4 Prompt Application

/// Apply the complete V4 prompt and negative prompt structures to the payload.
///
/// Combines buildV4PromptStructure and buildV4NegativePromptStructure,
/// setting both v4_prompt and v4_negative_prompt in the parameters.
func applyV4PromptStructures(
    _ payload: inout [String: Any],
    prompt: String,
    negativePrompt: String,
    charCaptions: [[String: Any]],
    charNegativeCaptions: [[String: Any]]
) {
    var parameters = payload["parameters"] as? [String: Any] ?? [:]

    parameters["v4_prompt"] = buildV4PromptStructure(
        prompt: prompt,
        charCaptions: charCaptions
    )
    parameters["v4_negative_prompt"] = buildV4NegativePromptStructure(
        negativePrompt: negativePrompt,
        charNegativeCaptions: charNegativeCaptions
    )

    payload["parameters"] = parameters
}
