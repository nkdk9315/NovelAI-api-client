import XCTest
@testable import NovelAIAPI

// MARK: - A. validateImageDataSize Tests

final class ValidateImageDataSizeTests: XCTestCase {

    func testDoesNotThrowForSmallBuffer() {
        let data = Data(count: 1024) // 1KB
        XCTAssertNoThrow(try validateImageDataSize(data))
    }

    func testThrowsForOversizedBuffer() {
        let size = (MAX_REF_IMAGE_SIZE_MB + 1) * 1024 * 1024
        let data = Data(count: size)
        XCTAssertThrowsError(try validateImageDataSize(data)) { error in
            guard case NovelAIError.imageFileSize = error else {
                XCTFail("Expected NovelAIError.imageFileSize, got \(error)")
                return
            }
        }
    }

    func testErrorMessageIncludesSource() {
        let size = (MAX_REF_IMAGE_SIZE_MB + 1) * 1024 * 1024
        let data = Data(count: size)
        XCTAssertThrowsError(try validateImageDataSize(data, source: "test.png")) { error in
            if case NovelAIError.imageFileSize(let msg) = error {
                XCTAssertTrue(msg.contains("test.png"), "Error message should contain source path")
            } else {
                XCTFail("Expected NovelAIError.imageFileSize")
            }
        }
    }

    func testExactlySizeLimitPasses() {
        let size = MAX_REF_IMAGE_SIZE_MB * 1024 * 1024
        let data = Data(count: size)
        XCTAssertNoThrow(try validateImageDataSize(data))
    }

    func testEmptyBufferPasses() {
        let data = Data()
        XCTAssertNoThrow(try validateImageDataSize(data))
    }
}

// MARK: - B. getImageBuffer Tests

final class GetImageBufferTests: XCTestCase {

    func testReturnsDataInputAsIs() {
        let original = Data([1, 2, 3, 4])
        let result = try! getImageBuffer(.bytes(original))
        XCTAssertEqual(result, original)
    }

    func testDecodesBase64WithDataURLPrefix() {
        let original = Data("hello world".utf8)
        let b64 = original.base64EncodedString()
        let result = try! getImageBuffer(.dataURL("data:image/png;base64,\(b64)"))
        XCTAssertEqual(result, original)
    }

    func testDecodesPlainBase64() {
        let original = Data("test-data".utf8)
        let b64 = original.base64EncodedString()
        let result = try! getImageBuffer(.base64(b64))
        XCTAssertEqual(result, original)
    }

    func testThrowsForPathTraversal() {
        XCTAssertThrowsError(try getImageBuffer(.filePath("../../etc/passwd"))) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("path traversal"), "Expected path traversal error")
            }
        }
    }

    func testThrowsForNonexistentFile() {
        XCTAssertThrowsError(try getImageBuffer(.filePath("/nonexistent/image.png"))) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("not found or not readable"), "Expected file not found error")
            }
        }
    }

    func testThrowsForInvalidBase64() {
        XCTAssertThrowsError(try getImageBuffer(.base64("abc!@#$def"))) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid Base64"), "Expected invalid base64 error")
            }
        }
    }

    func testThrowsForEmptyBase64() {
        XCTAssertThrowsError(try getImageBuffer(.base64(""))) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid Base64"), "Expected invalid base64 error")
            }
        }
    }
}

// MARK: - C. getImageBase64 Tests

final class GetImageBase64Tests: XCTestCase {

    func testConvertsDataToBase64() {
        let data = Data("hello".utf8)
        let result = try! getImageBase64(.bytes(data))
        XCTAssertEqual(result, data.base64EncodedString())
    }

    func testRoundTrips() {
        let original = Data([72, 101, 108, 108, 111]) // "Hello"
        let b64 = try! getImageBase64(.bytes(original))
        let decoded = Data(base64Encoded: b64)
        XCTAssertEqual(decoded, original)
    }
}

// MARK: - D. looksLikeFilePath Tests

final class LooksLikeFilePathTests: XCTestCase {

    func testDataURLIsNotPath() {
        XCTAssertFalse(looksLikeFilePath("data:image/png;base64,abc"))
    }

    func testLongBase64IsNotPath() {
        let longB64 = String(repeating: "x", count: 100)
        XCTAssertFalse(looksLikeFilePath(longB64))
    }

    func testAbsolutePathWithExtension() {
        XCTAssertTrue(looksLikeFilePath("/image.png"))
    }

    func testAbsolutePathWithTwoSegments() {
        XCTAssertTrue(looksLikeFilePath("/dir/file"))
    }

    func testWindowsAbsolutePath() {
        XCTAssertTrue(looksLikeFilePath("C:\\images\\test.png"))
    }

    func testRelativePathWithExtension() {
        XCTAssertTrue(looksLikeFilePath("images/test.png"))
    }

    func testFilenameWithExtension() {
        XCTAssertTrue(looksLikeFilePath("test.png"))
    }

    func testVibeExtension() {
        XCTAssertTrue(looksLikeFilePath("file.naiv4vibe"))
    }

    func testPathWithSlash() {
        XCTAssertTrue(looksLikeFilePath("dir/file"))
    }

    func testShortBase64NoSlash() {
        // Short string without slashes, extensions, or path markers
        // Should be treated as base64 (returns false)
        XCTAssertFalse(looksLikeFilePath("abc123"))
    }
}

// MARK: - E. loadVibeFile Tests

final class LoadVibeFileTests: XCTestCase {

    func testThrowsForPathTraversal() {
        XCTAssertThrowsError(try loadVibeFile("../../etc/secret.naiv4vibe")) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("path traversal"))
            }
        }
    }

    func testThrowsForNonexistentFile() {
        XCTAssertThrowsError(try loadVibeFile("/nonexistent/vibe.naiv4vibe"))
    }
}

// MARK: - F. extractEncoding Tests

final class ExtractEncodingTests: XCTestCase {

    private func makeVibeData(
        modelKey: String,
        encoding: String,
        params: [String: Any] = [:],
        importInfo: [String: Any]? = nil
    ) -> [String: Any] {
        var result: [String: Any] = [
            "encodings": [
                modelKey: [
                    "someKey": [
                        "encoding": encoding,
                        "params": params,
                    ] as [String: Any],
                ] as [String: Any],
            ] as [String: Any],
        ]
        if let importInfo = importInfo {
            result["importInfo"] = importInfo
        }
        return result
    }

    func testExtractsEncodingAndInfoExtracted() {
        let data = makeVibeData(modelKey: "v4-5full", encoding: "abc123", params: ["information_extracted": 0.8])
        let result = try! extractEncoding(data)
        XCTAssertEqual(result.encoding, "abc123")
        XCTAssertEqual(result.informationExtracted, 0.8)
    }

    func testImportInfoTakesPriority() {
        let data = makeVibeData(
            modelKey: "v4-5full",
            encoding: "abc123",
            params: ["information_extracted": 0.5],
            importInfo: ["information_extracted": 0.9]
        )
        let result = try! extractEncoding(data)
        XCTAssertEqual(result.informationExtracted, 0.9)
    }

    func testThrowsForNonexistentModelKey() {
        let data: [String: Any] = ["encodings": [:] as [String: Any]]
        XCTAssertThrowsError(try extractEncoding(data, model: .naiDiffusion45Full)) { error in
            if case NovelAIError.image(let msg) = error {
                XCTAssertTrue(msg.contains("No encoding found"))
            }
        }
    }

    func testDefaultModel() {
        let data = makeVibeData(modelKey: "v4-5full", encoding: "default-enc")
        let result = try! extractEncoding(data) // no model arg
        XCTAssertEqual(result.encoding, "default-enc")
    }
}

// MARK: - G. processVibes Tests

final class ProcessVibesTests: XCTestCase {

    func testProcessesVibeEncodeResult() {
        let vibeResult = VibeEncodeResult(
            encoding: "enc1",
            model: .naiDiffusion45Full,
            informationExtracted: 0.7,
            strength: 0.5,
            sourceImageHash: String(repeating: "a", count: 64),
            createdAt: Date()
        )
        let result = try! processVibes([.encoded(vibeResult)], model: .naiDiffusion45Full)
        XCTAssertEqual(result.encodings, ["enc1"])
        XCTAssertEqual(result.infoExtractedList, [0.7])
    }

    func testProcessesRawBase64String() {
        let result = try! processVibes([.filePath("someBase64EncodedString")], model: .naiDiffusion45Full)
        XCTAssertEqual(result.encodings, ["someBase64EncodedString"])
        XCTAssertEqual(result.infoExtractedList, [1.0])
    }

    func testEmptyArrayReturnsEmptyResults() {
        let result = try! processVibes([], model: .naiDiffusion45Full)
        XCTAssertEqual(result.encodings, [])
        XCTAssertEqual(result.infoExtractedList, [])
    }
}

// MARK: - H. Mask Tests

final class MaskTests: XCTestCase {

    // MARK: createRectangularMask

    func testRectangularMaskThrowsForZeroWidth() {
        XCTAssertThrowsError(
            try createRectangularMask(width: 0, height: 100, region: MaskRegion(x: 0, y: 0, w: 1, h: 1))
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid dimensions"))
            }
        }
    }

    func testRectangularMaskThrowsForNegativeHeight() {
        XCTAssertThrowsError(
            try createRectangularMask(width: 100, height: -1, region: MaskRegion(x: 0, y: 0, w: 1, h: 1))
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid dimensions"))
            }
        }
    }

    func testRectangularMaskThrowsForInvalidRegionX() {
        XCTAssertThrowsError(
            try createRectangularMask(width: 100, height: 100, region: MaskRegion(x: 1.5, y: 0, w: 0.5, h: 0.5))
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid region.x"))
            }
        }
    }

    func testRectangularMaskThrowsForNegativeRegionY() {
        XCTAssertThrowsError(
            try createRectangularMask(width: 100, height: 100, region: MaskRegion(x: 0, y: -0.1, w: 0.5, h: 0.5))
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid region.y"))
            }
        }
    }

    func testRectangularMaskReturnsDataForValidInput() {
        let result = try! createRectangularMask(
            width: 800, height: 600, region: MaskRegion(x: 0.1, y: 0.1, w: 0.5, h: 0.5)
        )
        XCTAssertFalse(result.isEmpty)
        // Check PNG signature
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47])) // PNG magic bytes
    }

    // MARK: createCircularMask

    func testCircularMaskThrowsForZeroWidth() {
        XCTAssertThrowsError(
            try createCircularMask(width: 0, height: 100, center: MaskCenter(x: 0.5, y: 0.5), radius: 0.3)
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid dimensions"))
            }
        }
    }

    func testCircularMaskThrowsForInvalidCenter() {
        XCTAssertThrowsError(
            try createCircularMask(width: 100, height: 100, center: MaskCenter(x: 1.5, y: 0.5), radius: 0.3)
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid center"))
            }
        }
    }

    func testCircularMaskThrowsForNegativeRadius() {
        XCTAssertThrowsError(
            try createCircularMask(width: 100, height: 100, center: MaskCenter(x: 0.5, y: 0.5), radius: -0.1)
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid radius"))
            }
        }
    }

    func testCircularMaskThrowsForRadiusOver1() {
        XCTAssertThrowsError(
            try createCircularMask(width: 100, height: 100, center: MaskCenter(x: 0.5, y: 0.5), radius: 1.5)
        ) { error in
            if case NovelAIError.validation(let msg) = error {
                XCTAssertTrue(msg.contains("Invalid radius"))
            }
        }
    }

    func testCircularMaskReturnsDataForValidInput() {
        let result = try! createCircularMask(
            width: 800, height: 600, center: MaskCenter(x: 0.5, y: 0.5), radius: 0.3
        )
        XCTAssertFalse(result.isEmpty)
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47]))
    }
}

// MARK: - I. calculateCacheSecretKey Tests

final class CacheSecretKeyTests: XCTestCase {

    func testReturnsHexString() {
        let data = Data("test image data".utf8)
        let key = calculateCacheSecretKey(data)
        XCTAssertEqual(key.count, 64) // SHA256 hex = 64 chars
        XCTAssertTrue(key.allSatisfy { $0.isHexDigit })
    }

    func testDifferentDataProducesDifferentKeys() {
        let key1 = calculateCacheSecretKey(Data("data1".utf8))
        let key2 = calculateCacheSecretKey(Data("data2".utf8))
        XCTAssertNotEqual(key1, key2)
    }

    func testSameDataProducesSameKey() {
        let data = Data("consistent".utf8)
        let key1 = calculateCacheSecretKey(data)
        let key2 = calculateCacheSecretKey(data)
        XCTAssertEqual(key1, key2)
    }
}
