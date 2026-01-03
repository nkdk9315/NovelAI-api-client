/**
 * NovelAI Inpaint/Mask機能のテスト例
 * 
 * 元画像の一部をマスクして、その領域のみを再生成します
 */

import { NovelAIClient } from "./src/client";
import * as Utils from "./src/utils";
import fs from "fs";
import path from "path";
import sharp from "sharp";
import dotenv from "dotenv";

// Load environment variables
dotenv.config();

async function main() {
  console.log("=== NovelAI Inpaint/Mask Test ===\n");

  // クライアント初期化
  const client = new NovelAIClient();
  console.log("✓ Client initialized");

  // 出力ディレクトリ
  const outputDir = "./output/inpaint";
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // === テスト1: 既存画像を使用したInpaint ===
  // await testInpaintWithExistingImage(client, outputDir);

  // === テスト2: まず画像を生成してからInpaint ===
  // await testGenerateThenInpaint(client, outputDir);

  // === テスト3: Inpaint + Vibe Transfer + キャラクター設定 ===
  await testInpaintWithComplexFeatures(client, outputDir);
}

/**
 * テスト1: reference フォルダ内の既存画像を使用してInpaint
 */
async function testInpaintWithExistingImage(client: NovelAIClient, outputDir: string) {
  console.log("\n--- Test: Inpaint with Existing Image ---");

  // referenceフォルダ内の画像を探す
  const referenceDir = "./reference";
  if (!fs.existsSync(referenceDir)) {
    console.log("⚠ Reference directory not found. Skipping this test.");
    console.log("  Please create ./reference folder and add an image file.");
    return;
  }

  const files = fs.readdirSync(referenceDir).filter(f => 
    /\.(png|jpg|jpeg|webp)$/i.test(f)
  );

  if (files.length === 0) {
    console.log("⚠ No image files found in reference folder. Skipping.");
    return;
  }

  const sourceImagePath = path.join(referenceDir, files[0]);
  console.log(`✓ Using source image: ${sourceImagePath}`);

  // 画像のサイズを取得
  const sourceBuffer = fs.readFileSync(sourceImagePath);
  const metadata = await sharp(sourceBuffer).metadata();
  const width = metadata.width || 832;
  const height = metadata.height || 1216;

  console.log(`  Image size: ${width}x${height}`);

  // 中央に矩形マスクを作成（画像の中央30%をマスク）
  console.log("✓ Creating rectangular mask (center 30%)...");
  const mask = await Utils.createRectangularMask(width, height, {
    x: 0.35,  // 左から35%の位置
    y: 0.35,  // 上から35%の位置
    w: 0.30,  // 幅30%
    h: 0.30,  // 高さ30%
  });

  // マスク画像も保存（デバッグ用）
  const maskPath = path.join(outputDir, "debug_mask.png");
  fs.writeFileSync(maskPath, mask);
  console.log(`  Debug mask saved to: ${maskPath}`);

  // Inpaint実行
  console.log("✓ Running inpaint...");
  try {
    const result = await client.generate({
      prompt: "3::deep kiss::, 2::tongue out::",
      action: "infill",
      source_image: sourceBuffer,
      mask: mask,
      width: Math.floor(width / 64) * 64,  // 64の倍数に調整
      height: Math.floor(height / 64) * 64,
      inpaint_strength: 0.7,
      inpaint_noise: 0,
      steps: 23,
      scale: 5.0,
      save_dir: outputDir,
    });

    console.log(`✓ Inpaint completed!`);
    console.log(`  Saved to: ${result.saved_path}`);
    console.log(`  Seed: ${result.seed}`);
    console.log(`  Anlas consumed: ${result.anlas_consumed}`);
    console.log(`  Anlas remaining: ${result.anlas_remaining}`);
  } catch (error: any) {
    console.error(`✗ Error: ${error.message}`);
  }
}

/**
 * テスト2: まず画像を生成してから、その一部をInpaintで変更
 */
async function testGenerateThenInpaint(client: NovelAIClient, outputDir: string) {
  console.log("\n--- Test: Generate then Inpaint ---");

  // Step 1: 画像を生成
  console.log("✓ Generating initial image...");
  const initialResult = await client.generate({
    prompt: "1girl, solo, standing, simple background, school uniform",
    width: 832,
    height: 1216,
    steps: 23,
    scale: 5.0,
  });

  const initialPath = path.join(outputDir, `initial_${initialResult.seed}.png`);
  fs.writeFileSync(initialPath, initialResult.image_data);
  console.log(`  Initial image saved to: ${initialPath}`);

  // Step 2: 円形マスクを作成（顔周辺を変更するイメージ）
  console.log("✓ Creating circular mask (upper center)...");
  const mask = await Utils.createCircularMask(832, 1216, {
    x: 0.5,   // 中央
    y: 0.25,  // 上から25%
  }, 0.15);   // 半径15%

  const maskPath = path.join(outputDir, "debug_mask_circle.png");
  fs.writeFileSync(maskPath, mask);
  console.log(`  Debug mask saved to: ${maskPath}`);

  // Step 3: Inpaint実行
  console.log("✓ Running inpaint on generated image...");
  try {
    const inpaintResult = await client.generate({
      prompt: "1girl, solo, standing, simple background, school uniform, smiling, happy",
      action: "infill",
      source_image: initialResult.image_data,
      mask: mask,
      width: 832,
      height: 1216,
      inpaint_strength: 0.7,
      seed: initialResult.seed, // 同じシードを使用
      steps: 23,
      scale: 5.0,
      save_dir: outputDir,
    });

    console.log(`✓ Inpaint completed!`);
    console.log(`  Saved to: ${inpaintResult.saved_path}`);
    console.log(`  Anlas consumed: ${inpaintResult.anlas_consumed}`);
  } catch (error: any) {
    console.error(`✗ Error: ${error.message}`);
  }
}

/**
 * テスト3: Inpaint機能と他の機能（Vibe, Character）の同時利用テスト
 */
async function testInpaintWithComplexFeatures(client: NovelAIClient, outputDir: string) {
  console.log("\n--- Test: Inpaint + Vibes + Characters ---");

  // referenceフォルダ内の画像
  const referenceDir = "./reference";
  const files = fs.existsSync(referenceDir) 
    ? fs.readdirSync(referenceDir).filter(f => /\.(png|jpg|jpeg|webp)$/i.test(f))
    : [];

  if (files.length === 0) {
    console.log("⚠ No image files found in reference folder. Skipping.");
    return;
  }

  const sourceImagePath = path.join(referenceDir, files[0]);
  const sourceBuffer = fs.readFileSync(sourceImagePath);
  const metadata = await sharp(sourceBuffer).metadata();
  const width = metadata.width || 832;
  const height = metadata.height || 1216;

  // マスク作成 (中央部)
  const mask = await Utils.createRectangularMask(width, height, {
    x: 0.35, y: 0.35, w: 0.30, h: 0.30,
  });

  // Vibeファイルの探索
  const vibeFiles = [
    "vibes/えのきっぷ1.naiv4vibe",
    "vibes/漆黒の性王.naiv4vibe",
  ].filter(f => fs.existsSync(f));

  // キャラクター設定
  const characters = [
    {
      prompt: "1girl, solo, holding a umbrella, looking at viewer",
      center_x: 0.5,
      center_y: 0.5,
    }
  ];

  console.log(`✓ Using source: ${sourceImagePath}`);
  console.log(`✓ Vibes: ${vibeFiles.length > 0 ? vibeFiles.join(", ") : "None found"}`);
  console.log(`✓ Characters: ${characters.length}`);

  try {
    const result = await client.generate({
      prompt: "masterpiece, best quality, rainy street, neon lights",
      action: "infill",
      source_image: sourceBuffer,
      mask: mask,
      width: Math.floor(width / 64) * 64,
      height: Math.floor(height / 64) * 64,
      characters: characters,
      vibes: vibeFiles.length > 0 ? vibeFiles : undefined,
      vibe_strengths: vibeFiles.length > 0 ? new Array(vibeFiles.length).fill(0.6) : undefined,
      inpaint_strength: 0.7,
      save_dir: outputDir,
    });

    console.log(`✓ Complex Inpaint completed!`);
    console.log(`  Saved to: ${result.saved_path}`);
    console.log(`  Anlas consumed: ${result.anlas_consumed}`);
  } catch (error: any) {
    console.error(`✗ Error: ${error.message}`);
  }
}

main().catch(console.error);
