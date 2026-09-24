import XCTest
@testable import NovelAIAPI

/// The Qwen tokenizer (V5) must match the official site's encoder output.
/// Expected IDs were produced by running the site's own BPE class on the same definition file.
final class QwenTokenizerTests: XCTestCase {
    private struct Case: Decodable {
        let text: String
        let ids: [Int]
    }

    func testMatchesOfficialEncoder() async throws {
        let tokenizer: NovelAIQwenTokenizer
        do {
            tokenizer = try await getQwenTokenizer()
        } catch {
            throw XCTSkip("Qwen tokenizer unavailable: \(error)")
        }
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .appendingPathComponent("Fixtures/qwen_expected.json")
        let cases = try JSONDecoder().decode([Case].self, from: Data(contentsOf: url))
        for c in cases {
            XCTAssertEqual(tokenizer.encode(c.text), c.ids, "text: \(c.text.debugDescription)")
        }
    }
}
