import * as Schemas from '../src/schemas';
import { ZodError } from 'zod';
import { validateTokenCount, TokenValidationError, getT5Tokenizer } from '../src/tokenizer';
import type { GenerateParams } from '../src/schemas';

(async () => {
  console.log("=== バリデーションテスト ===\n");

  // Test 1: 1216×832 (OK)
  console.log("Test 1: 1216×832");
  try {
    const params = await Schemas.GenerateParamsSchema.parseAsync({
      prompt: "test",
      width: 1216,
      height: 832,
    }) as GenerateParams;
    console.log("✅ OK - バリデーション成功");
    console.log(`   総ピクセル数: ${1216 * 832} = ${params.width! * params.height!}\n`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.log("❌ NG - バリデーションエラー:");
      e.issues.forEach(issue => {
        console.log(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  // Test 2: 1280×1280 (NG)
  console.log("\nTest 2: 1280×1280");
  try {
    const params = await Schemas.GenerateParamsSchema.parseAsync({
      prompt: "test",
      width: 1280,
      height: 1280,
    }) as GenerateParams;
    console.log("✅ OK - バリデーション成功");
    console.log(`   総ピクセル数: ${params.width! * params.height!}\n`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.log("❌ NG - バリデーションエラー:");
      e.issues.forEach(issue => {
        console.log(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  // Test 3: 1024×1024 (OK - ちょうど限界値)
  console.log("\nTest 3: 1024×1024");
  try {
    const params = await Schemas.GenerateParamsSchema.parseAsync({
      prompt: "test",
      width: 1024,
      height: 1024,
    }) as GenerateParams;
    console.log("✅ OK - バリデーション成功");
    console.log(`   総ピクセル数: ${params.width! * params.height!}\n`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.log("❌ NG - バリデーションエラー:");
      e.issues.forEach(issue => {
        console.log(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  // ===== トークン数バリデーションテスト =====
  console.log("\n=== トークン数バリデーションテスト ===\n");

  // まずトークナイザーをプリロード
  console.log("トークナイザーをプリロード中...");
  const tokenizer = await getT5Tokenizer();
  console.log("トークナイザー準備完了\n");

  // Test 4: プロンプトが512トークン以下 (OK)
  console.log("Test 4: 短いプロンプト (512トークン以下)");
  try {
    const shortPrompt = "a beautiful landscape with mountains and rivers";
    const tokenCount = await tokenizer.countTokens(shortPrompt);
    console.log(`   プロンプトのトークン数: ${tokenCount}`);
    
    const params = await Schemas.GenerateParamsSchema.parseAsync({
      prompt: shortPrompt,
    }) as GenerateParams;
    console.log("✅ OK - バリデーション成功");
    console.log(`   プロンプト: "${shortPrompt}"\n`);
  } catch (e) {
    if (e instanceof ZodError) {
      console.log("❌ NG - バリデーションエラー:");
      e.issues.forEach(issue => {
        console.log(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  // Test 5: プロンプトが512トークンを超える (NG)
  console.log("\nTest 5: 長すぎるプロンプト (512トークン超過)");
  try {
    // 512トークンを確実に超えるように非常に長いプロンプトを作成
    // 各単語がほぼ1トークンなので、600単語以上で確実に超える
    const longPrompt = Array(600).fill("masterpiece beautiful detailed anime girl").join(", ");
    const tokenCount = await tokenizer.countTokens(longPrompt);
    console.log(`   プロンプトのトークン数: ${tokenCount}`);
    
    const params = await Schemas.GenerateParamsSchema.parseAsync({
      prompt: longPrompt,
    }) as GenerateParams;
    console.log("❌ NG - バリデーション成功（これは期待されない結果です - エラーになるべき）");
  } catch (e) {
    if (e instanceof ZodError) {
      console.log("✅ OK - バリデーションエラー（期待通り）:");
      e.issues.forEach(issue => {
        console.log(`   - ${issue.path.join('.')}: ${issue.message}`);
      });
    } else if (e instanceof TokenValidationError) {
      console.log("✅ OK - トークン検証エラー（期待通り）:");
      console.log(`   - トークン数: ${e.tokenCount}, 上限: ${e.maxTokens}`);
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  // Test 6: validateTokenCount関数の直接テスト
  console.log("\nTest 6: validateTokenCount関数の直接テスト");
  try {
    const shortPrompt = "hello world";
    const count = await validateTokenCount(shortPrompt);
    console.log(`✅ OK - 短いプロンプト: トークン数=${count}`);
  } catch (e) {
    console.error("❌ NG - 短いプロンプトでエラー:", e);
  }

  try {
    const longPrompt = Array(600).fill("masterpiece beautiful detailed anime").join(", ");
    const count = await validateTokenCount(longPrompt);
    console.log(`❌ NG - 長いプロンプトが通過（エラーになるべき）: トークン数=${count}`);
  } catch (e) {
    if (e instanceof TokenValidationError) {
      console.log(`✅ OK - 長いプロンプトでエラー: トークン数=${e.tokenCount}, 上限=${e.maxTokens}`);
    } else {
      console.error("予期しないエラー:", e);
    }
  }

  console.log("\n=== テスト完了 ===");
})();
