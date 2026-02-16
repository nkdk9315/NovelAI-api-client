import XCTest
@testable import NovelAIAPI

// MARK: - preprocessT5 Tests

final class PreprocessT5Tests: XCTestCase {

    // MARK: Bracket Removal

    func testRemovesSquareBrackets() {
        XCTAssertEqual(preprocessT5("[hello]"), "hello")
    }

    func testRemovesCurlyBrackets() {
        XCTAssertEqual(preprocessT5("{hello}"), "hello")
    }

    func testRemovesMixedBrackets() {
        XCTAssertEqual(preprocessT5("[hello] {world}"), "hello world")
    }

    func testRemovesNestedBrackets() {
        XCTAssertEqual(preprocessT5("[[nested]]"), "nested")
    }

    func testEmptyBracketsRemoved() {
        XCTAssertEqual(preprocessT5("before[]after"), "beforeafter")
    }

    // MARK: Weight Syntax Removal

    func testRemovesIntegerWeight() {
        XCTAssertEqual(preprocessT5("2::hello::"), "hello")
    }

    func testRemovesDecimalWeight() {
        XCTAssertEqual(preprocessT5("1.5::hello world::"), "hello world")
    }

    func testRemovesNegativeWeight() {
        XCTAssertEqual(preprocessT5("-1::bad quality::"), "bad quality")
    }

    func testRemovesWeightWithoutNumber() {
        XCTAssertEqual(preprocessT5("::content::"), "content")
    }

    // MARK: Preservation

    func testPreservesCase() {
        XCTAssertEqual(preprocessT5("Hello WORLD"), "Hello WORLD")
    }

    func testPreservesWhitespace() {
        XCTAssertEqual(preprocessT5("hello  world"), "hello  world")
    }

    func testPreservesHTMLEntities() {
        // T5 does NOT decode HTML entities (unlike CLIP)
        XCTAssertEqual(preprocessT5("&amp;"), "&amp;")
    }

    func testEmptyString() {
        XCTAssertEqual(preprocessT5(""), "")
    }

    // MARK: Combined Operations

    func testBracketsAndWeightsTogether() {
        XCTAssertEqual(preprocessT5("[1girl], 2::beautiful::"), "1girl, beautiful")
    }

    func testComplexPrompt() {
        let input = "1girl, [solo], {masterpiece}, 1.5::beautiful scenery::, best quality"
        let expected = "1girl, solo, masterpiece, beautiful scenery, best quality"
        XCTAssertEqual(preprocessT5(input), expected)
    }
}

// MARK: - HTML Entity Decoding Tests (used by CLIP)

final class HTMLEntityDecodingTests: XCTestCase {

    func testDecodesNamedEntities() {
        XCTAssertEqual(decodeHTMLEntities("&amp;"), "&")
        XCTAssertEqual(decodeHTMLEntities("&lt;"), "<")
        XCTAssertEqual(decodeHTMLEntities("&gt;"), ">")
        XCTAssertEqual(decodeHTMLEntities("&quot;"), "\"")
    }

    func testDecodesNumericEntities() {
        XCTAssertEqual(decodeHTMLEntities("&#65;"), "A")
        XCTAssertEqual(decodeHTMLEntities("&#97;"), "a")
    }

    func testDecodesHexEntities() {
        XCTAssertEqual(decodeHTMLEntities("&#x41;"), "A")
        XCTAssertEqual(decodeHTMLEntities("&#x1F60E;"), "😎")
    }

    func testDoubleDecoding() {
        // &amp;amp; → first decode → &amp; → second decode → &
        let once = decodeHTMLEntities("&amp;amp;")
        let twice = decodeHTMLEntities(once)
        XCTAssertEqual(twice, "&")
    }

    func testPassthroughNoEntities() {
        XCTAssertEqual(decodeHTMLEntities("hello world"), "hello world")
    }

    func testUnknownEntityPassthrough() {
        XCTAssertEqual(decodeHTMLEntities("&unknown;"), "&unknown;")
    }
}

// MARK: - Shared Test Vocabulary

/// Minimal vocabulary used by both PureUnigram and T5Tokenizer tests.
private let sharedTestVocab: [(String, Double)] = [
    ("<unk>", 0),       // id 0
    ("</s>", 0),        // id 1
    ("\u{2581}", -1.0),        // ▁ (metaspace), id 2
    ("\u{2581}hello", -2.0),   // id 3
    ("hello", -3.0),           // id 4
    ("world", -3.0),           // id 5
    ("\u{2581}world", -2.0),   // id 6
    ("h", -5.0),              // id 7
    ("e", -5.0),              // id 8
    ("l", -5.0),              // id 9
    ("o", -5.0),              // id 10
    ("w", -5.0),              // id 11
    ("r", -5.0),              // id 12
    ("d", -5.0),              // id 13
]

// MARK: - PureUnigram Tests

final class PureUnigramTests: XCTestCase {

    /// Create a minimal vocabulary for testing.
    private func makeTestUnigram() -> PureUnigram {
        return PureUnigram(vocabEntries: sharedTestVocab, unkId: 0)
    }

    func testTokenToIdKnown() {
        let unigram = makeTestUnigram()
        XCTAssertEqual(unigram.tokenToId("</s>"), 1)
        XCTAssertEqual(unigram.tokenToId("\u{2581}hello"), 3)
    }

    func testTokenToIdUnknown() {
        let unigram = makeTestUnigram()
        XCTAssertNil(unigram.tokenToId("nonexistent"))
    }

    func testEncodeSimple() {
        let unigram = makeTestUnigram()
        let ids = unigram.encode("hello")
        XCTAssertFalse(ids.isEmpty)
        // Should start with ▁hello (id 3) since metaspace is prepended
        XCTAssertEqual(ids.first, 3) // ▁hello
    }

    func testEncodeMultipleWords() {
        let unigram = makeTestUnigram()
        let ids = unigram.encode("hello world")
        XCTAssertFalse(ids.isEmpty)
        // Should produce ▁hello (3) and ▁world (6)
        XCTAssertTrue(ids.contains(3))
        XCTAssertTrue(ids.contains(6))
    }

    func testEncodeEmptyString() {
        let unigram = makeTestUnigram()
        let ids = unigram.encode("")
        XCTAssertTrue(ids.isEmpty)
    }

    func testEncodeWhitespaceOnly() {
        let unigram = makeTestUnigram()
        let ids = unigram.encode("   ")
        XCTAssertTrue(ids.isEmpty)
    }
}

// MARK: - NovelAIT5Tokenizer Tests

final class NovelAIT5TokenizerTests: XCTestCase {

    private func makeTestTokenizer() -> NovelAIT5Tokenizer {
        let unigram = PureUnigram(vocabEntries: sharedTestVocab, unkId: 0)
        return NovelAIT5Tokenizer(backend: unigram)
    }

    func testEncodeEmptyReturnsEOS() {
        let tokenizer = makeTestTokenizer()
        let ids = tokenizer.encode("")
        XCTAssertEqual(ids, [1]) // EOS only
    }

    func testEncodeAppendsEOS() {
        let tokenizer = makeTestTokenizer()
        let ids = tokenizer.encode("hello")
        XCTAssertFalse(ids.isEmpty)
        XCTAssertEqual(ids.last, 1) // EOS at end
    }

    func testCountTokensIncludesEOS() {
        let tokenizer = makeTestTokenizer()
        let count = tokenizer.countTokens("hello")
        XCTAssertGreaterThan(count, 1) // At least token + EOS
    }

    func testCountTokensEmptyIsOne() {
        let tokenizer = makeTestTokenizer()
        let count = tokenizer.countTokens("")
        XCTAssertEqual(count, 1) // EOS only
    }

    func testPreprocessRemovesBrackets() {
        let tokenizer = makeTestTokenizer()
        let withBrackets = tokenizer.encode("[hello]")
        let without = tokenizer.encode("hello")
        XCTAssertEqual(withBrackets, without)
    }
}

// MARK: - getCacheFilename Tests

final class CacheFilenameTests: XCTestCase {

    func testGeneratesCorrectFilename() throws {
        let filename = try getCacheFilename("https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true")
        XCTAssertEqual(filename, "t5_tokenizer_v2.json")
    }

    func testClipTokenizerFilename() throws {
        let filename = try getCacheFilename("https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true")
        XCTAssertEqual(filename, "clip_tokenizer_v2.json")
    }

    func testMissingVersionUsesUnknown() throws {
        let filename = try getCacheFilename("https://example.com/tokenizer.def")
        XCTAssertEqual(filename, "tokenizer_vunknown.json")
    }

    func testEmptyBasenameThrows() {
        XCTAssertThrowsError(try getCacheFilename("https://example.com/.def")) { error in
            guard case NovelAIError.tokenizer = error else {
                XCTFail("Expected NovelAIError.tokenizer")
                return
            }
        }
    }
}

// MARK: - bytesToUnicode Tests

final class BytesToUnicodeTests: XCTestCase {

    func testMapsAll256Bytes() {
        let mapping = bytesToUnicode()
        XCTAssertEqual(mapping.count, 256)
    }

    func testAllValuesUnique() {
        let mapping = bytesToUnicode()
        let values = Set(mapping.values)
        XCTAssertEqual(values.count, 256)
    }

    func testPrintableASCIIMapsToSelf() {
        let mapping = bytesToUnicode()
        // 'A' = 65, should map to 'A'
        XCTAssertEqual(mapping[65], Character("A"))
        // 'z' = 122, should map to 'z'
        XCTAssertEqual(mapping[122], Character("z"))
        // '0' = 48 maps to ?, not self (it's < 33)
        // Actually 48 is in 33..126, so it maps to self
        XCTAssertEqual(mapping[48], Character("0"))
    }
}
