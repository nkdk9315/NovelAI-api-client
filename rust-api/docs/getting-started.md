# クイックスタート

## インストール

`Cargo.toml` に依存を追加:

```toml
[dependencies]
novelai-api = { path = "../rust-api" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
dotenvy = "0.15"
```

## 環境変数

`.env` ファイルまたは環境変数で API キーを設定:

```bash
NOVELAI_API_KEY=your_api_key_here
```

---

## 基本的な画像生成 (txt2img)

```rust
use novelai_api::client::NovelAIClient;
use novelai_api::schemas::{GenerateParams, SaveTarget};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let client = NovelAIClient::new(None)?;

    let params = GenerateParams::builder()
        .prompt("1girl, best quality, amazing quality, very aesthetic, absurdres")
        .width(832)
        .height(1216)
        .save_dir("./output")
        .build()?;

    let result = client.generate(&params).await?;

    println!("生成完了! seed: {}", result.seed);
    if let Some(path) = &result.saved_path {
        println!("保存先: {}", path);
    }

    Ok(())
}
```

### ポイント

- `NovelAIClient::new(None)` は環境変数 `NOVELAI_API_KEY` を自動読取
- `GenerateParams::builder()` でメソッドチェーン → `.build()` でバリデーション
- `.save_dir("./output")` で出力ディレクトリを指定 (ファイル名は自動生成)
- `.save_path("./output/exact_name.png")` で完全パスを指定することも可能

---

## img2img (画像から画像)

```rust
use novelai_api::schemas::{GenerateAction, GenerateParams, ImageInput};

let params = GenerateParams::builder()
    .prompt("1girl, blue hair")
    .action(GenerateAction::Img2Img {
        source_image: ImageInput::FilePath("./input.png".into()),
        strength: 0.62,
        noise: 0.0,
    })
    .width(832)
    .height(1216)
    .save_dir("./output")
    .build()?;

let result = client.generate(&params).await?;
```

### ImageInput の柔軟性

画像入力は4つの形式に対応:

```rust
// ファイルパス
ImageInput::FilePath("./image.png".into())

// Base64文字列
ImageInput::Base64("iVBORw0KGgo...".to_string())

// データURL
ImageInput::DataUrl("data:image/png;base64,iVBORw0KGgo...".to_string())

// バイト列
ImageInput::Bytes(vec![0x89, 0x50, 0x4e, 0x47, ...])
```

---

## Inpaint (部分修正)

```rust
use novelai_api::schemas::GenerateAction;
use novelai_api::utils::mask;

// プログラムでマスクを生成 (中央領域を白く塗る)
let mask_png = mask::create_rectangular_mask(
    832, 1216,
    &mask::MaskRegion { x: 0.25, y: 0.25, w: 0.5, h: 0.5 },
)?;

let params = GenerateParams::builder()
    .prompt("1girl, red eyes")
    .action(GenerateAction::Infill {
        source_image: ImageInput::FilePath("./input.png".into()),
        mask: ImageInput::Bytes(mask_png),
        mask_strength: 0.68,
        color_correct: true,
        hybrid_strength: None,
        hybrid_noise: None,
    })
    .width(832)
    .height(1216)
    .save_dir("./output")
    .build()?;

let result = client.generate(&params).await?;
```

### マスク生成ユーティリティ

```rust
// 矩形マスク (相対座標 0.0-1.0)
let mask = mask::create_rectangular_mask(width, height, &MaskRegion { x, y, w, h })?;

// 円形マスク
let mask = mask::create_circular_mask(width, height, &MaskCenter { x, y }, radius)?;

// 既存のマスク画像ファイルも使用可能
ImageInput::FilePath("./mask.png".into())
```

---

## Vibe Transfer

```rust
use novelai_api::schemas::{VibeConfig, VibeItem};

// 事前エンコード済み .naiv4vibe ファイルを使用
let params = GenerateParams::builder()
    .prompt("1girl")
    .vibes(vec![
        VibeConfig {
            item: VibeItem::FilePath("./style.naiv4vibe".into()),
            strength: 0.7,
            info_extracted: 0.7,
        },
    ])
    .save_dir("./output")
    .build()?;

let result = client.generate(&params).await?;
```

### Vibe エンコード

画像から Vibe エンコーディングを生成:

```rust
use novelai_api::schemas::{EncodeVibeParams, ImageInput, SaveTarget};

let params = EncodeVibeParams {
    image: ImageInput::FilePath("./reference.png".into()),
    information_extracted: 0.7,
    strength: 0.7,
    save: SaveTarget::Directory {
        dir: "./vibes".to_string(),
        filename: None,
    },
};

let result = client.encode_vibe(&params).await?;
println!("Vibe エンコード完了: {} bytes", result.encoding.len());
```

---

## Character Reference

```rust
use novelai_api::schemas::{CharacterReferenceConfig, CharRefMode};

let params = GenerateParams::builder()
    .prompt("1girl, standing")
    .character_reference(vec![
        CharacterReferenceConfig {
            image: ImageInput::FilePath("./character.png".into()),
            mode: CharRefMode::CharacterAndStyle,
            strength: 0.6,
            fidelity: 1.0,
        },
    ])
    .save_dir("./output")
    .build()?;

let result = client.generate(&params).await?;
```

### CharRefMode

| モード | 説明 |
|-------|------|
| `CharRefMode::Character` | キャラクターのみ参照 |
| `CharRefMode::CharacterAndStyle` | キャラクター + スタイル参照 |
| `CharRefMode::Style` | スタイルのみ参照 |

---

## マルチキャラクター

```rust
use novelai_api::schemas::CharacterConfig;

let params = GenerateParams::builder()
    .prompt("2girls, facing each other")
    .characters(vec![
        CharacterConfig {
            prompt: "red hair, red eyes".to_string(),
            center_x: 0.3,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
        CharacterConfig {
            prompt: "blue hair, blue eyes".to_string(),
            center_x: 0.7,
            center_y: 0.5,
            negative_prompt: String::new(),
        },
    ])
    .save_dir("./output")
    .build()?;

let result = client.generate(&params).await?;
```

---

## Augment (画像加工)

```rust
use novelai_api::constants::AugmentReqType;
use novelai_api::schemas::AugmentParams;

// カラー化
let params = AugmentParams {
    image: ImageInput::FilePath("./input.png".into()),
    req_type: AugmentReqType::Colorize,
    prompt: Some("vibrant colors".to_string()),
    defry: Some(3),
    save: SaveTarget::Directory {
        dir: "./output".to_string(),
        filename: None,
    },
};

let result = client.augment_image(&params).await?;
```

### 利用可能なツール

| ツール | req_type | prompt | defry |
|-------|----------|--------|-------|
| カラー化 | `Colorize` | 任意テキスト | 0-5 (必須) |
| 表情変更 | `Emotion` | キーワード (happy, sad等) | 0-5 (必須) |
| スケッチ | `Sketch` | 不要 | 不要 |
| 線画抽出 | `Lineart` | 不要 | 不要 |
| デクラッター | `Declutter` | 不要 | 不要 |
| 背景削除 | `BgRemoval` | 不要 | 不要 |

---

## Upscale (アップスケール)

```rust
use novelai_api::schemas::UpscaleParams;

let params = UpscaleParams {
    image: ImageInput::FilePath("./input.png".into()),
    scale: 4,
    save: SaveTarget::Directory {
        dir: "./output".to_string(),
        filename: None,
    },
};

let result = client.upscale_image(&params).await?;
println!("出力サイズ: {}x{}", result.output_width, result.output_height);
```

---

## エラーハンドリング

```rust
use novelai_api::error::NovelAIError;

match client.generate(&params).await {
    Ok(result) => println!("成功! seed: {}", result.seed),
    Err(NovelAIError::Validation(msg)) => eprintln!("パラメータエラー: {}", msg),
    Err(NovelAIError::Api { status_code, message }) => {
        eprintln!("APIエラー ({}): {}", status_code, message);
    }
    Err(NovelAIError::TokenValidation { token_count, max_tokens }) => {
        eprintln!("トークン超過: {}/{}", token_count, max_tokens);
    }
    Err(e) => eprintln!("その他のエラー: {}", e),
}
```

---

## Anlas 残高確認

```rust
let balance = client.get_anlas_balance().await?;
println!("Anlas残高: {}", balance.fixed_training_steps_left + balance.purchased_training_steps);
```

---

## Tips

- **シード固定**: `.seed(12345)` で再現可能な生成
- **ネガティブプロンプト**: `.negative_prompt("bad quality, low quality")`
- **サンプラー変更**: `.sampler(Sampler::KEulerAncestral)`
- **ノイズスケジュール**: `.noise_schedule(NoiseSchedule::Karras)`
- **CFG Rescale**: `.cfg_rescale(0.0)` (0.0-1.0)
- **バリデーション**: `.build()` は同期バリデーション、`validate_async()` でトークン数チェックも実行
