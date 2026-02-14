//! NovelAI Validation Test Example
//!
//! バリデーション動作の確認用サンプル。
//!
//! Run with: cargo run --example test_validation

use novelai_api::constants::MAX_PIXELS;
use novelai_api::error::NovelAIError;
use novelai_api::schemas::GenerateParams;
use novelai_api::tokenizer::validate_token_count;

#[tokio::main]
async fn main() {
    println!("=== バリデーションテスト ===\n");

    // Test 1: 1216x832 (OK)
    println!("Test 1: 1216x832");
    match GenerateParams::builder("test").width(1216).height(832).build() {
        Ok(params) => {
            let pixels = params.width as u64 * params.height as u64;
            println!("  OK - バリデーション成功");
            println!("  総ピクセル数: {} (MAX: {})\n", pixels, MAX_PIXELS);
        }
        Err(e) => {
            println!("  NG - バリデーションエラー: {}\n", e);
        }
    }

    // Test 2: 1280x1280 (OK - under MAX_PIXELS)
    println!("Test 2: 1280x1280");
    match GenerateParams::builder("test").width(1280).height(1280).build() {
        Ok(params) => {
            let pixels = params.width as u64 * params.height as u64;
            println!("  OK - バリデーション成功");
            println!("  総ピクセル数: {}\n", pixels);
        }
        Err(e) => {
            println!("  NG - バリデーションエラー: {}\n", e);
        }
    }

    // Test 3: 1024x1024 (OK - within Opus free limit)
    println!("Test 3: 1024x1024");
    match GenerateParams::builder("test").width(1024).height(1024).build() {
        Ok(params) => {
            let pixels = params.width as u64 * params.height as u64;
            println!("  OK - バリデーション成功");
            println!("  総ピクセル数: {}\n", pixels);
        }
        Err(e) => {
            println!("  NG - バリデーションエラー: {}\n", e);
        }
    }

    // ===== Token count validation tests =====
    println!("\n=== トークン数バリデーションテスト ===\n");

    // Test 4: Short prompt (under 512 tokens) - OK
    println!("Test 4: 短いプロンプト (512トークン以下)");
    let short_prompt = "a beautiful landscape with mountains and rivers";
    match validate_token_count(short_prompt).await {
        Ok(count) => {
            println!("  OK - トークン数: {}", count);
            println!("  プロンプト: \"{}\"\n", short_prompt);
        }
        Err(e) => {
            println!("  NG - エラー: {}\n", e);
        }
    }

    // Test 5: Long prompt (exceeds 512 tokens) - NG expected
    println!("Test 5: 長すぎるプロンプト (512トークン超過)");
    let long_prompt = std::iter::repeat_n("masterpiece beautiful detailed anime girl", 600)
        .collect::<Vec<_>>()
        .join(", ");
    match validate_token_count(&long_prompt).await {
        Ok(count) => {
            println!(
                "  NG - バリデーション成功（これは期待されない結果です）: トークン数={}\n",
                count
            );
        }
        Err(NovelAIError::TokenValidation {
            token_count,
            max_tokens,
        }) => {
            println!("  OK - トークン検証エラー（期待通り）:");
            println!("  トークン数: {}, 上限: {}\n", token_count, max_tokens);
        }
        Err(e) => {
            println!("  予期しないエラー: {}\n", e);
        }
    }

    // Test 6: validateTokenCount direct test
    println!("Test 6: validateTokenCount関数の直接テスト");
    match validate_token_count("hello world").await {
        Ok(count) => println!("  OK - 短いプロンプト: トークン数={}", count),
        Err(e) => println!("  NG - 短いプロンプトでエラー: {}", e),
    }

    let long_prompt2 = std::iter::repeat_n("masterpiece beautiful detailed anime", 600)
        .collect::<Vec<_>>()
        .join(", ");
    match validate_token_count(&long_prompt2).await {
        Ok(count) => {
            println!(
                "  NG - 長いプロンプトが通過（エラーになるべき）: トークン数={}",
                count
            );
        }
        Err(NovelAIError::TokenValidation {
            token_count,
            max_tokens,
        }) => {
            println!(
                "  OK - 長いプロンプトでエラー: トークン数={}, 上限={}",
                token_count, max_tokens
            );
        }
        Err(e) => {
            println!("  予期しないエラー: {}", e);
        }
    }

    println!("\n=== テスト完了 ===");
}
