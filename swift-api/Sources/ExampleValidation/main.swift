import Foundation
import NovelAIAPI

print("=== バリデーションテスト ===\n")

// Test 1: 1216×832 (OK)
print("Test 1: 1216×832")
do {
    let params = GenerateParams(prompt: "test", width: 1216, height: 832)
    try params.validate()
    print("✅ OK - バリデーション成功")
    print("   総ピクセル数: \(1216 * 832) = \(params.width * params.height)\n")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("❌ NG - バリデーションエラー:")
        print("   - \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

// Test 2: 1280×1280 (NG - exceeds max pixels)
print("\nTest 2: 1280×1280")
do {
    let params = GenerateParams(prompt: "test", width: 1280, height: 1280)
    try params.validate()
    print("✅ OK - バリデーション成功")
    print("   総ピクセル数: \(params.width * params.height)\n")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("❌ NG - バリデーションエラー:")
        print("   - \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

// Test 3: 1024×1024 (OK - at limit)
print("\nTest 3: 1024×1024")
do {
    let params = GenerateParams(prompt: "test", width: 1024, height: 1024)
    try params.validate()
    print("✅ OK - バリデーション成功")
    print("   総ピクセル数: \(params.width * params.height)\n")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("❌ NG - バリデーションエラー:")
        print("   - \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

// Token validation tests
print("\n=== トークン数バリデーションテスト ===\n")
print("トークナイザーをプリロード中...")
let tokenizer = try await getT5Tokenizer()
print("トークナイザー準備完了\n")

// Test 4: Short prompt (OK)
print("Test 4: 短いプロンプト (512トークン以下)")
do {
    let shortPrompt = "a beautiful landscape with mountains and rivers"
    let tokenCount = tokenizer.countTokens(shortPrompt)
    print("   プロンプトのトークン数: \(tokenCount)")
    let params = GenerateParams(prompt: shortPrompt)
    try params.validate()
    let _ = try await validateTokenCount(shortPrompt)
    print("✅ OK - バリデーション成功")
    print("   プロンプト: \"\(shortPrompt)\"\n")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("❌ NG - バリデーションエラー:")
        print("   - \(msg)")
    case .tokenValidation(let msg):
        print("❌ NG - トークン検証エラー:")
        print("   - \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

// Test 5: Long prompt (NG - exceeds 512 tokens)
print("\nTest 5: 長すぎるプロンプト (512トークン超過)")
do {
    let longPrompt = Array(repeating: "masterpiece beautiful detailed anime girl", count: 600).joined(separator: ", ")
    let tokenCount = tokenizer.countTokens(longPrompt)
    print("   プロンプトのトークン数: \(tokenCount)")
    let _ = try await validateTokenCount(longPrompt)
    print("❌ NG - バリデーション成功（これは期待されない結果です - エラーになるべき）")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("✅ OK - バリデーションエラー（期待通り）:")
        print("   - \(msg)")
    case .tokenValidation(let msg):
        print("✅ OK - トークン検証エラー（期待通り）:")
        print("   - \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

// Test 6: validateTokenCount direct test
print("\nTest 6: validateTokenCount関数の直接テスト")
do {
    let count = try await validateTokenCount("hello world")
    print("✅ OK - 短いプロンプト: トークン数=\(count)")
} catch {
    print("❌ NG - 短いプロンプトでエラー: \(error)")
}

do {
    let longPrompt = Array(repeating: "masterpiece beautiful detailed anime", count: 600).joined(separator: ", ")
    let count = try await validateTokenCount(longPrompt)
    print("❌ NG - 長いプロンプトが通過（エラーになるべき）: トークン数=\(count)")
} catch let error as NovelAIError {
    switch error {
    case .tokenValidation(let msg):
        print("✅ OK - 長いプロンプトでエラー: \(msg)")
    default:
        print("予期しないエラー: \(error)")
    }
} catch {
    print("予期しないエラー: \(error)")
}

print("\n=== テスト完了 ===")
