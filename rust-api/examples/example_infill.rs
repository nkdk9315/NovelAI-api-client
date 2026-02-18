//! NovelAI Infill (Inpaint) Example
//!
//! マスクを使った部分再生成のサンプル
//!
//! Run with: cargo run --example example_infill

use std::io::Cursor;
use std::path::Path;

use anyhow::Result;
use image::{ImageEncoder, RgbaImage};
use novelai_api::client::NovelAIClient;
use novelai_api::schemas::*;

/// Create a PNG mask image in memory.
/// White rectangle = area to regenerate, black = area to keep.
fn create_mask(
    width: u32,
    height: u32,
    rect_x: u32,
    rect_y: u32,
    rect_w: u32,
    rect_h: u32,
) -> Result<Vec<u8>> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        if x >= rect_x && x < rect_x + rect_w && y >= rect_y && y < rect_y + rect_h {
            image::Rgba([255, 255, 255, 255]) // White = regenerate
        } else {
            image::Rgba([0, 0, 0, 255]) // Black = keep
        }
    });

    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder.write_image(
        img.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )?;

    Ok(buf.into_inner())
}

/// Test 1: Basic img2img (baseline)
async fn test_img2img(client: &NovelAIClient, input_image: &str, output_dir: &str) -> Result<()> {
    println!("\n=== Test: Img2Img ===");

    let params = GenerateParams::builder("1girl, beautiful, masterpiece")
        .action(GenerateAction::Img2Img {
            source_image: ImageInput::FilePath(input_image.into()),
            strength: 0.6,
            noise: 0.1,
        })
        .width(832)
        .height(1216)
        .save_dir(output_dir)
        .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Img2Img success! Saved to: {}",
        result.saved_path.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Anlas consumed: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

/// Test 2: Infill + Img2Img hybrid mode
async fn test_infill_with_img2img(
    client: &NovelAIClient,
    input_image: &str,
    output_dir: &str,
) -> Result<()> {
    println!("\n=== Test: Infill + Img2Img (Hybrid Mode) ===");

    let mask_bytes = create_mask(832, 1216, 116, 208, 600, 800)?;
    println!("  Created in-memory mask (832x1216, rect at 116,208 600x800)");

    let params = GenerateParams::builder("1girl, beautiful dress, elegant")
        .action(GenerateAction::Infill {
            source_image: ImageInput::FilePath(input_image.into()),
            mask: ImageInput::Bytes(mask_bytes),
            mask_strength: 0.68,
            color_correct: false,
            hybrid_strength: Some(0.45),
            hybrid_noise: Some(0.0),
        })
        .width(832)
        .height(1216)
        .save_dir(output_dir)
        .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Infill + Img2Img (Hybrid) success! Saved to: {}",
        result.saved_path.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Anlas consumed: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let input_image = "./reference/input.jpeg";
    let output_dir = "./output/test/";

    std::fs::create_dir_all(output_dir)?;

    println!("========================================");
    println!("NovelAI API Infill Tests");
    println!("Input Image: {}", input_image);
    println!("Output Dir: {}", output_dir);
    println!("========================================");

    if !Path::new(input_image).exists() {
        eprintln!("Input image not found: {}", input_image);
        std::process::exit(1);
    }

    let client = NovelAIClient::new(None, None)?;

    let mut results: Vec<(&str, bool)> = Vec::new();

    // Test 1: Img2Img
    let success = test_img2img(&client, input_image, output_dir).await.is_ok();
    results.push(("Img2Img", success));

    // Test 2: Infill + Img2Img (Hybrid)
    let success = test_infill_with_img2img(&client, input_image, output_dir)
        .await
        .is_ok();
    results.push(("Infill + Img2Img", success));

    // Summary
    println!("\n========================================");
    println!("Test Results Summary");
    println!("========================================");
    for (name, success) in &results {
        let icon = if *success { "OK" } else { "NG" };
        println!("{} {}", icon, name);
    }

    let failed = results.iter().filter(|(_, s)| !*s).count();
    if failed > 0 {
        println!("\n{} test(s) failed.", failed);
        std::process::exit(1);
    } else {
        println!("\nAll tests passed!");
    }

    Ok(())
}
