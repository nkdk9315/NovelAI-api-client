/**
 * NovelAI API Integration Tests
 * 実際にAPIリクエストを行うテスト
 * 
 * 使用方法: pnpm tsx test.ts
 */

import { NovelAIClient } from '../src/client';
import dotenv from 'dotenv';
import fs from 'fs';
import path from 'path';

dotenv.config();

const client = new NovelAIClient();
const INPUT_IMAGE = './reference/input.jpeg';
const OUTPUT_DIR = './output/test/';

// 出力ディレクトリ作成
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

/**
 * Test 1: Img2Img (Image to Image) - 元画像を参照して新しい画像を生成
 */
async function testImg2Img() {
  console.log('\n=== Test: Img2Img ===');
  try {
    const result = await client.generate({
      prompt: '1girl, beautiful, masterpiece',
      action: 'img2img',
      source_image: INPUT_IMAGE,
      img2img_strength: 0.6,
      img2img_noise: 0.1,
      width: 832,
      height: 1216,
      save_dir: OUTPUT_DIR,
    });
    console.log('✅ Img2Img success!');
    console.log(`   Saved to: ${result.saved_path}`);
    console.log(`   Anlas consumed: ${result.anlas_consumed}`);
    return true;
  } catch (error) {
    console.error('❌ Img2Img failed:', error);
    return false;
  }
}

/**
 * Test 2: Infill (マスクのみ) - マスク領域のみを再生成
 * ※ マスク画像が必要（白=再生成エリア, 黒=保持エリア）
 */
async function testInfillOnly() {
  console.log('\n=== Test: Infill (Mask Only) ===');

  // 簡易マスク画像を作成（中央部分を白にする）
  const maskPath = path.join(OUTPUT_DIR, 'temp_mask.png');

  // sharpがなければスキップ
  try {
    const sharp = (await import('sharp')).default;

    // 832x1216 の画像、中央600x800を白、それ以外を黒
    const mask = await sharp({
      create: {
        width: 832,
        height: 1216,
        channels: 4,
        background: { r: 0, g: 0, b: 0, alpha: 255 }
      }
    })
      .composite([{
        input: Buffer.from(
          `<svg width="832" height="1216">
          <rect x="116" y="208" width="600" height="800" fill="white"/>
        </svg>`
        ),
        top: 0,
        left: 0
      }])
      .png()
      .toBuffer();

    fs.writeFileSync(maskPath, mask);
    console.log(`   Created temp mask: ${maskPath}`);

    const result = await client.generate({
      prompt: '1girl, smiling, happy',
      action: 'infill',
      source_image: INPUT_IMAGE,
      mask: maskPath,
      mask_strength: 0.7,  // マスク反映度
      width: 832,
      height: 1216,
      save_dir: OUTPUT_DIR,
    });
    console.log('✅ Infill (Mask Only) success!');
    console.log(`   Saved to: ${result.saved_path}`);
    console.log(`   Anlas consumed: ${result.anlas_consumed}`);

    // 一時ファイル削除
    fs.unlinkSync(maskPath);
    return true;
  } catch (error) {
    console.error('❌ Infill (Mask Only) failed:', error);
    return false;
  }
}

/**
 * Test 3: Infill + Img2Img (ハイブリッドモード) - マスクと元画像参照を同時使用
 */
async function testInfillWithImg2Img() {
  console.log('\n=== Test: Infill + Img2Img (Hybrid Mode) ===');

  const maskPath = path.join(OUTPUT_DIR, 'temp_mask_hybrid.png');

  try {
    const sharp = (await import('sharp')).default;

    // マスク画像作成
    const mask = await sharp({
      create: {
        width: 832,
        height: 1216,
        channels: 4,
        background: { r: 0, g: 0, b: 0, alpha: 255 }
      }
    })
      .composite([{
        input: Buffer.from(
          `<svg width="832" height="1216">
          <rect x="116" y="208" width="600" height="800" fill="white"/>
        </svg>`
        ),
        top: 0,
        left: 0
      }])
      .png()
      .toBuffer();

    fs.writeFileSync(maskPath, mask);
    console.log(`   Created temp mask: ${maskPath}`);

    const result = await client.generate({
      prompt: '1girl, beautiful dress, elegant',
      action: 'infill',
      source_image: INPUT_IMAGE,
      mask: maskPath,
      mask_strength: 0.68,              // マスク反映度
      hybrid_img2img_strength: 0.45,    // 元画像維持率
      hybrid_img2img_noise: 0,          // 元画像ノイズ
      width: 832,
      height: 1216,
      save_dir: OUTPUT_DIR,
    });
    console.log('✅ Infill + Img2Img (Hybrid) success!');
    console.log(`   Saved to: ${result.saved_path}`);
    console.log(`   Anlas consumed: ${result.anlas_consumed}`);

    // 一時ファイル削除
    fs.unlinkSync(maskPath);
    return true;
  } catch (error) {
    console.error('❌ Infill + Img2Img (Hybrid) failed:', error);
    return false;
  }
}

// メイン実行
async function main() {
  console.log('========================================');
  console.log('NovelAI API Integration Tests');
  console.log(`Input Image: ${INPUT_IMAGE}`);
  console.log(`Output Dir: ${OUTPUT_DIR}`);
  console.log('========================================');

  // 入力画像存在チェック
  if (!fs.existsSync(INPUT_IMAGE)) {
    console.error(`❌ Input image not found: ${INPUT_IMAGE}`);
    process.exit(1);
  }

  const results: { name: string; success: boolean }[] = [];

  // Test 1: Img2Img
  results.push({ name: 'Img2Img', success: await testImg2Img() });

  // Test 2: Infill (Mask Only)
  results.push({ name: 'Infill (Mask Only)', success: await testInfillOnly() });

  // Test 3: Infill + Img2Img (Hybrid)
  results.push({ name: 'Infill + Img2Img', success: await testInfillWithImg2Img() });

  // 結果サマリー
  console.log('\n========================================');
  console.log('Test Results Summary');
  console.log('========================================');
  for (const r of results) {
    console.log(`${r.success ? '✅' : '❌'} ${r.name}`);
  }

  const failed = results.filter(r => !r.success).length;
  if (failed > 0) {
    console.log(`\n${failed} test(s) failed.`);
    process.exit(1);
  } else {
    console.log('\nAll tests passed!');
  }
}

main();