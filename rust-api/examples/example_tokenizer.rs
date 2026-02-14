//! NovelAI Tokenizer Example
//!
//! トークナイザーの使い方を示すサンプル。
//!
//! Run with:
//!   cargo run --example example_tokenizer
//!   cargo run --example example_tokenizer -- "your custom prompt here"

use std::time::Instant;

use novelai_api::tokenizer::{get_clip_tokenizer, get_t5_tokenizer};

/// CLIP Tokenizer example
/// - Counts "raw" token count of the prompt
/// - Includes weight syntax (e.g., {beautiful:1.2})
async fn clip_tokenizer_example(text: &str) -> anyhow::Result<()> {
    println!("\n=== CLIP Tokenizer ===");
    println!("Input: \"{}\"", text);

    let tokenizer = get_clip_tokenizer(false).await?;

    let tokens = tokenizer.encode(text);

    let preview: Vec<String> = tokens.iter().take(10).map(|t| t.to_string()).collect();
    let suffix = if tokens.len() > 10 { ", ..." } else { "" };
    println!("Token IDs: [{}{}]", preview.join(", "), suffix);
    println!("Token Count: {}", tokens.len());

    Ok(())
}

/// T5 Tokenizer example
/// - Counts "effective" token count (after removing weight syntax)
/// - Used by NovelAI v4 T5 encoder
async fn t5_tokenizer_example(text: &str) -> anyhow::Result<()> {
    println!("\n=== T5 Tokenizer ===");
    println!("Input: \"{}\"", text);

    let text_with_tags = format!("masterpiece, best quality, {}", text);
    println!("With tags: \"{}\"", text_with_tags);

    let tokenizer = get_t5_tokenizer(false).await?;
    let ids = tokenizer.encode(&text_with_tags);

    let preview: Vec<String> = ids.iter().take(10).map(|t| t.to_string()).collect();
    let suffix = if ids.len() > 10 { ", ..." } else { "" };
    println!("Token IDs: [{}{}]", preview.join(", "), suffix);
    println!("Effective Token Count: {}", ids.len());

    Ok(())
}

/// Cache behavior demonstration
async fn cache_example() -> anyhow::Result<()> {
    println!("\n=== Cache Behavior ===");

    println!("First call (fetches from server)...");
    let start1 = Instant::now();
    get_clip_tokenizer(false).await?;
    println!("Time: {}ms", start1.elapsed().as_millis());

    println!("Second call (uses cache)...");
    let start2 = Instant::now();
    get_clip_tokenizer(false).await?;
    println!("Time: {}ms", start2.elapsed().as_millis());

    println!("Force refresh (fetches again)...");
    let start3 = Instant::now();
    get_clip_tokenizer(true).await?;
    println!("Time: {}ms", start3.elapsed().as_millis());

    Ok(())
}

/// countTokens() demonstration
async fn count_tokens_example() -> anyhow::Result<()> {
    println!("\n=== Count Tokens ===");

    let tokenizer = get_t5_tokenizer(false).await?;

    let count1 = tokenizer.count_tokens(
        "3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}",
    );
    println!("Token count: {} Expected: 25", count1);

    let count2 = tokenizer.count_tokens(
        "1girl, graphite (medium), plaid background, from side, cowboy shot, stuffed animal, stuffed lion, mimikaki, candle, offering hand",
    );
    println!("Token count: {} Expected: 38", count2);

    let count3 = tokenizer.count_tokens(
        "2::girls::, 2::smile, standing, ::, {{ scared }}, 3::sitting::, 3::spread arms, spread wings::",
    );
    println!("Token count: {} Expected: 19", count3);

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let sample_text = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1girl, {beautiful:1.2}, [masterpiece], blonde hair, blue eyes".into());

    println!("========================================");
    println!("NovelAI Tokenizer Example");
    println!("========================================");

    clip_tokenizer_example(&sample_text).await?;
    t5_tokenizer_example(&sample_text).await?;
    cache_example().await?;
    count_tokens_example().await?;

    println!("\nAll examples completed successfully!");

    Ok(())
}
