//! V5 support tests: model helpers, validation, cost, usage and payload.

use novelai_api::anlas::{calculate_generation_cost, summarize_opus_usage, GenerationCostParams};
use novelai_api::client::payload;
use novelai_api::constants::{self, Model};
use novelai_api::schemas::*;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(width, height))
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    buf.into_inner()
}

fn v5(prompt: &str) -> GenerateParams {
    GenerateParams {
        prompt: prompt.to_string(),
        model: Model::NaiDiffusion5Full,
        width: 512,
        height: 768,
        ..Default::default()
    }
}

#[test]
fn model_helpers() {
    assert!(Model::NaiDiffusion5Full.is_v5());
    assert!(Model::NaiDiffusion5Curated.is_v5());
    assert!(!Model::NaiDiffusion45Full.is_v5());
    assert_eq!(Model::NaiDiffusion5Full.inpaint_model(), "nai-diffusion-5-full-inpainting");
    assert_eq!(Model::NaiDiffusion5Curated.inpaint_model(), "nai-diffusion-4-5-curated-inpainting");
    assert_eq!(Model::NaiDiffusion45Full.inpaint_model(), "nai-diffusion-4-5-full-inpainting");
    assert_eq!(Model::NaiDiffusion5Full.max_tokens(), 1471);
    assert_eq!(Model::NaiDiffusion5Curated.max_tokens(), 703);
    assert_eq!(Model::NaiDiffusion45Full.max_tokens(), 512);
    assert_eq!("nai-diffusion-5-full".parse::<Model>().unwrap(), Model::NaiDiffusion5Full);
}

#[test]
fn v5_rejects_vibes_and_charref() {
    let mut p = v5("1girl");
    p.vibes = Some(vec![VibeConfig {
        item: VibeItem::RawEncoding("AAAA".into()),
        strength: 0.7,
        info_extracted: 0.7,
    }]);
    assert!(p.validate().is_err());

    let mut p = v5("1girl");
    p.character_reference = Some(CharacterReferenceConfig {
        image: ImageInput::Bytes(png(64, 64)),
        strength: 0.6,
        fidelity: 1.0,
        mode: CharRefMode::CharacterAndStyle,
    });
    assert!(p.validate().is_err());
}

#[test]
fn character_limits_depend_on_model() {
    let chars = |n: usize| {
        Some((0..n).map(|_| CharacterConfig { prompt: "girl".into(), ..Default::default() }).collect::<Vec<_>>())
    };
    let mut p = v5("2girls");
    p.characters = chars(32);
    assert!(p.validate().is_ok());
    p.characters = chars(33);
    assert!(p.validate().is_err());

    let mut p = GenerateParams { prompt: "2girls".into(), ..Default::default() };
    p.characters = chars(6);
    assert!(p.validate().is_ok());
    p.characters = chars(7);
    assert!(p.validate().is_err());
}

#[test]
fn transparent_background_only_on_v5() {
    let mut p = v5("1girl");
    p.transparent_background = true;
    assert!(p.validate().is_ok());
    assert_eq!(p.effective_prompt(), "1girl, transparent background");

    let p = GenerateParams { prompt: "1girl".into(), transparent_background: true, ..Default::default() };
    assert!(p.validate().is_err());

    let mut p = v5("transparent background, 1girl");
    p.transparent_background = true;
    assert_eq!(p.effective_prompt(), "transparent background, 1girl");
}

#[test]
fn encode_vibe_rejects_v5() {
    let p = EncodeVibeParams {
        image: ImageInput::Bytes(png(64, 64)),
        model: Model::NaiDiffusion5Full,
        ..Default::default()
    };
    assert!(p.validate().is_err());
}

#[test]
fn v5_cost_multiplier_matches_measurement() {
    // Measured: 1088x1024, steps 1 → V5 6 Anlas, V4.5 4 Anlas
    let base = GenerationCostParams { width: 1088, height: 1024, steps: 1, ..Default::default() };
    let v5 = calculate_generation_cost(&GenerationCostParams { is_v5: true, ..base.clone() }).unwrap();
    let v45 = calculate_generation_cost(&base).unwrap();
    assert_eq!(v5.total_cost, 6);
    assert_eq!(v5.model_multiplier, 1.5);
    assert_eq!(v45.total_cost, 4);
}

#[test]
fn v5_opus_free_requires_remaining_usage() {
    let base = GenerationCostParams { width: 832, height: 1216, steps: 23, tier: 3, is_v5: true, ..Default::default() };
    assert_eq!(calculate_generation_cost(&base).unwrap().total_cost, 0);
    let exhausted = calculate_generation_cost(&GenerationCostParams { opus_usage_exhausted: true, ..base.clone() }).unwrap();
    assert!(!exhausted.is_opus_free);
    assert_eq!(exhausted.total_cost, 26); // ceil(17 * 1.5)
    let v45 = GenerationCostParams { is_v5: false, opus_usage_exhausted: true, ..base };
    assert_eq!(calculate_generation_cost(&v45).unwrap().total_cost, 0);
}

#[test]
fn usage_summary_and_parsing() {
    let resp: AnlasBalanceResponse = serde_json::from_str(
        r#"{"tier":3,"trainingStepsLeft":{"fixedTrainingStepsLeft":0,"purchasedTrainingSteps":9960},
            "usage":{"percent":100,"isNegative":false,"timeUntilNextPercent":7888}}"#,
    )
    .unwrap();
    let usage = resp.usage.unwrap();
    let s = summarize_opus_usage(&usage);
    assert_eq!(s.remaining_percent, 100.0);
    assert_eq!(s.refill_percent_per_day, 11.0);
    assert_eq!(s.estimated_images_remaining, 1730);
    assert!(!s.is_low);

    let empty = summarize_opus_usage(&OpusUsage { percent: 3.0, is_negative: true, time_until_next_percent: 0.0 });
    assert_eq!(empty.remaining_percent, 0.0);
    assert!(empty.is_low && empty.is_exhausted);

    let no_usage: AnlasBalanceResponse = serde_json::from_str(r#"{"tier":0}"#).unwrap();
    assert!(no_usage.usage.is_none());
}

#[test]
fn v5_payload() {
    let mut p = v5("1girl");
    p.transparent_background = true;
    p.noise_schedule = constants::NoiseSchedule::Exponential;
    let payload = payload::build_base_payload(&p, 1, "neg");
    assert_eq!(payload["input"], "1girl, transparent background");
    assert_eq!(payload["parameters"]["params_version"], 4);
    assert_eq!(payload["parameters"]["noise_schedule"], "karras");
    assert_eq!(payload["parameters"]["straight_alpha"], true);
    assert_eq!(payload["parameters"]["tag_hint_transparent_background"], true);

    let v45 = payload::build_base_payload(&GenerateParams { prompt: "1girl".into(), ..Default::default() }, 1, "neg");
    assert_eq!(v45["parameters"]["params_version"], 3);
    assert!(v45["parameters"].get("straight_alpha").is_none());
}

#[test]
fn v5_curated_infill_uses_45_curated_inpainting() {
    let image = png(512, 768);
    let p = GenerateParams {
        prompt: "1girl".into(),
        model: Model::NaiDiffusion5Curated,
        width: 512,
        height: 768,
        action: GenerateAction::Infill {
            source_image: ImageInput::Bytes(image.clone()),
            mask: ImageInput::Bytes(image),
            mask_strength: 0.7,
            color_correct: true,
            hybrid_strength: None,
            hybrid_noise: None,
        },
        ..Default::default()
    };
    let mut payload = payload::build_base_payload(&p, 1, "neg");
    payload::apply_infill_params(&mut payload, &p).unwrap();
    assert_eq!(payload["model"], "nai-diffusion-4-5-curated-inpainting");
}
