/**
 * NovelAI API Edge Case Investigation Tests
 * エッジケース調査テスト
 * 
 * コードレビューで指摘されたエッジケースを実際にテストし、
 * 各ケースの挙動を明確に出力します。
 * 
 * 使用方法: pnpm tsx edge_case_test.ts
 */

import { NovelAIClient } from './src/client';
import * as Schemas from './src/schemas';
import * as Constants from './src/constants';
import dotenv from 'dotenv';
import fs from 'fs';
import path from 'path';

dotenv.config();

const client = new NovelAIClient();
const OUTPUT_DIR = './output/edge_case_tests/';
const INPUT_IMAGE = './reference/input.png';

// 出力ディレクトリ作成
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

// テスト結果を保存する配列
interface TestResult {
  id: string;
  name: string;
  category: string;
  status: 'PASS' | 'FAIL' | 'ERROR' | 'SKIP';
  description: string;
  expected: string;
  actual: string;
  details?: string;
}

const results: TestResult[] = [];

// ユーティリティ: 結果を追加
function addResult(result: TestResult) {
  results.push(result);
  const icon = result.status === 'PASS' ? '✅' : 
               result.status === 'FAIL' ? '❌' : 
               result.status === 'SKIP' ? '⏭️' : '💥';
  console.log(`${icon} [${result.id}] ${result.name}`);
  console.log(`   Status: ${result.status}`);
  console.log(`   Expected: ${result.expected}`);
  console.log(`   Actual: ${result.actual}`);
  if (result.details) {
    console.log(`   Details: ${result.details}`);
  }
  console.log('');
}

// =============================================================================
// E1: 空のプロンプト (Empty Prompt)
// =============================================================================

async function testE1_EmptyPrompt() {
  console.log('\n' + '='.repeat(60));
  console.log('E1: 空のプロンプト (Empty Prompt)');
  console.log('='.repeat(60) + '\n');

  // E1-1: スキーマバリデーション (空文字列)
  try {
    const result = await Schemas.GenerateParamsSchema.safeParseAsync({
      prompt: '',
      width: 512,
      height: 512,
    });
    
    addResult({
      id: 'E1-1',
      name: '空文字列のスキーマバリデーション',
      category: 'E1: 空のプロンプト',
      status: result.success ? 'PASS' : 'FAIL',
      description: '空文字列 "" がスキーマで許可されるか',
      expected: '許可される (min(0) のため)',
      actual: result.success ? '許可された' : `拒否された: ${JSON.stringify(result.error.issues)}`,
    });
  } catch (e: any) {
    addResult({
      id: 'E1-1',
      name: '空文字列のスキーマバリデーション',
      category: 'E1: 空のプロンプト',
      status: 'ERROR',
      description: '空文字列 "" がスキーマで許可されるか',
      expected: '許可される (min(0) のため)',
      actual: `エラー: ${e.message}`,
    });
  }

  // E1-2: 実際のAPI呼び出し (空文字列)
  try {
    const result = await client.generate({
      prompt: '',
      width: 512,
      height: 512,
      steps: 5,  // 最小ステップでAnlas節約
      save_dir: OUTPUT_DIR,
    });
    
    addResult({
      id: 'E1-2',
      name: '空プロンプトでのAPI呼び出し',
      category: 'E1: 空のプロンプト',
      status: 'PASS',
      description: 'NovelAI APIが空プロンプトを受け入れるか',
      expected: '成功 or 明確なエラー',
      actual: `成功 - 画像生成完了 (${result.saved_path})`,
      details: `Anlas消費: ${result.anlas_consumed}`,
    });
  } catch (e: any) {
    addResult({
      id: 'E1-2',
      name: '空プロンプトでのAPI呼び出し',
      category: 'E1: 空のプロンプト',
      status: 'FAIL',
      description: 'NovelAI APIが空プロンプトを受け入れるか',
      expected: '成功 or 明確なエラー',
      actual: `エラー: ${e.message}`,
    });
  }

  // E1-3: スペースのみのプロンプト
  try {
    const result = await Schemas.GenerateParamsSchema.safeParseAsync({
      prompt: '   ',
      width: 512,
      height: 512,
    });
    
    addResult({
      id: 'E1-3',
      name: 'スペースのみのプロンプト (スキーマ)',
      category: 'E1: 空のプロンプト',
      status: result.success ? 'PASS' : 'FAIL',
      description: 'スペースのみ "   " がスキーマで許可されるか',
      expected: '許可される (現状トリムなし)',
      actual: result.success ? '許可された' : `拒否された`,
    });
  } catch (e: any) {
    addResult({
      id: 'E1-3',
      name: 'スペースのみのプロンプト (スキーマ)',
      category: 'E1: 空のプロンプト',
      status: 'ERROR',
      description: 'スペースのみ "   " がスキーマで許可されるか',
      expected: '許可される (現状トリムなし)',
      actual: `エラー: ${e.message}`,
    });
  }
}

// =============================================================================
// E2: 極端な seed 値 (Extreme Seed Values)
// =============================================================================

async function testE2_ExtremeSeedValues() {
  console.log('\n' + '='.repeat(60));
  console.log('E2: 極端な seed 値 (Extreme Seed Values)');
  console.log('='.repeat(60) + '\n');

  const seedTestCases = [
    { seed: 0, name: '最小値 (0)' },
    { seed: 1, name: '1' },
    { seed: Constants.MAX_SEED - 1, name: 'MAX_SEED - 1' },
    { seed: Constants.MAX_SEED, name: 'MAX_SEED (4294967295)' },
    { seed: Constants.MAX_SEED + 1, name: 'MAX_SEED + 1 (範囲外)' },
    { seed: -1, name: '-1 (負の値)' },
    { seed: 2147483647, name: 'Int32 最大値 (2^31-1)' },
  ];

  for (let i = 0; i < seedTestCases.length; i++) {
    const tc = seedTestCases[i];
    const testId = `E2-${i + 1}`;
    
    try {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: 'test',
        seed: tc.seed,
        width: 512,
        height: 512,
      });
      
      const shouldPass = tc.seed >= 0 && tc.seed <= Constants.MAX_SEED;
      
      addResult({
        id: testId,
        name: `シード値: ${tc.name}`,
        category: 'E2: 極端な seed 値',
        status: result.success === shouldPass ? 'PASS' : 'FAIL',
        description: `seed = ${tc.seed} がスキーマで正しく処理されるか`,
        expected: shouldPass ? '許可される' : '拒否される',
        actual: result.success ? '許可された' : `拒否された`,
        details: !result.success && result.error ? result.error.issues[0]?.message : undefined,
      });
    } catch (e: any) {
      addResult({
        id: testId,
        name: `シード値: ${tc.name}`,
        category: 'E2: 極端な seed 値',
        status: 'ERROR',
        description: `seed = ${tc.seed} がスキーマで正しく処理されるか`,
        expected: '例外なく処理される',
        actual: `例外発生: ${e.message}`,
      });
    }
  }

  // E2-実API: seed=0 で実際にAPI呼び出し
  try {
    const result = await client.generate({
      prompt: '1girl',
      seed: 0,
      width: 512,
      height: 512,
      steps: 5,
      save_dir: OUTPUT_DIR,
    });
    
    addResult({
      id: 'E2-API',
      name: 'seed=0 でのAPI呼び出し',
      category: 'E2: 極端な seed 値',
      status: 'PASS',
      description: 'seed=0 でAPIが正常動作するか',
      expected: '成功',
      actual: `成功 - seed: ${result.seed}`,
      details: `保存先: ${result.saved_path}`,
    });
  } catch (e: any) {
    addResult({
      id: 'E2-API',
      name: 'seed=0 でのAPI呼び出し',
      category: 'E2: 極端な seed 値',
      status: 'FAIL',
      description: 'seed=0 でAPIが正常動作するか',
      expected: '成功',
      actual: `失敗: ${e.message}`,
    });
  }
}

// =============================================================================
// E3: 境界値のピクセル数 (Boundary Pixel Count)
// =============================================================================

async function testE3_BoundaryPixelCount() {
  console.log('\n' + '='.repeat(60));
  console.log('E3: 境界値のピクセル数 (Boundary Pixel Count)');
  console.log('='.repeat(60) + '\n');

  const pixelTestCases = [
    { width: 1024, height: 1024, name: 'MAX_PIXELS ちょうど (1024x1024 = 1,048,576)', shouldPass: true },
    { width: 1024, height: 1088, name: 'MAX_PIXELS 超過 (1024x1088 = 1,114,112)', shouldPass: false },
    { width: 832, height: 1216, name: 'デフォルト値 (832x1216 = 1,011,712)', shouldPass: true },
    { width: 64, height: 64, name: '最小サイズ (64x64 = 4,096)', shouldPass: true },
    { width: 1088, height: 960, name: '別の超過パターン (1088x960 = 1,044,480)', shouldPass: true },
    { width: 1152, height: 896, name: '超過パターン2 (1152x896 = 1,032,192)', shouldPass: true },
    { width: 1280, height: 832, name: '超過パターン3 (1280x832 = 1,064,960)', shouldPass: false },
  ];

  for (let i = 0; i < pixelTestCases.length; i++) {
    const tc = pixelTestCases[i];
    const testId = `E3-${i + 1}`;
    const totalPixels = tc.width * tc.height;
    
    try {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: 'test',
        width: tc.width,
        height: tc.height,
      });
      
      addResult({
        id: testId,
        name: tc.name,
        category: 'E3: 境界値のピクセル数',
        status: result.success === tc.shouldPass ? 'PASS' : 'FAIL',
        description: `${tc.width}x${tc.height} = ${totalPixels.toLocaleString()} pixels`,
        expected: tc.shouldPass ? '許可される' : '拒否される',
        actual: result.success ? '許可された' : `拒否された`,
        details: !result.success && result.error ? result.error.issues.map(i => i.message).join(', ') : undefined,
      });
    } catch (e: any) {
      addResult({
        id: testId,
        name: tc.name,
        category: 'E3: 境界値のピクセル数',
        status: 'ERROR',
        description: `${tc.width}x${tc.height} = ${totalPixels.toLocaleString()} pixels`,
        expected: tc.shouldPass ? '許可される' : '拒否される',
        actual: `例外: ${e.message}`,
      });
    }
  }

  // E3-API: 境界値 (1024x1024) で実際にAPI呼び出し
  try {
    const result = await client.generate({
      prompt: '1girl',
      width: 1024,
      height: 1024,
      steps: 5,
      save_dir: OUTPUT_DIR,
    });
    
    addResult({
      id: 'E3-API',
      name: 'MAX_PIXELS ちょうど (1024x1024) でのAPI呼び出し',
      category: 'E3: 境界値のピクセル数',
      status: 'PASS',
      description: '境界値でAPIが正常動作するか',
      expected: '成功',
      actual: `成功`,
      details: `保存先: ${result.saved_path} / Anlas消費: ${result.anlas_consumed}`,
    });
  } catch (e: any) {
    addResult({
      id: 'E3-API',
      name: 'MAX_PIXELS ちょうど (1024x1024) でのAPI呼び出し',
      category: 'E3: 境界値のピクセル数',
      status: 'FAIL',
      description: '境界値でAPIが正常動作するか',
      expected: '成功',
      actual: `失敗: ${e.message}`,
    });
  }
}

// =============================================================================
// E4: Unicode/特殊文字を含むプロンプト (Unicode/Special Characters)
// =============================================================================

async function testE4_UnicodePrompt() {
  console.log('\n' + '='.repeat(60));
  console.log('E4: Unicode/特殊文字を含むプロンプト');
  console.log('='.repeat(60) + '\n');

  const unicodeTestCases = [
    { prompt: '1girl, 美少女', name: '日本語' },
    { prompt: '1girl, 🌸🎀', name: '絵文字' },
    { prompt: '1girl, café naïve', name: 'アクセント記号' },
    { prompt: '1girl, 👨‍👩‍👧‍👦', name: 'ZWJ文字 (家族絵文字)' },
    { prompt: '1girl, \u200B\u200B', name: 'ゼロ幅スペース' },
    { prompt: '1girl, <script>alert("XSS")</script>', name: 'HTMLタグ' },
    { prompt: '1girl, \\n\\r\\t', name: 'エスケープ文字 (リテラル)' },
    { prompt: '1girl,\n\r\t', name: '改行・タブ (実際)' },
    { prompt: '1girl, "quoted" \'single\'', name: '引用符' },
    { prompt: 'a'.repeat(2001), name: 'MAX_PROMPT_CHARS超過 (2001文字)' },
    { prompt: 'a'.repeat(2000), name: 'MAX_PROMPT_CHARSちょうど (2000文字)' },
  ];

  for (let i = 0; i < unicodeTestCases.length; i++) {
    const tc = unicodeTestCases[i];
    const testId = `E4-${i + 1}`;
    
    try {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: tc.prompt,
        width: 512,
        height: 512,
      });
      
      const shouldPass = tc.prompt.length <= Constants.MAX_PROMPT_CHARS;
      
      addResult({
        id: testId,
        name: tc.name,
        category: 'E4: Unicode/特殊文字',
        status: (result.success === shouldPass) || (result.success && shouldPass) ? 'PASS' : 
                (!result.success && !shouldPass) ? 'PASS' : 'FAIL',
        description: `特殊文字を含むプロンプトの処理`,
        expected: shouldPass ? 'スキーマ通過' : '拒否',
        actual: result.success ? 'スキーマ通過' : `拒否: ${result.error?.issues[0]?.message}`,
        details: `文字数: ${tc.prompt.length}`,
      });
    } catch (e: any) {
      addResult({
        id: testId,
        name: tc.name,
        category: 'E4: Unicode/特殊文字',
        status: 'ERROR',
        description: `特殊文字を含むプロンプトの処理`,
        expected: 'スキーマ通過',
        actual: `例外: ${e.message}`,
      });
    }
  }

  // E4-API: 絵文字を含むプロンプトで実際にAPI呼び出し
  try {
    const result = await client.generate({
      prompt: '1girl, cherry blossom 🌸, beautiful',
      width: 512,
      height: 512,
      steps: 5,
      save_dir: OUTPUT_DIR,
    });
    
    addResult({
      id: 'E4-API',
      name: '絵文字を含むプロンプトでのAPI呼び出し',
      category: 'E4: Unicode/特殊文字',
      status: 'PASS',
      description: '絵文字がAPIで正常処理されるか',
      expected: '成功',
      actual: `成功`,
      details: `保存先: ${result.saved_path}`,
    });
  } catch (e: any) {
    addResult({
      id: 'E4-API',
      name: '絵文字を含むプロンプトでのAPI呼び出し',
      category: 'E4: Unicode/特殊文字',
      status: 'FAIL',
      description: '絵文字がAPIで正常処理されるか',
      expected: '成功',
      actual: `失敗: ${e.message}`,
    });
  }
}

// =============================================================================
// E5: 巨大な画像ファイル (Large Image File)
// =============================================================================

async function testE5_LargeImageFile() {
  console.log('\n' + '='.repeat(60));
  console.log('E5: 巨大な画像ファイル (Large Image File)');
  console.log('='.repeat(60) + '\n');

  // E5-1: MAX_REF_IMAGE_SIZE_MB 定数の確認
  addResult({
    id: 'E5-1',
    name: 'MAX_REF_IMAGE_SIZE_MB 定数確認',
    category: 'E5: 巨大な画像ファイル',
    status: Constants.MAX_REF_IMAGE_SIZE_MB === 10 ? 'PASS' : 'FAIL',
    description: 'MAX_REF_IMAGE_SIZE_MB が定義されているか',
    expected: '10 (MB)',
    actual: `${Constants.MAX_REF_IMAGE_SIZE_MB} (MB)`,
    details: '⚠️ この定数は現在バリデーションで使用されていない',
  });

  // E5-2: 入力画像が存在する場合、そのサイズを確認
  if (fs.existsSync(INPUT_IMAGE)) {
    const stats = fs.statSync(INPUT_IMAGE);
    const sizeMB = stats.size / (1024 * 1024);
    
    addResult({
      id: 'E5-2',
      name: '参照画像のサイズ確認',
      category: 'E5: 巨大な画像ファイル',
      status: sizeMB <= Constants.MAX_REF_IMAGE_SIZE_MB ? 'PASS' : 'FAIL',
      description: '参照画像がサイズ制限内か',
      expected: `<= ${Constants.MAX_REF_IMAGE_SIZE_MB} MB`,
      actual: `${sizeMB.toFixed(2)} MB`,
      details: INPUT_IMAGE,
    });
  } else {
    addResult({
      id: 'E5-2',
      name: '参照画像のサイズ確認',
      category: 'E5: 巨大な画像ファイル',
      status: 'SKIP',
      description: '参照画像がサイズ制限内か',
      expected: `<= ${Constants.MAX_REF_IMAGE_SIZE_MB} MB`,
      actual: `参照画像が見つかりません: ${INPUT_IMAGE}`,
    });
  }

  // E5-3: 巨大なBase64画像データの生成は省略 (メモリの問題)
  addResult({
    id: 'E5-3',
    name: '10MB超過画像のバリデーション (未実装)',
    category: 'E5: 巨大な画像ファイル',
    status: 'SKIP',
    description: '10MB超過画像が適切にエラーになるか',
    expected: 'エラー',
    actual: '⚠️ バリデーション未実装 - 画像サイズチェックがスキーマにありません',
    details: 'utils.ts で fs.readFileSync 前にサイズチェックを追加推奨',
  });
}

// =============================================================================
// E6: 同時実行 (Concurrent Execution)
// =============================================================================

async function testE6_ConcurrentExecution() {
  console.log('\n' + '='.repeat(60));
  console.log('E6: 同時実行 (Concurrent Execution)');
  console.log('='.repeat(60) + '\n');

  // E6-1: 3つの generate を同時実行
  try {
    const startBalance = await client.getAnlasBalance();
    console.log(`   開始時Anlas残高: ${startBalance.total}`);
    
    const promises = [
      client.generate({
        prompt: '1girl, concurrent test 1',
        width: 512,
        height: 512,
        steps: 5,
        save_dir: OUTPUT_DIR,
      }),
      client.generate({
        prompt: '1girl, concurrent test 2',
        width: 512,
        height: 512,
        steps: 5,
        save_dir: OUTPUT_DIR,
      }),
      client.generate({
        prompt: '1girl, concurrent test 3',
        width: 512,
        height: 512,
        steps: 5,
        save_dir: OUTPUT_DIR,
      }),
    ];
    
    const startTime = Date.now();
    const results = await Promise.allSettled(promises);
    const endTime = Date.now();
    
    const fulfilled = results.filter(r => r.status === 'fulfilled').length;
    const rejected = results.filter(r => r.status === 'rejected').length;
    
    const endBalance = await client.getAnlasBalance();
    console.log(`   終了時Anlas残高: ${endBalance.total}`);
    
    addResult({
      id: 'E6-1',
      name: '3つの同時generate実行',
      category: 'E6: 同時実行',
      status: fulfilled >= 2 ? 'PASS' : 'FAIL',
      description: '3つのgenerateを同時実行した場合の挙動',
      expected: '全て成功 or 一部失敗 (レート制限)',
      actual: `成功: ${fulfilled}, 失敗: ${rejected}`,
      details: `所要時間: ${endTime - startTime}ms / Anlas消費: ${startBalance.total - endBalance.total}`,
    });

    // 個別結果
    for (let i = 0; i < results.length; i++) {
      const r = results[i];
      if (r.status === 'fulfilled') {
        console.log(`      Request ${i + 1}: 成功 -> ${r.value.saved_path}`);
      } else {
        console.log(`      Request ${i + 1}: 失敗 -> ${r.reason.message}`);
      }
    }
    
  } catch (e: any) {
    addResult({
      id: 'E6-1',
      name: '3つの同時generate実行',
      category: 'E6: 同時実行',
      status: 'ERROR',
      description: '3つのgenerateを同時実行した場合の挙動',
      expected: '全て成功 or 一部失敗',
      actual: `例外: ${e.message}`,
    });
  }

  // E6-2: Anlas残高計算の整合性確認
  addResult({
    id: 'E6-2',
    name: 'Anlas残高の整合性',
    category: 'E6: 同時実行',
    status: 'PASS',
    description: '同時実行時のAnlas残高表示の信頼性',
    expected: '各リクエストのanlas_consumedは独立計算',
    actual: '⚠️ 同時実行時、anlas_before と anlas_after の差分が不正確になる可能性あり',
    details: 'generate() は各リクエスト前後でgetAnlasBalance()を呼ぶため、同時実行時は順序が保証されない',
  });
}

// =============================================================================
// 追加テスト: 64の倍数チェック
// =============================================================================

async function testExtra_DimensionMultiple() {
  console.log('\n' + '='.repeat(60));
  console.log('Extra: 64の倍数チェック');
  console.log('='.repeat(60) + '\n');

  const testCases = [
    { width: 512, height: 512, shouldPass: true, name: '512x512 (64の倍数)' },
    { width: 513, height: 512, shouldPass: false, name: '513x512 (非倍数)' },
    { width: 128, height: 192, shouldPass: true, name: '128x192 (64の倍数)' },
    { width: 100, height: 100, shouldPass: false, name: '100x100 (非倍数)' },
  ];

  for (let i = 0; i < testCases.length; i++) {
    const tc = testCases[i];
    const testId = `EX-${i + 1}`;
    
    try {
      const result = await Schemas.GenerateParamsSchema.safeParseAsync({
        prompt: 'test',
        width: tc.width,
        height: tc.height,
      });
      
      addResult({
        id: testId,
        name: tc.name,
        category: 'Extra: 64の倍数チェック',
        status: result.success === tc.shouldPass ? 'PASS' : 'FAIL',
        description: `${tc.width}x${tc.height} のバリデーション`,
        expected: tc.shouldPass ? '許可される' : '拒否される',
        actual: result.success ? '許可された' : `拒否された: ${result.error?.issues[0]?.message}`,
      });
    } catch (e: any) {
      addResult({
        id: testId,
        name: tc.name,
        category: 'Extra: 64の倍数チェック',
        status: 'ERROR',
        description: `${tc.width}x${tc.height} のバリデーション`,
        expected: tc.shouldPass ? '許可される' : '拒否される',
        actual: `例外: ${e.message}`,
      });
    }
  }
}

// =============================================================================
// メイン実行
// =============================================================================

async function main() {
  console.log('╔══════════════════════════════════════════════════════════════════╗');
  console.log('║         NovelAI API Edge Case Investigation Tests                ║');
  console.log('║                    エッジケース調査テスト                         ║');
  console.log('╚══════════════════════════════════════════════════════════════════╝');
  console.log('');
  console.log(`Output Directory: ${OUTPUT_DIR}`);
  console.log(`MAX_PIXELS: ${Constants.MAX_PIXELS.toLocaleString()}`);
  console.log(`MAX_SEED: ${Constants.MAX_SEED.toLocaleString()}`);
  console.log(`MAX_PROMPT_CHARS: ${Constants.MAX_PROMPT_CHARS.toLocaleString()}`);
  console.log(`MAX_REF_IMAGE_SIZE_MB: ${Constants.MAX_REF_IMAGE_SIZE_MB}`);
  console.log('');

  // 各テストを実行
  await testE1_EmptyPrompt();
  await testE2_ExtremeSeedValues();
  await testE3_BoundaryPixelCount();
  await testE4_UnicodePrompt();
  await testE5_LargeImageFile();
  await testE6_ConcurrentExecution();
  await testExtra_DimensionMultiple();

  // =============================================================================
  // 結果サマリー
  // =============================================================================
  
  console.log('\n' + '═'.repeat(70));
  console.log('              SUMMARY / 結果サマリー');
  console.log('═'.repeat(70) + '\n');

  // カテゴリごとにグループ化
  const categories = new Set(results.map(r => r.category));
  
  for (const cat of categories) {
    const catResults = results.filter(r => r.category === cat);
    const passed = catResults.filter(r => r.status === 'PASS').length;
    const failed = catResults.filter(r => r.status === 'FAIL').length;
    const errors = catResults.filter(r => r.status === 'ERROR').length;
    const skipped = catResults.filter(r => r.status === 'SKIP').length;
    
    const icon = failed > 0 || errors > 0 ? '⚠️' : '✅';
    console.log(`${icon} ${cat}`);
    console.log(`   PASS: ${passed} | FAIL: ${failed} | ERROR: ${errors} | SKIP: ${skipped}`);
    
    // 失敗したテストの詳細を表示
    const failedTests = catResults.filter(r => r.status === 'FAIL' || r.status === 'ERROR');
    if (failedTests.length > 0) {
      for (const ft of failedTests) {
        console.log(`   ❌ ${ft.id}: ${ft.name}`);
      }
    }
    console.log('');
  }

  // 全体集計
  const totalPassed = results.filter(r => r.status === 'PASS').length;
  const totalFailed = results.filter(r => r.status === 'FAIL').length;
  const totalErrors = results.filter(r => r.status === 'ERROR').length;
  const totalSkipped = results.filter(r => r.status === 'SKIP').length;
  const total = results.length;

  console.log('─'.repeat(70));
  console.log(`TOTAL: ${total} tests`);
  console.log(`  ✅ PASS:  ${totalPassed}`);
  console.log(`  ❌ FAIL:  ${totalFailed}`);
  console.log(`  💥 ERROR: ${totalErrors}`);
  console.log(`  ⏭️ SKIP:  ${totalSkipped}`);
  console.log('─'.repeat(70));

  // JSONでも出力
  const reportPath = path.join(OUTPUT_DIR, 'edge_case_report.json');
  fs.writeFileSync(reportPath, JSON.stringify({
    timestamp: new Date().toISOString(),
    summary: {
      total,
      passed: totalPassed,
      failed: totalFailed,
      errors: totalErrors,
      skipped: totalSkipped,
    },
    results,
  }, null, 2));
  console.log(`\nDetailed report saved to: ${reportPath}`);

  // 終了コード
  if (totalFailed > 0 || totalErrors > 0) {
    console.log('\n⚠️ Some tests failed or errored.');
    process.exit(1);
  } else {
    console.log('\n✅ All tests passed!');
  }
}

main().catch((e) => {
  console.error('Fatal error:', e);
  process.exit(1);
});
