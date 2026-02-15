//! NovelAI Augment & Upscale API Example
//!
//! 画像加工ツール（カラー化、表情変換、スケッチ化など）とアップスケール機能のサンプル
//!
//! 注意: width/height は画像から自動検出されるため、指定不要です
//!
//! Run with: cargo run --example example_augment

use anyhow::Result;
use novelai_api::client::NovelAIClient;
use novelai_api::constants::AugmentReqType;
use novelai_api::schemas::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let output_dir = "output/augment";
    std::fs::create_dir_all(output_dir)?;

    let client = NovelAIClient::new(None, None)?;
    let input_image = "reference/input.jpeg";

    // アンラス残高を確認
    let balance = client.get_anlas_balance().await?;
    println!("\n現在のアンラス残高: {}", balance.total);
    println!("  (固定: {}, 購入済み: {})\n", balance.fixed, balance.purchased);

    // =====================================================
    // 1. カラー化 (colorize)
    // =====================================================
    println!("カラー化テスト...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::Colorize,
            image: ImageInput::FilePath(input_image.into()),
            prompt: Some("vibrant colors, detailed shading".into()),
            defry: Some(3),
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 2. 表情変換 (emotion)
    // =====================================================
    println!("\n表情変換テスト...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::Emotion,
            image: ImageInput::FilePath(input_image.into()),
            prompt: Some("happy".into()),
            defry: Some(0),
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 3. スケッチ化 (sketch)
    // =====================================================
    println!("\nスケッチ化テスト...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::Sketch,
            image: ImageInput::FilePath(input_image.into()),
            prompt: None,
            defry: None,
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 4. 線画抽出 (lineart)
    // =====================================================
    println!("\n線画抽出テスト...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::Lineart,
            image: ImageInput::FilePath(input_image.into()),
            prompt: None,
            defry: None,
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 5. デクラッター (declutter)
    // =====================================================
    println!("\nデクラッターテスト...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::Declutter,
            image: ImageInput::FilePath(input_image.into()),
            prompt: None,
            defry: None,
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 6. 背景除去 (bg-removal) - 常にアンラス消費
    // =====================================================
    println!("\n背景除去テスト（常にアンラス消費）...");
    match client
        .augment_image(&AugmentParams {
            req_type: AugmentReqType::BgRemoval,
            image: ImageInput::FilePath(input_image.into()),
            prompt: None,
            defry: None,
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // =====================================================
    // 7. アップスケール (upscale) - 常にアンラス消費
    // =====================================================
    println!("\nアップスケールテスト（常にアンラス消費）...");
    match client
        .upscale_image(&UpscaleParams {
            image: ImageInput::FilePath(input_image.into()),
            scale: 4,
            save: SaveTarget::Directory { dir: output_dir.into(), filename: None },
        })
        .await
    {
        Ok(result) => {
            println!(
                "  保存先: {}",
                result.saved_path.as_deref().unwrap_or("N/A")
            );
            println!(
                "  出力サイズ: {}x{}",
                result.output_width, result.output_height
            );
            println!(
                "  消費アンラス: {}",
                result
                    .anlas_consumed
                    .map_or("N/A".into(), |v| v.to_string())
            );
        }
        Err(e) => println!("  エラー: {}", e),
    }

    // 最終アンラス残高を確認
    let final_balance = client.get_anlas_balance().await?;
    println!("\n最終アンラス残高: {}", final_balance.total);
    println!(
        "  総消費: {}\n",
        balance.total.saturating_sub(final_balance.total)
    );

    Ok(())
}
