//! V4.5 regression smoke test against the live API (small sizes to stay in the Opus free tier).
//!
//! Run with: cargo run --example smoke_v45 [step...]

use std::future::Future;
use std::io::Cursor;
use std::time::Instant;

use image::{ImageEncoder, RgbaImage};
use novelai_api::client::NovelAIClient;
use novelai_api::constants::{AugmentReqType, Model};
use novelai_api::schemas::*;

const OUT: &str = "/tmp/novelai_smoke/rust";
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

fn base(prompt: &str, seed: u64, path: &str) -> GenerateParamsBuilder {
    GenerateParams::builder(prompt)
        .model(Model::NaiDiffusion45Full)
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

    step(&wanted, "balance", || async { Ok(format!("{:?}", client.get_anlas_balance().await?)) }).await;
    step(&wanted, "t2i", || async {
        let r = client.generate(&base(PROMPT, 1, "t2i.png").build()?).await?;
        Ok(format!("seed={} {}", r.seed, anlas(r.anlas_consumed, r.anlas_remaining)))
    })
    .await;
    step(&wanted, "i2i", || async {
        let params = base(PROMPT, 2, "i2i.png")
            .action(GenerateAction::Img2Img {
                source_image: ImageInput::FilePath(src.clone().into()),
                strength: 0.6,
                noise: 0.0,
            })
            .build()?;
        let r = client.generate(&params).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "infill", || async {
        let params = base(&format!("{}, cat ears", PROMPT), 3, "infill.png")
            .action(GenerateAction::Infill {
                source_image: ImageInput::FilePath(src.clone().into()),
                mask: ImageInput::Bytes(mask_png(512, 768)),
                mask_strength: 0.7,
                color_correct: true,
                hybrid_strength: None,
                hybrid_noise: None,
            })
            .build()?;
        let r = client.generate(&params).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "vibe", || async {
        let params = base(PROMPT, 4, "vibe.png")
            .vibes(vec![VibeConfig {
                item: VibeItem::FilePath("vibes/input1.naiv4vibe".into()),
                strength: 0.7,
                info_extracted: 0.7,
            }])
            .build()?;
        let r = client.generate(&params).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "charref", || async {
        let params = base(PROMPT, 5, "charref.png")
            .character_reference(CharacterReferenceConfig {
                image: ImageInput::FilePath("reference/input.jpeg".into()),
                strength: 0.6,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            })
            .build()?;
        let r = client.generate(&params).await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "augment", || async {
        let r = client
            .augment_image(&AugmentParams {
                req_type: AugmentReqType::Sketch,
                image: ImageInput::FilePath(src.clone().into()),
                prompt: None,
                defry: None,
                save: SaveTarget::ExactPath(format!("{}/sketch.png", OUT)),
            })
            .await?;
        Ok(anlas(r.anlas_consumed, r.anlas_remaining))
    })
    .await;
    step(&wanted, "upscale", || async {
        let r = client
            .upscale_image(&UpscaleParams {
                image: ImageInput::FilePath(src.clone().into()),
                scale: 2,
                save: SaveTarget::ExactPath(format!("{}/upscale.png", OUT)),
            })
            .await?;
        Ok(format!("{}x{} {}", r.output_width, r.output_height, anlas(r.anlas_consumed, r.anlas_remaining)))
    })
    .await;
    Ok(())
}
