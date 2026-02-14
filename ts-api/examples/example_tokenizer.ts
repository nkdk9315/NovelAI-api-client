/// <reference types="node" />
/**
 * NovelAI Tokenizer 使用例
 * 
 * このファイルはTokenizerの使い方を示すサンプルです。
 * 
 * 実行方法:
 *   pnpm exec tsx example_tokenizer.ts
 *   pnpm exec tsx example_tokenizer.ts "your custom prompt here"
 */

import {
    getClipTokenizer,
    getT5Tokenizer,
    preprocessT5,
    NovelAIClipTokenizer,
} from '../src/tokenizer';
import { Tokenizer } from 'tokenizers';

// =============================================================================
// CLIP Tokenizer の使用例
// =============================================================================

/**
 * CLIP Tokenizer
 * - プロンプトの「生の」トークン数をカウント
 * - 重み付け構文 (e.g., {beautiful:1.2}) を含む
 * - NovelAI の画像生成で使用される最大トークン数の確認に便利
 */
async function clipTokenizerExample(text: string): Promise<void> {
    console.log('\n=== CLIP Tokenizer ===');
    console.log(`Input: "${text}"`);

    // トークナイザーを取得（初回はサーバーからフェッチ、以降はキャッシュ）
    const tokenizer: NovelAIClipTokenizer = await getClipTokenizer();

    // テキストをトークン化
    const tokens: number[] = tokenizer.encode(text);

    console.log(`Token IDs: [${tokens.slice(0, 10).join(', ')}${tokens.length > 10 ? ', ...' : ''}]`);
    console.log(`Token Count: ${tokens.length}`);
}

// =============================================================================
// T5 Tokenizer の使用例
// =============================================================================

/**
 * T5 Tokenizer
 * - 「実効トークン数」をカウント（重み付け構文を除去後）
 * - NovelAI v4 で使用される T5 エンコーダー用
 * - preprocessT5() で前処理してからエンコード
 */
async function t5TokenizerExample(text: string): Promise<void> {
    console.log('\n=== T5 Tokenizer ===');
    console.log(`Input: "${text}"`);

    // 1. 品質タグを追加（NovelAI の標準的なプリフィックス）
    // NOTE: 公式実装では、前処理の前に品質タグが結合されるようです。
    const textWithTags = "masterpiece, best quality, " + text;
    console.log(`With tags: "${textWithTags}"`);

    // 2. トークナイザーを取得
    // getT5Tokenizer() は現在、公式ロジックをカプセル化した NovelAIT5Tokenizer を返します。
    // これにより、内部で preprocessT5() が実行され、EOSトークンも自動的に追加されます。
    const tokenizer = await getT5Tokenizer();

    // 3. エンコード
    // 戻り値は number[] (Token IDs) です。
    const ids = await tokenizer.encode(textWithTags);

    console.log(`Token IDs: [${ids.slice(0, 10).join(', ')}${ids.length > 10 ? ', ...' : ''}]`);
    console.log(`Effective Token Count: ${ids.length}`);
}

// =============================================================================
// キャッシュの動作確認
// =============================================================================

async function cacheExample(): Promise<void> {
    console.log('\n=== Cache Behavior ===');

    console.log('First call (fetches from server)...');
    const start1 = Date.now();
    await getClipTokenizer();
    console.log(`Time: ${Date.now() - start1}ms`);

    console.log('Second call (uses cache)...');
    const start2 = Date.now();
    await getClipTokenizer();
    console.log(`Time: ${Date.now() - start2}ms`);

    console.log('Force refresh (fetches again)...');
    const start3 = Date.now();
    await getClipTokenizer(true);
    console.log(`Time: ${Date.now() - start3}ms`);
}

async function countTokensExample() {
    const tokenizer = await getT5Tokenizer();

    // countTokens() を使用（公式UIと一致）
    const count1 = await tokenizer.countTokens("3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}");
    console.log(`Token count: ${count1} Expected: 25`);

    const count2 = await tokenizer.countTokens("1girl, graphite (medium), plaid background, from side, cowboy shot, stuffed animal, stuffed lion, mimikaki, candle, offering hand");
    console.log(`Token count: ${count2} Expected: 38`);

    const count3 = await tokenizer.countTokens("2::girls::, 2::smile, standing, ::, {{ scared }}, 3::sitting::, 3::spread arms, spread wings::");
    console.log(`Token count: ${count3} Expected: 19`);
}
// =============================================================================
// メイン処理
// =============================================================================

(async () => {
    // コマンドライン引数からテキストを取得、なければデフォルト値
    const args = process.argv.slice(2);
    const sampleText = args.length > 0
        ? args.join(' ')
        : '1girl, {beautiful:1.2}, [masterpiece], blonde hair, blue eyes';

    console.log('========================================');
    console.log('NovelAI Tokenizer Example');
    console.log('========================================');

    try {
        // CLIP Tokenizer のデモ
        await clipTokenizerExample(sampleText);

        // T5 Tokenizer のデモ
        await t5TokenizerExample(sampleText);

        // キャッシュ動作のデモ
        await cacheExample();

        // countTokens() のデモ
        await countTokensExample();

        console.log('\n✅ All examples completed successfully!');
    } catch (error) {
        console.error('\n❌ Error:', error);
        process.exit(1);
    }
})();

