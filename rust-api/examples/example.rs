//! NovelAI Unified Client Example
//! すべての機能が統合された generate() と encode_vibe() の使用例
//!
//! Run with: cargo run --example example

use std::path::Path;

use anyhow::Result;
use novelai_api::client::NovelAIClient;
use novelai_api::error::NovelAIError;
use novelai_api::schemas::*;

#[allow(dead_code)]
async fn example_simple_generate() -> Result<()> {
    println!("\n=== シンプル生成 ===");

    let client = NovelAIClient::new(None, None)?;

    let params = GenerateParams::builder(
        "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
    )
    .save_dir("output/")
    .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!("  Seed: {}", result.seed);

    Ok(())
}

#[allow(dead_code)]
async fn example_with_vibes() -> Result<()> {
    println!("\n=== Vibe Transfer使用 ===");

    let client = NovelAIClient::new(None, None)?;

    let vibe_files = vec!["vibes/input1.naiv4vibe"];
    let valid_vibes: Vec<&str> = vibe_files
        .into_iter()
        .filter(|f| Path::new(f).exists())
        .collect();

    if valid_vibes.is_empty() {
        println!("Vibeファイルが見つかりません");
        return Ok(());
    }

    let characters = vec![
        CharacterConfig {
            prompt: "3::cynthia (pokemon) school uniform::, 3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::, 2::deep kiss::, 3::saliva on breasts and areolae::".into(),
            center_x: 0.2,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
        CharacterConfig {
            prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::".into(),
            center_x: 0.8,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
    ];

    let vibes: Vec<VibeItem> = valid_vibes
        .iter()
        .map(|f| VibeItem::FilePath(f.to_string()))
        .collect();
    let vibe_strengths: Vec<f64> = [0.4, 0.3, 0.5, 0.2]
        .iter()
        .take(valid_vibes.len())
        .copied()
        .collect();

    let params = GenerateParams::builder(
        "school classroom, sunny day, wide shot, detailed background, 2::face focus::, -3::multiple views::",
    )
    .characters(characters)
    .vibes(vibes)
    .vibe_strengths(vibe_strengths)
    .width(1024)
    .height(1024)
    .save_dir("output/multi_character/")
    .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!(
        "残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );
    println!(
        "今回消費: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_img2img() -> Result<()> {
    println!("\n=== Image2Image ===");

    let client = NovelAIClient::new(None, None)?;

    let input_image = "reference/input.jpeg";
    if !Path::new(input_image).exists() {
        println!("入力画像が見つかりません: {}", input_image);
        return Ok(());
    }

    let characters = vec![
        CharacterConfig {
            prompt: "3::cynthia (pokemon) school uniform::, 3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::, 2::deep kiss::, 3::saliva on breasts and areolae::".into(),
            center_x: 0.2,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
        CharacterConfig {
            prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::".into(),
            center_x: 0.8,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
    ];

    let params = GenerateParams::builder(
        "backstreet, night, neon lights, detailed background, 2::face focus::, -3::multiple views::",
    )
    .action(GenerateAction::Img2Img)
    .characters(characters)
    .source_image(ImageInput::FilePath(input_image.into()))
    .img2img_strength(0.8)
    .save_dir("output/")
    .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!(
        "残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );
    println!(
        "今回消費: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_img2img_with_vibes() -> Result<()> {
    println!("\n=== Image2Image + Vibe Transfer ===");

    let client = NovelAIClient::new(None, None)?;

    let input_image = "reference/input.jpeg";
    let vibe_file = "vibes/input1.naiv4vibe";

    if !Path::new(input_image).exists() {
        println!("入力画像が見つかりません: {}", input_image);
        return Ok(());
    }

    let mut builder = GenerateParams::builder("")
        .action(GenerateAction::Img2Img)
        .source_image(ImageInput::FilePath(input_image.into()))
        .img2img_strength(0.5)
        .img2img_noise(0.0)
        .width(1024)
        .height(1024)
        .save_dir("output/");

    if Path::new(vibe_file).exists() {
        builder = builder
            .vibes(vec![VibeItem::FilePath(vibe_file.into())])
            .vibe_strengths(vec![0.7]);
    }

    let params = builder.build()?;
    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!(
        "残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );
    println!(
        "今回消費: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_multi_character() -> Result<()> {
    println!("\n=== 複数キャラクター ===");

    let client = NovelAIClient::new(None, None)?;

    let characters = vec![
        CharacterConfig {
            prompt: "3::cynthia (pokemon) school uniform::, 3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::, 2::deep kiss::, 3::saliva on breasts and areolae::".into(),
            center_x: 0.2,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
        CharacterConfig {
            prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::".into(),
            center_x: 0.8,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
    ];

    let vibe_files = vec!["vibes/input1.naiv4vibe", "vibes/input2.naiv4vibe"];
    let valid_vibes: Vec<&str> = vibe_files
        .into_iter()
        .filter(|f| Path::new(f).exists())
        .collect();

    let mut builder = GenerateParams::builder("-3::multiple views::")
        .characters(characters)
        .width(1024)
        .height(1024)
        .save_dir("output/multi_character/");

    if !valid_vibes.is_empty() {
        let vibes: Vec<VibeItem> = valid_vibes
            .iter()
            .map(|f| VibeItem::FilePath(f.to_string()))
            .collect();
        let strengths: Vec<f64> = [0.4, 0.3, 0.5, 0.2]
            .iter()
            .take(valid_vibes.len())
            .copied()
            .collect();
        builder = builder.vibes(vibes).vibe_strengths(strengths);
    }

    let params = builder.build()?;
    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!(
        "残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );
    println!(
        "今回消費: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_encode_vibe() -> Result<()> {
    println!("\n=== Vibeエンコード ===");

    let client = NovelAIClient::new(None, None)?;

    let image_path = "reference/input.jpeg";
    if !Path::new(image_path).exists() {
        println!("参照画像が見つかりません: {}", image_path);
        return Ok(());
    }

    let params = EncodeVibeParams {
        image: ImageInput::FilePath(image_path.into()),
        save_dir: Some("./vibes".into()),
        save_filename: Some("input1".into()),
        ..Default::default()
    };

    let result = client.encode_vibe(&params).await?;

    println!(
        "  Saved: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!(
        "残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );
    println!(
        "今回消費: {}",
        result
            .anlas_consumed
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_character_reference() -> Result<()> {
    println!("\n=== キャラクター参照 ===");

    let client = NovelAIClient::new(None, None)?;

    let reference_image = "reference/input.jpeg";
    if !Path::new(reference_image).exists() {
        println!("参照画像が見つかりません: {}", reference_image);
        return Ok(());
    }

    let params = GenerateParams::builder("school classroom, sunny day, detailed background")
        .characters(vec![CharacterConfig {
            prompt: "3::peeing::".into(),
            center_x: 0.5,
            center_y: 0.5,
            negative_prompt: String::new(),
        }])
        .character_reference(CharacterReferenceConfig {
            image: ImageInput::FilePath(reference_image.into()),
            strength: 0.9,
            fidelity: 0.9,
            mode: CharRefMode::CharacterAndStyle,
        })
        .save_dir("output/charref/")
        .build()?;

    let result = client.generate(&params).await?;

    println!(
        "  Generated: {}",
        result.saved_path.as_deref().unwrap_or("(not saved)")
    );
    println!("  Seed: {}", result.seed);
    println!(
        "  残りアンラス: {}",
        result
            .anlas_remaining
            .map_or("N/A".into(), |v| v.to_string())
    );

    Ok(())
}

#[allow(dead_code)]
async fn example_character_reference_styles() -> Result<()> {
    println!("\n=== キャラクター参照モード比較 ===");

    let client = NovelAIClient::new(None, None)?;

    let reference_image = "reference/input.jpeg";
    if !Path::new(reference_image).exists() {
        println!("参照画像が見つかりません: {}", reference_image);
        return Ok(());
    }

    let modes = [
        CharRefMode::Character,
        CharRefMode::CharacterAndStyle,
        CharRefMode::Style,
    ];

    for mode in &modes {
        println!("\n--- mode: \"{}\" ---", mode.as_str());

        // Style mode omits characters (use_coords: false)
        let characters = if *mode == CharRefMode::Style {
            None
        } else {
            Some(vec![CharacterConfig {
                prompt: "1girl, standing".into(),
                center_x: 0.5,
                center_y: 0.5,
                negative_prompt: String::new(),
            }])
        };

        let mut builder =
            GenerateParams::builder("school classroom, sunny day, detailed background")
                .character_reference(CharacterReferenceConfig {
                    image: ImageInput::FilePath(reference_image.into()),
                    strength: 0.6,
                    fidelity: 0.8,
                    mode: *mode,
                })
                .save_dir("output/charref/");

        if let Some(chars) = characters {
            builder = builder.characters(chars);
        }

        match builder.build() {
            Ok(params) => match client.generate(&params).await {
                Ok(result) => {
                    println!(
                        "  Generated: {}",
                        result.saved_path.as_deref().unwrap_or("(not saved)")
                    );
                    println!("  Seed: {}", result.seed);
                    println!(
                        "  残りアンラス: {}",
                        result
                            .anlas_remaining
                            .map_or("N/A".into(), |v| v.to_string())
                    );
                }
                Err(e) => println!("Error: {}", e),
            },
            Err(e) => println!("Validation error: {}", e),
        }
    }

    Ok(())
}

fn print_error(e: &anyhow::Error) {
    if let Some(nai_err) = e.downcast_ref::<NovelAIError>() {
        match nai_err {
            NovelAIError::Validation(msg) | NovelAIError::Range(msg) => {
                eprintln!("  バリデーションエラー: {}", msg);
            }
            _ => eprintln!("Error: {}", e),
        }
    } else {
        eprintln!("Error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Ensure output directories exist
    let output_dirs = ["output", "output/multi_character", "output/charref", "vibes"];
    for dir in &output_dirs {
        std::fs::create_dir_all(dir).ok();
    }

    println!("{}", "=".repeat(50));
    println!("NovelAI Unified Client 使用例 (Rust)");
    println!("{}", "=".repeat(50));

    // 実行したい例のコメントを外してください

    // if let Err(e) = example_simple_generate().await { print_error(&e.into()); }

    // if let Err(e) = example_with_vibes().await { print_error(&e.into()); }

    // if let Err(e) = example_img2img().await { print_error(&e.into()); }

    // if let Err(e) = example_img2img_with_vibes().await {
    //     print_error(&e);
    // }

    // if let Err(e) = example_multi_character().await { print_error(&e.into()); }

    // if let Err(e) = example_encode_vibe().await { print_error(&e.into()); }

    if let Err(e) = example_character_reference().await {
        print_error(&e);
    }

    // if let Err(e) = example_character_reference_styles().await { print_error(&e.into()); }

    println!("\n使用したい例のコード内のコメントを外して実行してください。");
}
