/**
 * NovelAI Augment & Upscale API 使用例
 * 
 * 画像加工ツール（カラー化、表情変換、スケッチ化など）とアップスケール機能のサンプルコード
 * 
 * 注意: width/height は画像から自動検出されるため、指定不要です
 */
import { NovelAIClient } from '../src/client';
import path from 'path';
import dotenv from 'dotenv';

// .envファイルを読み込み
dotenv.config();

const OUTPUT_DIR = path.join(__dirname, '..', 'output', 'augment');

async function main() {
  const client = new NovelAIClient();

  // アンラス残高を確認
  const balance = await client.getAnlasBalance();
  console.log(`\n📊 現在のアンラス残高: ${balance.total}`);
  console.log(`   (固定: ${balance.fixed}, 購入済み: ${balance.purchased})\n`);

  // =====================================================
  // 1. カラー化 (colorize)
  // =====================================================
  console.log('🎨 カラー化テスト...');
  try {
    const colorizeResult = await client.augmentImage({
      req_type: "colorize",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),  // モノクロ画像
      // width/height は自動検出
      prompt: "vibrant colors, detailed shading",  // カラー化のヒント
      defry: 3,  // 中程度の変更 (0=最強, 5=最弱)
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${colorizeResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${colorizeResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 2. 表情変換 (emotion)
  // =====================================================
  console.log('\n😊 表情変換テスト...');

  // 利用可能な表情キーワード:
  // neutral, happy, sad, angry, scared, surprised, tired, excited,
  // nervous, thinking, confused, shy, disgusted, smug, bored,
  // laughing, irritated, aroused, embarrassed, love, worried,
  // determined, hurt, playful

  try {
    const emotionResult = await client.augmentImage({
      req_type: "emotion",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),  // 顔画像
      // width/height は自動検出
      prompt: "happy",  // 表情キーワード
      defry: 0,  // 最強の変更
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${emotionResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${emotionResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 3. スケッチ化 (sketch)
  // =====================================================
  console.log('\n✏️ スケッチ化テスト...');
  try {
    const sketchResult = await client.augmentImage({
      req_type: "sketch",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),
      // width/height は自動検出
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${sketchResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${sketchResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 4. 線画抽出 (lineart)
  // =====================================================
  console.log('\n📝 線画抽出テスト...');
  try {
    const lineartResult = await client.augmentImage({
      req_type: "lineart",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),
      // width/height は自動検出
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${lineartResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${lineartResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 5. デクラッター (declutter)
  // =====================================================
  console.log('\n🧹 デクラッターテスト...');
  try {
    const declutterResult = await client.augmentImage({
      req_type: "declutter",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),
      // width/height は自動検出
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${declutterResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${declutterResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 6. 背景除去 (bg-removal) - 常にアンラス消費
  // =====================================================
  console.log('\n🖼️ 背景除去テスト（⚠️ 常にアンラス消費）...');
  try {
    const bgRemovalResult = await client.augmentImage({
      req_type: "bg-removal",
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),
      // width/height は自動検出
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${bgRemovalResult.saved_path}`);
    console.log(`   💰 消費アンラス: ${bgRemovalResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // =====================================================
  // 7. アップスケール (upscale) - 常にアンラス消費
  // =====================================================
  console.log('\n🔍 アップスケールテスト（⚠️ 常にアンラス消費）...');
  try {
    const upscaleResult = await client.upscaleImage({
      image: path.join(__dirname, '..', 'reference', 'input.jpeg'),
      // width/height は自動検出
      scale: 4,  // 4倍拡大 (2 or 4)
      save_dir: OUTPUT_DIR,
    });
    console.log(`   ✅ 保存先: ${upscaleResult.saved_path}`);
    console.log(`   📐 出力サイズ: ${upscaleResult.output_width}x${upscaleResult.output_height}`);
    console.log(`   💰 消費アンラス: ${upscaleResult.anlas_consumed ?? 'N/A'}`);
  } catch (error) {
    console.log(`   ❌ エラー: ${error}`);
  }

  // 最終アンラス残高を確認
  const finalBalance = await client.getAnlasBalance();
  console.log(`\n📊 最終アンラス残高: ${finalBalance.total}`);
  console.log(`   総消費: ${balance.total - finalBalance.total}\n`);
}

main().catch(console.error);
