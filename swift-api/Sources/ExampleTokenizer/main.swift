import Foundation
import NovelAIAPI

// CLIP Tokenizer example
func clipTokenizerExample(text: String) async throws {
    print("\n=== CLIP Tokenizer ===")
    print("Input: \"\(text)\"")
    let tokenizer: NovelAIClipTokenizer = try await getClipTokenizer()
    let tokens: [Int] = tokenizer.encode(text)
    print("Token IDs: [\(tokens.prefix(10).map(String.init).joined(separator: ", "))\(tokens.count > 10 ? ", ..." : "")]")
    print("Token Count: \(tokens.count)")
}

// T5 Tokenizer example
func t5TokenizerExample(text: String) async throws {
    print("\n=== T5 Tokenizer ===")
    print("Input: \"\(text)\"")
    let textWithTags = "masterpiece, best quality, " + text
    print("With tags: \"\(textWithTags)\"")
    let tokenizer = try await getT5Tokenizer()
    let ids = tokenizer.encode(textWithTags)
    print("Token IDs: [\(ids.prefix(10).map(String.init).joined(separator: ", "))\(ids.count > 10 ? ", ..." : "")]")
    print("Effective Token Count: \(ids.count)")
}

// Cache behavior demo
func cacheExample() async throws {
    print("\n=== Cache Behavior ===")
    print("First call (fetches from server)...")
    let start1 = CFAbsoluteTimeGetCurrent()
    _ = try await getClipTokenizer()
    print("Time: \(Int((CFAbsoluteTimeGetCurrent() - start1) * 1000))ms")

    print("Second call (uses cache)...")
    let start2 = CFAbsoluteTimeGetCurrent()
    _ = try await getClipTokenizer()
    print("Time: \(Int((CFAbsoluteTimeGetCurrent() - start2) * 1000))ms")

    print("Force refresh (fetches again)...")
    let start3 = CFAbsoluteTimeGetCurrent()
    _ = try await getClipTokenizer(forceRefresh: true)
    print("Time: \(Int((CFAbsoluteTimeGetCurrent() - start3) * 1000))ms")
}

func countTokensExample() async throws {
    let tokenizer = try await getT5Tokenizer()
    let count1 = tokenizer.countTokens("3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}")
    print("Token count: \(count1) Expected: 25")
    let count2 = tokenizer.countTokens("1girl, graphite (medium), plaid background, from side, cowboy shot, stuffed animal, stuffed lion, mimikaki, candle, offering hand")
    print("Token count: \(count2) Expected: 38")
    let count3 = tokenizer.countTokens("2::girls::, 2::smile, standing, ::, {{ scared }}, 3::sitting::, 3::spread arms, spread wings::")
    print("Token count: \(count3) Expected: 19")
}

// main
let args = Array(CommandLine.arguments.dropFirst())
let sampleText = args.count > 0 ? args.joined(separator: " ") : "1girl, {beautiful:1.2}, [masterpiece], blonde hair, blue eyes"
print("========================================")
print("NovelAI Tokenizer Example")
print("========================================")
do {
    try await clipTokenizerExample(text: sampleText)
    try await t5TokenizerExample(text: sampleText)
    try await cacheExample()
    try await countTokensExample()
    print("\n✅ All examples completed successfully!")
} catch {
    print("\n❌ Error: \(error)")
    exit(1)
}
