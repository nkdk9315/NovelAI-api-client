use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::constants;
use crate::error::Result;
use crate::schemas::*;
use crate::utils;

/// Build the base generation payload as a JSON value.
pub fn build_base_payload(
    params: &GenerateParams,
    seed: u64,
    negative_prompt: &str,
) -> serde_json::Value {
    serde_json::json!({
        "input": params.prompt,
        "model": params.model.as_str(),
        "action": params.action.as_str(),
        "parameters": {
            "params_version": 3,
            "width": params.width,
            "height": params.height,
            "scale": params.scale,
            "sampler": params.sampler.as_str(),
            "steps": params.steps,
            "n_samples": 1,
            "ucPreset": 0,
            "qualityToggle": false,
            "autoSmea": false,
            "dynamic_thresholding": false,
            "controlnet_strength": 1,
            "legacy": false,
            "add_original_image": true,
            "cfg_rescale": params.cfg_rescale,
            "noise_schedule": params.noise_schedule.as_str(),
            "legacy_v3_extend": false,
            "skip_cfg_above_sigma": null,
            "use_coords": false,
            "legacy_uc": false,
            "normalize_reference_strength_multiple": true,
            "inpaintImg2ImgStrength": 1,
            "seed": seed,
            "negative_prompt": negative_prompt,
            "deliberate_euler_ancestral_bug": false,
            "prefer_brownian": true,
        },
        "use_new_shared_trial": true,
    })
}

/// Apply Img2Img parameters to the payload.
pub fn apply_img2img_params(
    payload: &mut serde_json::Value,
    params: &GenerateParams,
    seed: u64,
) -> Result<()> {
    if let GenerateAction::Img2Img { ref source_image, strength, noise } = params.action {
        let b64 = utils::image::resize_image_for_img2img(source_image, params.width, params.height)?;
        let extra_seed = if seed == 0 {
            constants::MAX_SEED as u64
        } else {
            seed - 1
        };
        payload["parameters"]["image"] = serde_json::Value::String(b64);
        payload["parameters"]["strength"] = serde_json::json!(strength);
        payload["parameters"]["noise"] = serde_json::json!(noise);
        payload["parameters"]["extra_noise_seed"] = serde_json::json!(extra_seed);
        payload["parameters"]["stream"] = serde_json::Value::String("msgpack".to_string());
        payload["parameters"]["image_format"] = serde_json::Value::String("png".to_string());
    }
    Ok(())
}

/// Apply Infill/Inpaint parameters to the payload.
pub fn apply_infill_params(
    payload: &mut serde_json::Value,
    params: &GenerateParams,
    seed: u64,
) -> Result<()> {
    if let GenerateAction::Infill {
        ref source_image,
        ref mask,
        mask_strength,
        color_correct,
        hybrid_strength,
        hybrid_noise,
    } = params.action
    {
        // Append -inpainting suffix (prevent duplicates)
        let model_str = params.model.as_str();
        let model_name = if model_str.ends_with("-inpainting") {
            model_str.to_string()
        } else {
            format!("{}-inpainting", model_str)
        };
        payload["model"] = serde_json::Value::String(model_name);

        // Source image: resize to target dimensions (same as img2img)
        let source_base64 = utils::image::resize_image_for_img2img(
            source_image, params.width, params.height
        )?;
        let source_buffer = BASE64.decode(source_base64.as_bytes())
            .map_err(|e| crate::error::NovelAIError::Image(format!("Failed to decode resized source image: {}", e)))?;

        // Mask: resize to 1/8 of target dimensions
        let mask_buffer = utils::image::get_image_buffer(mask)?;
        let resized_mask =
            utils::mask::resize_mask_image(&mask_buffer, params.width, params.height)?;
        let mask_base64 = BASE64.encode(&resized_mask);

        // Cache secret keys (SHA256)
        let image_cache_key = utils::mask::calculate_cache_secret_key(&source_buffer);
        let mask_cache_key = utils::mask::calculate_cache_secret_key(&resized_mask);

        // Strength parameters
        let effective_hybrid_strength = hybrid_strength.unwrap_or(mask_strength);
        let effective_hybrid_noise = hybrid_noise.unwrap_or(0.0);

        let extra_seed = if seed == 0 {
            constants::MAX_SEED as u64
        } else {
            seed - 1
        };

        payload["parameters"]["image"] = serde_json::Value::String(source_base64);
        payload["parameters"]["mask"] = serde_json::Value::String(mask_base64);
        payload["parameters"]["strength"] = serde_json::json!(effective_hybrid_strength);
        payload["parameters"]["noise"] = serde_json::json!(effective_hybrid_noise);
        payload["parameters"]["add_original_image"] = serde_json::json!(false);
        payload["parameters"]["extra_noise_seed"] = serde_json::json!(extra_seed);
        payload["parameters"]["inpaintImg2ImgStrength"] = serde_json::json!(mask_strength);
        payload["parameters"]["img2img"] = serde_json::json!({
            "strength": mask_strength,
            "color_correct": color_correct,
        });
        payload["parameters"]["image_cache_secret_key"] =
            serde_json::Value::String(image_cache_key);
        payload["parameters"]["mask_cache_secret_key"] =
            serde_json::Value::String(mask_cache_key);
        payload["parameters"]["image_format"] = serde_json::Value::String("png".to_string());
        payload["parameters"]["stream"] = serde_json::Value::String("msgpack".to_string());
    }

    Ok(())
}

/// Apply Vibe Transfer parameters to the payload.
pub fn apply_vibe_params(
    payload: &mut serde_json::Value,
    vibe_encodings: &[String],
    vibe_strengths: &Option<Vec<f64>>,
    vibe_info_list: &[f64],
) {
    if vibe_encodings.is_empty() {
        return;
    }
    payload["parameters"]["reference_image_multiple"] = serde_json::json!(vibe_encodings);
    payload["parameters"]["reference_strength_multiple"] = serde_json::json!(vibe_strengths);
    payload["parameters"]["reference_information_extracted_multiple"] =
        serde_json::json!(vibe_info_list);
    payload["parameters"]["normalize_reference_strength_multiple"] = serde_json::json!(true);
}

/// Apply Character Reference parameters to the payload.
pub fn apply_char_ref_params(
    payload: &mut serde_json::Value,
    char_ref_data: &crate::utils::charref::ProcessedCharacterReferences,
) {
    payload["parameters"]["director_reference_images"] =
        serde_json::json!(char_ref_data.images);
    payload["parameters"]["director_reference_descriptions"] =
        serde_json::json!(char_ref_data.descriptions);
    payload["parameters"]["director_reference_information_extracted"] =
        serde_json::json!(char_ref_data.info_extracted);
    payload["parameters"]["director_reference_strength_values"] =
        serde_json::json!(char_ref_data.strength_values);
    payload["parameters"]["director_reference_secondary_strength_values"] =
        serde_json::json!(char_ref_data.secondary_strength_values);
    payload["parameters"]["stream"] = serde_json::Value::String("msgpack".to_string());
    payload["parameters"]["image_format"] = serde_json::Value::String("png".to_string());
}

/// Build V4 prompt structure for the payload.
pub fn build_v4_prompt_structure(
    payload: &mut serde_json::Value,
    prompt: &str,
    negative_prompt: &str,
    char_captions: &[CaptionDict],
    char_negative_captions: &[CaptionDict],
) {
    let char_caps: Vec<serde_json::Value> = char_captions
        .iter()
        .map(|c| {
            serde_json::json!({
                "char_caption": c.char_caption,
                "centers": c.centers.iter().map(|center| {
                    serde_json::json!({"x": center.x, "y": center.y})
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    let char_neg_caps: Vec<serde_json::Value> = char_negative_captions
        .iter()
        .map(|c| {
            serde_json::json!({
                "char_caption": c.char_caption,
                "centers": c.centers.iter().map(|center| {
                    serde_json::json!({"x": center.x, "y": center.y})
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    let has_characters = !char_caps.is_empty();
    payload["parameters"]["v4_prompt"] = serde_json::json!({
        "caption": {
            "base_caption": prompt,
            "char_captions": char_caps,
        },
        "use_coords": has_characters,
        "use_order": true,
    });

    payload["parameters"]["v4_negative_prompt"] = serde_json::json!({
        "caption": {
            "base_caption": negative_prompt,
            "char_captions": char_neg_caps,
        },
        "legacy_uc": false,
    });
}

/// Apply character prompts (use_coords) to the payload.
pub fn apply_character_prompts(
    payload: &mut serde_json::Value,
    char_configs: &[CharacterConfig],
) {
    if !char_configs.is_empty() {
        payload["parameters"]["use_coords"] = serde_json::json!(true);
    }
    let prompts: Vec<serde_json::Value> = char_configs
        .iter()
        .map(|c| {
            serde_json::json!({
                "prompt": c.prompt,
                "uc": c.negative_prompt,
                "center": {"x": c.center_x, "y": c.center_y},
                "enabled": true,
            })
        })
        .collect();
    payload["parameters"]["characterPrompts"] = serde_json::json!(prompts);
}
