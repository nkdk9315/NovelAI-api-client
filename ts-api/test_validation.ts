import * as Schemas from './src/schemas';
import { ZodError } from 'zod';

console.log("=== バリデーションテスト ===\n");

// Test 1: 1216×832 (OK)
console.log("Test 1: 1216×832");
try {
  const params = Schemas.GenerateParamsSchema.parse({
    prompt: "test",
    width: 1216,
    height: 832,
  });
  console.log("✅ OK - バリデーション成功");
  console.log(`   総ピクセル数: ${1216 * 832} = ${params.width * params.height}\n`);
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
  const params = Schemas.GenerateParamsSchema.parse({
    prompt: "test",
    width: 1280,
    height: 1280,
  });
  console.log("✅ OK - バリデーション成功");
  console.log(`   総ピクセル数: ${params.width * params.height}\n`);
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
  const params = Schemas.GenerateParamsSchema.parse({
    prompt: "test",
    width: 1024,
    height: 1024,
  });
  console.log("✅ OK - バリデーション成功");
  console.log(`   総ピクセル数: ${params.width * params.height}\n`);
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
