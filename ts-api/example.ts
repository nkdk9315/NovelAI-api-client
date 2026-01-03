/**
 * NovelAI Unified Client Example
 * すべての機能が統合された generate() と encode_vibe() の使用例
 *
 * Run with: npx ts-node example.ts
 */

import path from 'path';
import fs from 'fs';
import dotenv from 'dotenv';
import { NovelAIClient } from './src/client';
import * as Schemas from './src/schemas';
import * as Constants from './src/constants';
import { ZodError } from 'zod';

// Load environment variables
dotenv.config();

// Ensure output directories exist
const outputDirs = ["output", "output/multi_character", "output/charref", "vibes"];
outputDirs.forEach(dir => {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
});

async function exampleSimpleGenerate() {
  console.log("\n=== シンプル生成 ===");

  try {
    const client = new NovelAIClient();

    const result = await client.generate({
      prompt: "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
      save_dir: "output/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`  Seed: ${result.seed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleWithVibes() {
  console.log("\n=== Vibe Transfer使用 ===");

  try {
    const client = new NovelAIClient();

    const vibeFiles = [
      "vibes/input1.naiv4vibe",
    ];

    // Filter existing
    const validVibes = vibeFiles.filter(f => fs.existsSync(f));

    if (validVibes.length === 0) {
      console.log("Vibeファイルが見つかりません");
      return;
    }
    const characters: Schemas.CharacterConfig[] = [
      {
        prompt: "3::cynthia (pokemon) school uniform::,  3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::,2::deep kiss::, 3::saliva on breasts and areolae::",
        center_x: 0.2,
        center_y: 0.5,
        negative_prompt: ""
      },
      {
        prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::,  ",
        center_x: 0.8,
        center_y: 0.5,
        negative_prompt: ""
      },
    ];

    const result = await client.generate({
      prompt: "school classroom, sunny day, wide shot, detailed background, 2::face focus::, -3::multiple views::",
      characters: characters,
      vibes: validVibes.length > 0 ? validVibes : undefined,
      vibe_strengths: validVibes.length > 0 ? [0.4, 0.3, 0.5, 0.2].slice(0, validVibes.length) : undefined,
      width: 1024,
      height: 1024,
      save_dir: "output/multi_character/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`残りアンラス: ${result.anlas_remaining}`);
    console.log(`今回消費: ${result.anlas_consumed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleImg2img() {
  console.log("\n=== Image2Image ===");

  try {
    const client = new NovelAIClient();

    const inputImage = "reference/input.png";
    if (!fs.existsSync(inputImage)) {
      console.log(`入力画像が見つかりません: ${inputImage}`);
      return;
    }
    const characters: Schemas.CharacterConfig[] = [
      {
        prompt: "3::cynthia (pokemon) school uniform::,  3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::,2::deep kiss::, 3::saliva on breasts and areolae::",
        center_x: 0.2,
        center_y: 0.5,
        negative_prompt: ""
      },
      {
        prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::,  ",
        center_x: 0.8,
        center_y: 0.5,
        negative_prompt: ""
      },
    ];

    const result = await client.generate({
      prompt: "1girl, beautiful anime girl, detailed eyes, masterpiece",
      action: "img2img",
      source_image: inputImage,
      img2img_strength: 0.8,
      save_dir: "output/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`残りアンラス: ${result.anlas_remaining}`);
    console.log(`今回消費: ${result.anlas_consumed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleImg2imgWithVibes() {
  console.log("\n=== Image2Image + Vibe Transfer ===");

  try {
    const client = new NovelAIClient();

    const inputImage = "reference/input.png";
    const vibeFile = "vibes/えのきっぷ1.naiv4vibe";

    if (!fs.existsSync(inputImage)) {
      console.log(`入力画像が見つかりません: ${inputImage}`);
      return;
    }

    const vibes = fs.existsSync(vibeFile) ? [vibeFile] : undefined;

    const result = await client.generate({
      prompt: "",
      action: "img2img",
      source_image: inputImage,
      img2img_strength: 0.5,
      img2img_noise: 0,
      vibes: vibes,
      vibe_strengths: vibes ? [0.7] : undefined,
      width: 1024,
      height: 1024,
      save_dir: "output/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`残りアンラス: ${result.anlas_remaining}`);
    console.log(`今回消費: ${result.anlas_consumed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleMultiCharacter() {
  console.log("\n=== 複数キャラクター ===");

  try {
    const client = new NovelAIClient();

    const characters: Schemas.CharacterConfig[] = [
      {
        prompt: "3::liko (pokemon) school uniform::,  3::saliva drip::, 2::embarrassed::, large areolae, cleavage, inverted nipples, 3::nude::, -2::loli::,2::deep kiss::, 3::saliva on breasts and areolae::",
        center_x: 0.2,
        center_y: 0.5,
        negative_prompt: ""
      },
      {
        prompt: "2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::,  ",
        center_x: 0.8,
        center_y: 0.5,
        negative_prompt: ""
      },
    ];

    const vibeFiles = [
      "vibes/えのきっぷ1.naiv4vibe",
      "vibes/20251215_231647.naiv4vibe",
      "vibes/漆黒の性王.naiv4vibe",
      "vibes/890bc110faa4_20251231_134734.naiv4vibe",
    ];

    // Filter valid
    const validVibes = vibeFiles.filter(f => fs.existsSync(f));

    const result = await client.generate({
      prompt: "school classroom, sunny day, wide shot, detailed background, 2::face focus::, -3::multiple views::",
      characters: characters,
      vibes: validVibes.length > 0 ? validVibes : undefined,
      vibe_strengths: validVibes.length > 0 ? [0.4, 0.3, 0.5, 0.2].slice(0, validVibes.length) : undefined,
      width: 1280,
      height: 1280,
      save_dir: "output/multi_character/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`残りアンラス: ${result.anlas_remaining}`);
    console.log(`今回消費: ${result.anlas_consumed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleEncodeVibe() {
  console.log("\n=== Vibeエンコード ===");

  try {
    const client = new NovelAIClient();

    const imagePath = "reference/input.png";
    if (!fs.existsSync(imagePath)) {
      console.log(`参照画像が見つかりません: ${imagePath}`);
      return;
    }

    // エンコードのみ（保存なし）
    // const result = await client.encodeVibe({
    //   image: imagePath,
    //   information_extracted: 0.5,
    //   strength: 0.7,
    // });
    // console.log(`✓ Encoded (hash: ${result.source_image_hash.slice(0, 12)}...)`);

    // エンコード + 自動保存
    const resultSaved = await client.encodeVibe({
      image: imagePath,
      save_filename: "input1", 
      save_dir: "./vibes"
    });
    console.log(`✓ Saved: ${resultSaved.saved_path}`);

    console.log(`残りアンラス: ${resultSaved.anlas_remaining}`);
    console.log(`今回消費: ${resultSaved.anlas_consumed}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function exampleCharacterReference() {
  console.log("\n=== キャラクター参照 ===");

  try {
    const client = new NovelAIClient();

    const referenceImage = "reference/input.png";
    if (!fs.existsSync(referenceImage)) {
      console.log(`参照画像が見つかりません: ${referenceImage}`);
      return;
    }

    const result = await client.generate({
      prompt: "school classroom, sunny day, detailed background",
      characters: [
        {
          prompt: "3::peeing::",
          center_x: 0.5,
          center_y: 0.5,
          negative_prompt: ""
        }
      ],
      character_reference: {
        image: referenceImage,
        fidelity: 0.8,
        include_style: true
      },
      save_dir: "output/charref/"
    });

    console.log(`✓ Generated: ${result.saved_path}`);
    console.log(`  Seed: ${result.seed}`);
    console.log(`  残りアンラス: ${result.anlas_remaining}`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.error("❌ バリデーションエラー:");
      e.issues.forEach(issue => {
        console.error(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("Error:", e);
    }
  }
}

async function main() {
  console.log("=".repeat(50));
  console.log("NovelAI Unified Client 使用例 (TypeScript)");
  console.log("=".repeat(50));

  // 実行したい例のコメントを外してください

  // exampleSimpleGenerate();

  // await exampleWithVibes();

  // await exampleImg2img();

  // await exampleImg2imgWithVibes();

  await exampleMultiCharacter();

  // await exampleEncodeVibe();

  // await exampleCharacterReference();

  console.log("\n使用したい例のコード内のコメントを外して実行してください。");
}

main().catch(console.error);
