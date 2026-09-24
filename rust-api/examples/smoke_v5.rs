//! V5 smoke test against the live API (small sizes to stay in the Opus free tier).
//!
//! Run with: cargo run --example smoke_v5 [step...]

use std::future::Future;
use std::io::Cursor;
use std::time::Instant;

use image::{ImageEncoder, RgbaImage};
use novelai_api::anlas::summarize_opus_usage;
use novelai_api::client::NovelAIClient;
use novelai_api::constants::{AugmentReqType, Model};
use novelai_api::schemas::*;

const OUT: &str = "/tmp/novelai_smoke/rust_v5";
const PROMPT: &str = "1girl, solo, red apple in hand, simple background";

fn mask_png(width: u32, height: u32) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        if (128..384).contains(&x) && y < 256 {
            image::Rgba([255, 255, 255, 255])
        } else {
            image::Rgba([0, 0, 0, 255])
        }
    });
    let mut buf = Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    buf.into_inner()
}

fn alpha_ratio(path: &str) -> String {
    match image::open(path) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let transparent = rgba.pixels().filter(|p| p[3] == 0).count();
            format!("{}x{} alpha0={:.1}%", rgba.width(), rgba.height(), transparent as f64 * 100.0 / (rgba.width() * rgba.height()) as f64)
        }
        Err(e) => format!("(could not read: {})", e),
    }
}

fn base(prompt: &str, model: Model, seed: u64, path: &str) -> GenerateParamsBuilder {
    GenerateParams::builder(prompt)
        .model(model)
        .width(512)
        .height(768)
        .steps(23)
        .seed(seed)
        .save_path(format!("{}/{}", OUT, path))
}

fn anlas(consumed: Option<u64>, remaining: Option<u64>) -> String {
    format!("anlas consumed={:?} remaining={:?}", consumed, remaining)
}

async fn step<F, Fut>(wanted: &[String], name: &str, f: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<String>>,
{
    if !wanted.is_empty() && !wanted.iter().any(|w| w == name) {
        return;
    }
    let t0 = Instant::now();
    match f().await {
        Ok(info) => println!("OK   {} ({:.1}s) {}", name, t0.elapsed().as_secs_f64(), info),
        Err(e) => println!("FAIL {} ({:.1}s) {}", name, t0.elapsed().as_secs_f64(), e),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    std::fs::create_dir_all(OUT)?;
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    let client = NovelAIClient::new(None, None)?;
    let src = format!("{}/t2i.png", OUT);
    let mask = mask_png(512, 768);

    step(&wanted, "balance", || async {
        let b = client.get_anlas_balance().await?;
        let summary = b.usage.as_ref().map(summarize_opus_usage);
        Ok(format!("total={} tier={} usage={:?} summary={:?}", b.total, b.tier, b.usage, summary))
    })
    .await;
    step(&wanted, "t2i_transparent", || async {
        let p = base(PROMPT, Model::NaiDiffusion5Full, 1, "t2i.png").transparent_background(true).build()?;
        let r = client.generate(&p).await?;
        Ok(format!("{} {}", alpha_ratio(&src), anlas(r.anlas_consumed, r.anlas_remaining)))
    })
    .await;
    step(&wanted, "t2i_webp", || async {
        let p = base(PROMPT, Model::NaiDiffusion5Full, 8, "unused.png")
            .transparent_background(true)
            .image_format(novelai_api::constants::OutputFormat::Webp)
            .save_dir(format!("{}/webp_dir", OUT))
            .build()?;
        let r = client.generate(&p).await?;
        let path = r.saved_path.clone().unwrap_or_default();
        Ok(format!("format={} file={} {} {}", r.image_format, path.rsplit('/').next().unwrap_or(""), alpha_ratio(&path), anlas(r.anlas_consumed, r.anlas_remaining)))
    })
    .await;
    step(&wanted, "i2i", || async {
        let p = base(PROMPT, Model::NaiDiffusion5Full, 2, "i2i.png")
            .transparent_background(true)
            .action(GenerateAction::Img2Img { source_image: ImageInput::FilePath(src.clone().into()), strength: 0.6, noise: 0.0 })
            .build()?;
        let r = client.generate(&p).await?;
        Ok(format!("{} {}", alpha_ratio(&format!("{}/i2i.png", OUT)), anlas(r.anlas_consumed, r.anlas_remaining)))
    })
    .await;
    step(&wanted, "infill_full", || async {
        let p = base(&format!("{}, cat ears", PROMPT), Model::NaiDiffusion5Full, 3, "infill_full.png")
            .action(GenerateAction::Infill {
                source_image: ImageInput::FilePath(src.clone().into()),
                mask: ImageInput::Bytes(mask.clone()),
                mask_strength: 0.7,
                color_correct: true,
                hybrid_strength: None,
                hybrid_noise: None,
            })
            .build()?;
        let r = client.generate(&p).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "t2i_curated", || async {
        let r = client.generate(&base(PROMPT, Model::NaiDiffusion5Curated, 4, "curated.png").build()?).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "infill_curated", || async {
        let p = base(PROMPT, Model::NaiDiffusion5Curated, 5, "infill_curated.png")
            .action(GenerateAction::Infill {
                source_image: ImageInput::FilePath(format!("{}/curated.png", OUT).into()),
                mask: ImageInput::Bytes(mask.clone()),
                mask_strength: 0.7,
                color_correct: true,
                hybrid_strength: None,
                hybrid_noise: None,
            })
            .build()?;
        let r = client.generate(&p).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "characters", || async {
        let p = base("2girls, standing, simple background", Model::NaiDiffusion5Full, 6, "characters.png")
            .characters(vec![
                CharacterConfig { prompt: "girl, red hair".into(), center_x: 0.3, center_y: 0.5, ..Default::default() },
                CharacterConfig { prompt: "girl, blue hair".into(), center_x: 0.7, center_y: 0.5, ..Default::default() },
            ])
            .build()?;
        let r = client.generate(&p).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "vibe_rejected", || async {
        let built = base(PROMPT, Model::NaiDiffusion5Full, 7, "never.png")
            .vibes(vec![VibeConfig { item: VibeItem::FilePath("vibes/input1.naiv4vibe".into()), strength: 0.7, info_extracted: 0.7 }])
            .build();
        Ok(match built {
            Err(e) => format!("rejected locally: {}", e),
            Ok(_) => "UNEXPECTED: accepted".into(),
        })
    })
    .await;
    step(&wanted, "declutter_keep_bubbles", || async {
        let r = client
            .augment_image(&AugmentParams {
                req_type: AugmentReqType::DeclutterKeepBubbles,
                image: ImageInput::FilePath(src.clone().into()),
                prompt: None,
                defry: None,
                save: SaveTarget::ExactPath(format!("{}/declutter.png", OUT)),
            })
            .await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    Ok(())
}
