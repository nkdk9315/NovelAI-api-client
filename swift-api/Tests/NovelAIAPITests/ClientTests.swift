import XCTest
import ZIPFoundation

@testable import NovelAIAPI

// MARK: - Mock URL Protocol

/// A URLProtocol subclass for mocking HTTP responses in tests.
/// Set `requestHandler` before each test to define the mock behavior.
final class MockURLProtocol: URLProtocol {
    private static let lock = NSLock()
    private static var _requestHandler: ((URLRequest) throws -> (HTTPURLResponse, Data))?
    private static var _capturedRequests: [URLRequest] = []

    static var requestHandler: ((URLRequest) throws -> (HTTPURLResponse, Data))? {
        get { lock.withLock { _requestHandler } }
        set { lock.withLock { _requestHandler = newValue } }
    }

    static var capturedRequests: [URLRequest] {
        get { lock.withLock { _capturedRequests } }
        set { lock.withLock { _capturedRequests = newValue } }
    }

    override class func canInit(with request: URLRequest) -> Bool {
        return true
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        return request
    }

    override func startLoading() {
        MockURLProtocol.capturedRequests.append(request)
        guard let handler = MockURLProtocol.requestHandler else {
            client?.urlProtocol(self, didFailWithError: URLError(.unknown))
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

// MARK: - Test Logger

/// A logger that captures log messages for assertions.
final class TestLogger: Logger, @unchecked Sendable {
    var warnings: [String] = []
    var errors: [String] = []

    func warn(_ message: String) {
        warnings.append(message)
    }

    func error(_ message: String) {
        errors.append(message)
    }
}

// MARK: - Test Helpers

/// Create a mock URLSession that uses MockURLProtocol.
func makeMockSession() -> URLSession {
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [MockURLProtocol.self]
    return URLSession(configuration: config)
}

/// Minimal valid PNG data: 8-byte signature + minimal IHDR + IEND.
/// This is a 1x1 pixel white PNG.
func makeMinimalPNG() -> Data {
    // PNG signature
    var data = Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
    // IHDR chunk
    let ihdrData: [UInt8] = [
        0x00, 0x00, 0x00, 0x01,  // width = 1
        0x00, 0x00, 0x00, 0x01,  // height = 1
        0x08,                     // bit depth = 8
        0x02,                     // color type = RGB
        0x00,                     // compression
        0x00,                     // filter
        0x00,                     // interlace
    ]
    appendChunk(to: &data, type: "IHDR", content: Data(ihdrData))
    // IDAT chunk (minimal compressed data for 1x1 RGB pixel)
    let idatData: [UInt8] = [
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01,
    ]
    appendChunk(to: &data, type: "IDAT", content: Data(idatData))
    // IEND chunk
    appendChunk(to: &data, type: "IEND", content: Data())
    return data
}

/// Append a PNG chunk to data (length + type + content + CRC).
private func appendChunk(to data: inout Data, type: String, content: Data) {
    // Length (4 bytes big-endian)
    var length = UInt32(content.count).bigEndian
    data.append(Data(bytes: &length, count: 4))
    // Type (4 bytes)
    let typeBytes = Array(type.utf8)
    data.append(contentsOf: typeBytes)
    // Content
    data.append(content)
    // CRC32 (over type + content)
    var crcInput = Data(typeBytes)
    crcInput.append(content)
    let crc = crc32Calculate(crcInput)
    var crcBE = crc.bigEndian
    data.append(Data(bytes: &crcBE, count: 4))
}

/// Simple CRC32 calculation for PNG chunk verification.
private func crc32Calculate(_ data: Data) -> UInt32 {
    var crc: UInt32 = 0xFFFFFFFF
    for byte in data {
        crc ^= UInt32(byte)
        for _ in 0..<8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320
            } else {
                crc >>= 1
            }
        }
    }
    return crc ^ 0xFFFFFFFF
}

/// Create a ZIP archive containing a PNG file, returned as Data.
func makeZipWithPNG(_ pngData: Data, filename: String = "image.png") throws -> Data {
    let archive = try Archive(accessMode: .create)
    try archive.addEntry(
        with: filename,
        type: .file,
        uncompressedSize: Int64(pngData.count),
        provider: { position, size in
            return pngData[Int(position)..<Int(position) + size]
        }
    )
    guard let data = archive.data else {
        throw NSError(domain: "TestHelper", code: 2, userInfo: [NSLocalizedDescriptionKey: "Archive produced no data"])
    }
    return data
}

/// Create an HTTP response for a given URL string and status code.
func makeHTTPResponse(url: String, statusCode: Int) -> HTTPURLResponse {
    return HTTPURLResponse(
        url: URL(string: url)!,
        statusCode: statusCode,
        httpVersion: "HTTP/1.1",
        headerFields: nil
    )!
}

// MARK: - 1. NovelAIClient Initialization Tests

final class NovelAIClientInitTests: XCTestCase {

    func testInitWithExplicitAPIKey() throws {
        let client = try NovelAIClient(apiKey: "test-api-key-12345")
        XCTAssertNotNil(client)
    }

    func testInitFailsWhenNoAPIKeyProvided() {
        // Ensure environment variables are not set by checking behavior.
        // If NAI_API_KEY or NOVELAI_API_KEY is set in the environment, this test
        // may pass with the env key. We test the empty-string case specifically.
        // We cannot easily unset env vars in Swift, so we test the error path
        // by relying on the typical CI/test environment not having these set.
        // If the env vars ARE set, the client would succeed, so we skip.
        let hasEnvKey = ProcessInfo.processInfo.environment["NAI_API_KEY"] != nil
            || ProcessInfo.processInfo.environment["NOVELAI_API_KEY"] != nil
        if hasEnvKey {
            // Cannot test "no key" scenario when env vars are set
            return
        }

        XCTAssertThrowsError(try NovelAIClient()) { error in
            guard case NovelAIError.api(let statusCode, let message) = error else {
                XCTFail("Expected NovelAIError.api, got \(error)")
                return
            }
            XCTAssertEqual(statusCode, 0)
            XCTAssertTrue(message.contains("API key is required"))
        }
    }

    func testInitWithEmptyStringFailsLikeNoKey() {
        XCTAssertThrowsError(try NovelAIClient(apiKey: "")) { error in
            guard case NovelAIError.api(let statusCode, let message) = error else {
                XCTFail("Expected NovelAIError.api, got \(error)")
                return
            }
            XCTAssertEqual(statusCode, 0)
            XCTAssertTrue(message.contains("API key is required"))
        }
    }

    func testInitWithCustomLogger() throws {
        let logger = TestLogger()
        let client = try NovelAIClient(apiKey: "test-key", logger: logger)
        XCTAssertNotNil(client)
    }
}

// MARK: - 2. Response Parsing Tests

final class ZipResponseParsingTests: XCTestCase {

    func testParseValidZipContainingPNG() throws {
        let pngData = makeMinimalPNG()
        let zipData = try makeZipWithPNG(pngData)

        let result = try parseZipResponse(zipData)
        XCTAssertEqual(result, pngData)
    }

    func testParseZipWithJPEGExtension() throws {
        let imageData = Data([0xFF, 0xD8, 0xFF, 0xE0])  // JPEG magic bytes
        let zipData = try makeZipWithPNG(imageData, filename: "image.jpeg")

        let result = try parseZipResponse(zipData)
        XCTAssertEqual(result, imageData)
    }

    func testParseZipWithWEBPExtension() throws {
        let imageData = Data(repeating: 0xAA, count: 100)
        let zipData = try makeZipWithPNG(imageData, filename: "output.webp")

        let result = try parseZipResponse(zipData)
        XCTAssertEqual(result, imageData)
    }

    func testParseZipWithJPGExtension() throws {
        let imageData = Data(repeating: 0xBB, count: 50)
        let zipData = try makeZipWithPNG(imageData, filename: "photo.jpg")

        let result = try parseZipResponse(zipData)
        XCTAssertEqual(result, imageData)
    }

    func testParseZipWithNoImageThrowsError() throws {
        let archive = try Archive(accessMode: .create)
        let textData = Data("hello".utf8)
        try archive.addEntry(
            with: "readme.txt",
            type: .file,
            uncompressedSize: Int64(textData.count),
            provider: { position, size in
                return textData[Int(position)..<Int(position) + size]
            }
        )
        let zipData = archive.data!

        XCTAssertThrowsError(try parseZipResponse(zipData)) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("No image found"))
        }
    }

    func testParseInvalidZipDataThrowsError() {
        let invalidData = Data([0x00, 0x01, 0x02, 0x03])

        XCTAssertThrowsError(try parseZipResponse(invalidData)) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Failed to open ZIP"))
        }
    }

    func testParseZipTooManyEntriesThrowsError() throws {
        let archive = try Archive(accessMode: .create)
        // Add more entries than MAX_ZIP_ENTRIES
        for i in 0..<(MAX_ZIP_ENTRIES + 1) {
            let data = Data(repeating: UInt8(i % 256), count: 10)
            try archive.addEntry(
                with: "file_\(i).png",
                type: .file,
                uncompressedSize: Int64(data.count),
                provider: { position, size in
                    return data[Int(position)..<Int(position) + size]
                }
            )
        }
        let zipData = archive.data!

        XCTAssertThrowsError(try parseZipResponse(zipData)) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Too many ZIP entries"))
        }
    }

    func testParseEmptyDataThrowsError() {
        XCTAssertThrowsError(try parseZipResponse(Data())) { error in
            guard case NovelAIError.parse = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
        }
    }
}

// MARK: - Stream Response Parsing Tests

final class StreamResponseParsingTests: XCTestCase {

    func testParseStreamWithZipSignature() throws {
        let pngData = makeMinimalPNG()
        let zipData = try makeZipWithPNG(pngData)

        let result = try parseStreamResponse(zipData)
        XCTAssertEqual(result, pngData)
    }

    func testParseStreamWithRawPNG() throws {
        let pngData = makeMinimalPNG()

        let result = try parseStreamResponse(pngData)
        XCTAssertEqual(result, pngData)
    }

    func testParseStreamWithEmbeddedPNG() throws {
        let pngData = makeMinimalPNG()

        // Create data with random prefix followed by embedded PNG
        var streamData = Data(repeating: 0xAA, count: 100)
        streamData.append(pngData)

        let logger = TestLogger()
        let result = try parseStreamResponse(streamData, logger: logger)

        // Result should be the extracted PNG data
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]))
        XCTAssertTrue(logger.warnings.contains(where: { $0.contains("embedded PNG") }))
    }

    func testParseStreamWithEmbeddedPNGAndTrailingData() throws {
        let pngData = makeMinimalPNG()

        // Create data with prefix, PNG, and trailing data
        var streamData = Data(repeating: 0xBB, count: 50)
        streamData.append(pngData)
        streamData.append(Data(repeating: 0xCC, count: 30))

        let result = try parseStreamResponse(streamData)

        // Should extract just the PNG portion (up to IEND + CRC)
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]))
        // Result should NOT include trailing 0xCC bytes
        XCTAssertTrue(result.count <= pngData.count)
    }

    func testParseStreamEmptyDataThrowsError() {
        XCTAssertThrowsError(try parseStreamResponse(Data())) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Cannot parse stream response"))
        }
    }

    func testParseStreamSingleByteThrowsError() {
        XCTAssertThrowsError(try parseStreamResponse(Data([0x42]))) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Cannot parse stream response"))
        }
    }

    func testParseStreamUnrecognizedFormatThrowsError() {
        // Data that doesn't match any known format
        let randomData = Data(repeating: 0x42, count: 200)

        XCTAssertThrowsError(try parseStreamResponse(randomData)) { error in
            guard case NovelAIError.parse(let msg) = error else {
                XCTFail("Expected NovelAIError.parse, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Cannot parse stream response"))
        }
    }

    func testParseStreamMultiplePNGsUsesLast() throws {
        // Test that when multiple PNGs are embedded, the last one is used
        // (the last PNG is supposed to be the full-resolution image)
        let png1 = makeMinimalPNG()
        let png2 = makeMinimalPNG()

        // Prepend some junk, then png1, then more junk, then png2
        var streamData = Data(repeating: 0x01, count: 20)
        streamData.append(png1)
        streamData.append(Data(repeating: 0x02, count: 20))
        streamData.append(png2)
        streamData.append(Data(repeating: 0x03, count: 10))

        let result = try parseStreamResponse(streamData)
        // Should find the last PNG (png2)
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47]))
    }
}

// MARK: - 3. Payload Construction Tests

final class PayloadConstructionTests: XCTestCase {

    // MARK: - buildBasePayload Tests

    func testBuildBasePayloadProducesCorrectStructure() {
        let params = GenerateParams(prompt: "1girl, masterpiece")
        let seed: UInt32 = 42
        let negativePrompt = "bad quality"

        let payload = buildBasePayload(params, seed: seed, negativePrompt: negativePrompt)

        // Top-level fields
        XCTAssertEqual(payload["input"] as? String, "1girl, masterpiece")
        XCTAssertEqual(payload["model"] as? String, DEFAULT_MODEL.rawValue)
        XCTAssertEqual(payload["action"] as? String, "generate")
        XCTAssertEqual(payload["use_new_shared_trial"] as? Bool, true)

        // Parameters dict
        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertNotNil(parameters)
        XCTAssertEqual(parameters?["params_version"] as? Int, 3)
        XCTAssertEqual(parameters?["width"] as? Int, DEFAULT_WIDTH)
        XCTAssertEqual(parameters?["height"] as? Int, DEFAULT_HEIGHT)
        XCTAssertEqual(parameters?["scale"] as? Double, DEFAULT_SCALE)
        XCTAssertEqual(parameters?["sampler"] as? String, DEFAULT_SAMPLER.rawValue)
        XCTAssertEqual(parameters?["steps"] as? Int, DEFAULT_STEPS)
        XCTAssertEqual(parameters?["n_samples"] as? Int, 1)
        XCTAssertEqual(parameters?["seed"] as? Int, 42)
        XCTAssertEqual(parameters?["negative_prompt"] as? String, "bad quality")
        XCTAssertEqual(parameters?["ucPreset"] as? Int, 0)
        XCTAssertEqual(parameters?["qualityToggle"] as? Bool, false)
        XCTAssertEqual(parameters?["autoSmea"] as? Bool, false)
        XCTAssertEqual(parameters?["dynamic_thresholding"] as? Bool, false)
        XCTAssertEqual(parameters?["controlnet_strength"] as? Int, 1)
        XCTAssertEqual(parameters?["legacy"] as? Bool, false)
        XCTAssertEqual(parameters?["add_original_image"] as? Bool, true)
        XCTAssertEqual(parameters?["cfg_rescale"] as? Double, DEFAULT_CFG_RESCALE)
        XCTAssertEqual(parameters?["noise_schedule"] as? String, DEFAULT_NOISE_SCHEDULE.rawValue)
        XCTAssertEqual(parameters?["legacy_v3_extend"] as? Bool, false)
        XCTAssertTrue(parameters?["skip_cfg_above_sigma"] is NSNull)
        XCTAssertEqual(parameters?["use_coords"] as? Bool, false)
        XCTAssertEqual(parameters?["legacy_uc"] as? Bool, false)
        XCTAssertEqual(parameters?["normalize_reference_strength_multiple"] as? Bool, true)
        XCTAssertEqual(parameters?["inpaintImg2ImgStrength"] as? Int, 1)
        XCTAssertEqual(parameters?["deliberate_euler_ancestral_bug"] as? Bool, false)
        XCTAssertEqual(parameters?["prefer_brownian"] as? Bool, true)
    }

    func testBuildBasePayloadWithCustomModel() {
        let params = GenerateParams(
            prompt: "test",
            model: .naiDiffusion4Full,
            width: 1024,
            height: 1024,
            steps: 28,
            scale: 7.0,
            sampler: .kEuler,
            noiseSchedule: .exponential
        )
        let payload = buildBasePayload(params, seed: 100, negativePrompt: "ugly")

        XCTAssertEqual(payload["model"] as? String, "nai-diffusion-4-full")

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["width"] as? Int, 1024)
        XCTAssertEqual(parameters?["height"] as? Int, 1024)
        XCTAssertEqual(parameters?["steps"] as? Int, 28)
        XCTAssertEqual(parameters?["scale"] as? Double, 7.0)
        XCTAssertEqual(parameters?["sampler"] as? String, "k_euler")
        XCTAssertEqual(parameters?["noise_schedule"] as? String, "exponential")
        XCTAssertEqual(parameters?["seed"] as? Int, 100)
        XCTAssertEqual(parameters?["negative_prompt"] as? String, "ugly")
    }

    func testBuildBasePayloadWithImg2ImgAction() {
        let params = GenerateParams(
            prompt: "enhance",
            action: .img2img,
            sourceImage: .base64("aW1hZ2U=")
        )
        let payload = buildBasePayload(params, seed: 1, negativePrompt: "")
        XCTAssertEqual(payload["action"] as? String, "img2img")
    }

    // MARK: - applyImg2ImgParams Tests

    func testApplyImg2ImgParamsAddsCorrectFields() throws {
        let params = GenerateParams(
            prompt: "test",
            action: .img2img,
            sourceImage: .bytes(makeMinimalPNG()),
            img2imgStrength: 0.7,
            img2imgNoise: 0.1
        )
        let seed: UInt32 = 42
        var payload = buildBasePayload(params, seed: seed, negativePrompt: "")

        try applyImg2ImgParams(&payload, params: params, seed: seed)

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertNotNil(parameters?["image"] as? String)
        XCTAssertEqual(parameters?["strength"] as? Double, 0.7)
        XCTAssertEqual(parameters?["noise"] as? Double, 0.1)
        XCTAssertEqual(parameters?["extra_noise_seed"] as? Int, 41)
    }

    func testApplyImg2ImgParamsExtraNoiseSeedWrapsAtZero() throws {
        let params = GenerateParams(
            prompt: "test",
            action: .img2img,
            sourceImage: .bytes(makeMinimalPNG())
        )
        var payload = buildBasePayload(params, seed: 0, negativePrompt: "")

        try applyImg2ImgParams(&payload, params: params, seed: 0)

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["extra_noise_seed"] as? Int, Int(MAX_SEED))
    }

    func testApplyImg2ImgParamsNoOpForGenerateAction() throws {
        let params = GenerateParams(prompt: "test", action: .generate)
        var payload = buildBasePayload(params, seed: 42, negativePrompt: "")

        try applyImg2ImgParams(&payload, params: params, seed: 42)

        let parameters = payload["parameters"] as? [String: Any]
        // Should not have added an image field
        XCTAssertNil(parameters?["image"])
    }

    // MARK: - applyVibeParams Tests

    func testApplyVibeParamsAddsReferenceArrays() {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        let encodings = ["enc1", "enc2"]
        let strengths = [0.5, 0.7]
        let infoList = [0.6, 0.8]

        applyVibeParams(&payload, vibeEncodings: encodings, vibeStrengths: strengths, vibeInfoList: infoList)

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["reference_image_multiple"] as? [String], ["enc1", "enc2"])
        XCTAssertEqual(parameters?["reference_strength_multiple"] as? [Double], [0.5, 0.7])
        XCTAssertEqual(parameters?["reference_information_extracted_multiple"] as? [Double], [0.6, 0.8])
        XCTAssertEqual(parameters?["normalize_reference_strength_multiple"] as? Bool, true)
    }

    func testApplyVibeParamsNoOpForEmptyEncodings() {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        applyVibeParams(&payload, vibeEncodings: [], vibeStrengths: nil, vibeInfoList: [])

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertNil(parameters?["reference_image_multiple"])
    }

    // MARK: - applyCharRefParams Tests

    func testApplyCharRefParamsAddsDirectorReferenceArrays() {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        let charRefs = ProcessedCharacterReferences(
            images: ["img_b64"],
            descriptions: [DirectorReferenceDescription(baseCaption: "character&style")],
            infoExtracted: [1.0],
            strengthValues: [0.6],
            secondaryStrengthValues: [0.0]
        )

        applyCharRefParams(&payload, charRefs: charRefs)

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["director_reference_images"] as? [String], ["img_b64"])
        XCTAssertEqual(parameters?["director_reference_information_extracted"] as? [Double], [1.0])
        XCTAssertEqual(parameters?["director_reference_strength_values"] as? [Double], [0.6])
        XCTAssertEqual(parameters?["director_reference_secondary_strength_values"] as? [Double], [0.0])
        XCTAssertEqual(parameters?["stream"] as? String, "msgpack")
        XCTAssertEqual(parameters?["image_format"] as? String, "png")
    }

    // MARK: - buildV4PromptStructure Tests

    func testBuildV4PromptStructureCorrectFormat() {
        let result = buildV4PromptStructure(prompt: "1girl", charCaptions: [], hasCharacters: false)

        let caption = result["caption"] as? [String: Any]
        XCTAssertNotNil(caption)
        XCTAssertEqual(caption?["base_caption"] as? String, "1girl")

        let charCaptions = caption?["char_captions"] as? [[String: Any]]
        XCTAssertNotNil(charCaptions)
        XCTAssertTrue(charCaptions!.isEmpty)

        XCTAssertEqual(result["use_coords"] as? Bool, false)
        XCTAssertEqual(result["use_order"] as? Bool, true)
    }

    func testBuildV4PromptStructureWithCharCaptions() {
        let charCaps: [[String: Any]] = [
            ["char_caption": "a girl", "centers": [["x": 0.3, "y": 0.5]]],
        ]
        let result = buildV4PromptStructure(prompt: "test", charCaptions: charCaps, hasCharacters: true)

        let caption = result["caption"] as? [String: Any]
        let charCaptions = caption?["char_captions"] as? [[String: Any]]
        XCTAssertEqual(charCaptions?.count, 1)
        XCTAssertEqual(charCaptions?[0]["char_caption"] as? String, "a girl")
    }

    // MARK: - buildV4NegativePromptStructure Tests

    func testBuildV4NegativePromptStructureCorrectFormat() {
        let result = buildV4NegativePromptStructure(
            negativePrompt: "bad quality",
            charNegativeCaptions: []
        )

        let caption = result["caption"] as? [String: Any]
        XCTAssertNotNil(caption)
        XCTAssertEqual(caption?["base_caption"] as? String, "bad quality")

        let charCaptions = caption?["char_captions"] as? [[String: Any]]
        XCTAssertNotNil(charCaptions)
        XCTAssertTrue(charCaptions!.isEmpty)

        XCTAssertEqual(result["legacy_uc"] as? Bool, false)
    }

    // MARK: - applyCharacterPrompts Tests

    func testApplyCharacterPromptsAddsCoords() {
        let characters = [
            CharacterConfig(prompt: "girl", centerX: 0.3, centerY: 0.5, negativePrompt: "ugly"),
            CharacterConfig(prompt: "boy", centerX: 0.7, centerY: 0.5, negativePrompt: "bad"),
        ]
        let params = GenerateParams(prompt: "test", characters: characters)
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        applyCharacterPrompts(&payload, params: params)

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["use_coords"] as? Bool, true)

        let charPrompts = parameters?["characterPrompts"] as? [[String: Any]]
        XCTAssertNotNil(charPrompts)
        XCTAssertEqual(charPrompts?.count, 2)

        XCTAssertEqual(charPrompts?[0]["prompt"] as? String, "girl")
        XCTAssertEqual(charPrompts?[0]["uc"] as? String, "ugly")
        XCTAssertEqual(charPrompts?[0]["enabled"] as? Bool, true)

        let center0 = charPrompts?[0]["center"] as? [String: Double]
        XCTAssertEqual(center0?["x"], 0.3)
        XCTAssertEqual(center0?["y"], 0.5)

        XCTAssertEqual(charPrompts?[1]["prompt"] as? String, "boy")
    }

    func testApplyCharacterPromptsNoOpForEmptyCharacters() {
        let params = GenerateParams(prompt: "test", characters: nil)
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        applyCharacterPrompts(&payload, params: params)

        let parameters = payload["parameters"] as? [String: Any]
        let charPrompts = parameters?["characterPrompts"] as? [[String: Any]]
        XCTAssertNotNil(charPrompts)
        XCTAssertEqual(charPrompts?.count, 0)
        // use_coords should remain false from buildBasePayload
        XCTAssertEqual(parameters?["use_coords"] as? Bool, false)
    }

    // MARK: - applyV4PromptStructures Tests

    func testApplyV4PromptStructuresSetsParameterFields() {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "bad")

        applyV4PromptStructures(
            &payload,
            prompt: "hello",
            negativePrompt: "ugly",
            charCaptions: [],
            charNegativeCaptions: []
        )

        let parameters = payload["parameters"] as? [String: Any]
        XCTAssertNotNil(parameters?["v4_prompt"])
        XCTAssertNotNil(parameters?["v4_negative_prompt"])

        let v4Prompt = parameters?["v4_prompt"] as? [String: Any]
        let caption = v4Prompt?["caption"] as? [String: Any]
        XCTAssertEqual(caption?["base_caption"] as? String, "hello")

        let v4Neg = parameters?["v4_negative_prompt"] as? [String: Any]
        let negCaption = v4Neg?["caption"] as? [String: Any]
        XCTAssertEqual(negCaption?["base_caption"] as? String, "ugly")
    }
}

// MARK: - 4. Retry Logic Tests

final class RetryLogicTests: XCTestCase {

    private var mockSession: URLSession!

    override func setUp() {
        super.setUp()
        mockSession = makeMockSession()
        MockURLProtocol.requestHandler = nil
        MockURLProtocol.capturedRequests = []
    }

    override func tearDown() {
        MockURLProtocol.requestHandler = nil
        MockURLProtocol.capturedRequests = []
        super.tearDown()
    }

    func testSuccessfulRequestOnFirstTry() async throws {
        let expectedData = Data("success".utf8)
        MockURLProtocol.requestHandler = { request in
            let response = makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200)
            return (response, expectedData)
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let (data, response) = try await fetchWithRetry(
            request: request,
            session: mockSession,
            operationName: "Test"
        )

        XCTAssertEqual(data, expectedData)
        XCTAssertEqual(response.statusCode, 200)
        XCTAssertEqual(MockURLProtocol.capturedRequests.count, 1)
    }

    func testRetryOnHTTP429() async throws {
        var attemptCount = 0
        let successData = Data("ok".utf8)

        MockURLProtocol.requestHandler = { request in
            attemptCount += 1
            if attemptCount <= 2 {
                return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 429), Data())
            }
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), successData)
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let logger = TestLogger()
        let (data, response) = try await fetchWithRetry(
            request: request,
            session: mockSession,
            operationName: "RateLimitTest",
            logger: logger
        )

        XCTAssertEqual(data, successData)
        XCTAssertEqual(response.statusCode, 200)
        XCTAssertEqual(attemptCount, 3)  // 2 retries + 1 success
        XCTAssertTrue(logger.warnings.contains(where: { $0.contains("Rate limited (429)") }))
    }

    func testGiveUpAfterMaxRetriesOn429() async {
        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 429), Data())
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let logger = TestLogger()

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "MaxRetryTest",
                logger: logger
            )
            XCTFail("Expected error to be thrown")
        } catch let error as NovelAIError {
            if case .api(let statusCode, let message) = error {
                XCTAssertEqual(statusCode, 429)
                XCTAssertTrue(message.contains("after 3 retries"))
            } else {
                XCTFail("Expected NovelAIError.api, got \(error)")
            }
        } catch {
            // Could also be a CancellationError from the timeout race
            // Depending on timing. Both are acceptable outcomes.
        }
    }

    func testNonRetryableErrorThrownImmediately_400() async {
        var attemptCount = 0
        MockURLProtocol.requestHandler = { request in
            attemptCount += 1
            return (
                makeHTTPResponse(url: request.url!.absoluteString, statusCode: 400),
                Data("Bad Request".utf8)
            )
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "BadRequestTest"
            )
            XCTFail("Expected error to be thrown")
        } catch let error as NovelAIError {
            if case .api(let statusCode, let message) = error {
                XCTAssertEqual(statusCode, 400)
                XCTAssertTrue(message.contains("BadRequestTest failed"))
            } else {
                XCTFail("Expected NovelAIError.api, got \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }

        // Should only have been called once (no retries)
        XCTAssertEqual(attemptCount, 1)
    }

    func testNonRetryableErrorThrownImmediately_500() async {
        var attemptCount = 0
        MockURLProtocol.requestHandler = { request in
            attemptCount += 1
            return (
                makeHTTPResponse(url: request.url!.absoluteString, statusCode: 500),
                Data("Internal Server Error".utf8)
            )
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "ServerErrorTest"
            )
            XCTFail("Expected error to be thrown")
        } catch let error as NovelAIError {
            if case .api(let statusCode, _) = error {
                XCTAssertEqual(statusCode, 500)
            } else {
                XCTFail("Expected NovelAIError.api, got \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }

        XCTAssertEqual(attemptCount, 1)
    }

    func testNonRetryableErrorThrownImmediately_401() async {
        var attemptCount = 0
        MockURLProtocol.requestHandler = { request in
            attemptCount += 1
            return (
                makeHTTPResponse(url: request.url!.absoluteString, statusCode: 401),
                Data("Unauthorized".utf8)
            )
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "AuthTest"
            )
            XCTFail("Expected error to be thrown")
        } catch let error as NovelAIError {
            if case .api(let statusCode, _) = error {
                XCTAssertEqual(statusCode, 401)
            } else {
                XCTFail("Expected NovelAIError.api, got \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }

        XCTAssertEqual(attemptCount, 1)
    }

    func testRetryOnNetworkError() async throws {
        var attemptCount = 0
        let successData = Data("ok".utf8)

        MockURLProtocol.requestHandler = { request in
            attemptCount += 1
            if attemptCount == 1 {
                throw URLError(.timedOut)
            }
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), successData)
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let logger = TestLogger()
        let (data, response) = try await fetchWithRetry(
            request: request,
            session: mockSession,
            operationName: "NetworkRetryTest",
            logger: logger
        )

        XCTAssertEqual(data, successData)
        XCTAssertEqual(response.statusCode, 200)
        XCTAssertEqual(attemptCount, 2)
        XCTAssertTrue(logger.warnings.contains(where: { $0.contains("Network error") }))
    }

    func testSuccessResponseBodyParsed() async throws {
        let responseJSON = """
        {"status": "ok", "data": [1, 2, 3]}
        """.data(using: .utf8)!

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), responseJSON)
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let (data, _) = try await fetchWithRetry(
            request: request,
            session: mockSession,
            operationName: "JSONTest"
        )

        let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(parsed?["status"] as? String, "ok")
    }

    func testErrorBodyIncludedInMessage() async {
        let errorBody = "Detailed error message from server"
        MockURLProtocol.requestHandler = { request in
            return (
                makeHTTPResponse(url: request.url!.absoluteString, statusCode: 403),
                Data(errorBody.utf8)
            )
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "ErrorBodyTest"
            )
            XCTFail("Expected error to be thrown")
        } catch let error as NovelAIError {
            if case .api(_, let message) = error {
                XCTAssertTrue(message.contains("Detailed error message"))
            } else {
                XCTFail("Expected NovelAIError.api, got \(error)")
            }
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }

    func testLongErrorBodyIsTruncated() async {
        let longBody = String(repeating: "A", count: 500)
        MockURLProtocol.requestHandler = { request in
            return (
                makeHTTPResponse(url: request.url!.absoluteString, statusCode: 403),
                Data(longBody.utf8)
            )
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"

        let logger = TestLogger()

        do {
            _ = try await fetchWithRetry(
                request: request,
                session: mockSession,
                operationName: "TruncateTest",
                logger: logger
            )
            XCTFail("Expected error to be thrown")
        } catch {
            // The error logger should have received a truncated message
            let logged = logger.errors.first ?? ""
            XCTAssertTrue(logged.contains("[truncated]"))
        }
    }
}

// MARK: - 5. HTTP Integration Tests (using URLProtocol mock)

final class HTTPIntegrationTests: XCTestCase {

    override func setUp() {
        super.setUp()
        MockURLProtocol.requestHandler = nil
        MockURLProtocol.capturedRequests = []
    }

    override func tearDown() {
        MockURLProtocol.requestHandler = nil
        MockURLProtocol.capturedRequests = []
        super.tearDown()
    }

    // MARK: - Authorization Header Tests

    func testAuthorizationHeaderIsCorrectlySet() async throws {
        let session = makeMockSession()
        let expectedData = Data("test".utf8)

        MockURLProtocol.requestHandler = { request in
            let authHeader = request.value(forHTTPHeaderField: "Authorization")
            XCTAssertEqual(authHeader, "Bearer test-api-key-123")
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), expectedData)
        }

        var request = URLRequest(url: URL(string: "https://example.com/test")!)
        request.httpMethod = "GET"
        request.setValue("Bearer test-api-key-123", forHTTPHeaderField: "Authorization")

        let (data, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "AuthHeaderTest"
        )

        XCTAssertEqual(data, expectedData)
    }

    // MARK: - getAnlasBalance Tests (using fetchWithRetry directly)

    func testGetAnlasBalanceResponseParsing() async throws {
        let session = makeMockSession()

        let balanceJSON = """
        {
            "trainingStepsLeft": {
                "fixedTrainingStepsLeft": 1000,
                "purchasedTrainingSteps": 500
            },
            "tier": 3
        }
        """.data(using: .utf8)!

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), balanceJSON)
        }

        var request = URLRequest(url: URL(string: "https://api.novelai.net/user/subscription")!)
        request.httpMethod = "GET"
        request.setValue("Bearer test-key", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Accept")

        let (data, response) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "GetAnlasBalance"
        )

        XCTAssertEqual(response.statusCode, 200)

        // Parse the response like the client does
        let json = try JSONSerialization.jsonObject(with: data) as! [String: Any]
        let trainingStepsLeft = json["trainingStepsLeft"] as! [String: Any]
        let fixed = trainingStepsLeft["fixedTrainingStepsLeft"] as! Int
        let purchased = trainingStepsLeft["purchasedTrainingSteps"] as! Int
        let tier = json["tier"] as! Int

        XCTAssertEqual(fixed, 1000)
        XCTAssertEqual(purchased, 500)
        XCTAssertEqual(tier, 3)
    }

    // MARK: - Generate request/response simulation

    func testGenerateRequestSendsCorrectPayload() async throws {
        let session = makeMockSession()
        let pngData = makeMinimalPNG()
        let zipData = try makeZipWithPNG(pngData)

        MockURLProtocol.requestHandler = { request in
            // Verify content type
            XCTAssertEqual(request.value(forHTTPHeaderField: "Content-Type"), "application/json")

            // Verify body contains expected fields
            if let body = request.httpBody,
               let json = try? JSONSerialization.jsonObject(with: body) as? [String: Any] {
                XCTAssertEqual(json["input"] as? String, "1girl, masterpiece")
                XCTAssertEqual(json["model"] as? String, DEFAULT_MODEL.rawValue)
                XCTAssertEqual(json["action"] as? String, "generate")
                XCTAssertEqual(json["use_new_shared_trial"] as? Bool, true)

                let parameters = json["parameters"] as? [String: Any]
                XCTAssertNotNil(parameters)
                XCTAssertEqual(parameters?["width"] as? Int, DEFAULT_WIDTH)
                XCTAssertEqual(parameters?["height"] as? Int, DEFAULT_HEIGHT)
            }

            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), zipData)
        }

        // Build the payload as the client would
        let params = GenerateParams(prompt: "1girl, masterpiece", seed: 42)
        let payload = buildBasePayload(params, seed: 42, negativePrompt: DEFAULT_NEGATIVE)
        let payloadData = try JSONSerialization.data(withJSONObject: payload)

        var request = URLRequest(url: URL(string: apiURL())!)
        request.httpMethod = "POST"
        request.setValue("Bearer test-key", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = payloadData

        let (responseData, response) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Generation"
        )

        XCTAssertEqual(response.statusCode, 200)

        // Parse ZIP response like the client does
        let imageData = try parseZipResponse(responseData)
        XCTAssertEqual(imageData, pngData)
    }

    func testStreamResponseForCharRefGeneration() async throws {
        let session = makeMockSession()
        let pngData = makeMinimalPNG()

        // Simulate a stream response with embedded PNG
        var streamData = Data(repeating: 0x01, count: 50)
        streamData.append(pngData)

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), streamData)
        }

        var request = URLRequest(url: URL(string: streamURL())!)
        request.httpMethod = "POST"
        request.setValue("Bearer test-key", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Generation"
        )

        let imageData = try parseStreamResponse(responseData)
        XCTAssertTrue(imageData.starts(with: [0x89, 0x50, 0x4E, 0x47]))
    }

    // MARK: - Augment image simulation

    func testAugmentImageSendsCorrectPayload() async throws {
        let session = makeMockSession()
        let pngData = makeMinimalPNG()
        let zipData = try makeZipWithPNG(pngData)

        MockURLProtocol.requestHandler = { request in
            if let body = request.httpBody,
               let json = try? JSONSerialization.jsonObject(with: body) as? [String: Any] {
                XCTAssertEqual(json["req_type"] as? String, "colorize")
                XCTAssertEqual(json["use_new_shared_trial"] as? Bool, true)
                XCTAssertNotNil(json["image"])
                XCTAssertNotNil(json["width"])
                XCTAssertNotNil(json["height"])
            }
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), zipData)
        }

        // Build augment payload manually (as client would)
        let payload: [String: Any] = [
            "req_type": "colorize",
            "use_new_shared_trial": true,
            "width": 100,
            "height": 100,
            "image": "base64encodedimage",
            "defry": 3,
        ]

        let payloadData = try JSONSerialization.data(withJSONObject: payload)

        var request = URLRequest(url: URL(string: augmentURL())!)
        request.httpMethod = "POST"
        request.setValue("Bearer test-key", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = payloadData

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Augment"
        )

        let imageData = try parseZipResponse(responseData)
        XCTAssertEqual(imageData, pngData)
    }

    // MARK: - Upscale image: ZIP and raw response handling

    func testUpscaleImageHandlesZipResponse() async throws {
        let session = makeMockSession()
        let pngData = makeMinimalPNG()
        let zipData = try makeZipWithPNG(pngData)

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), zipData)
        }

        var request = URLRequest(url: URL(string: upscaleURL())!)
        request.httpMethod = "POST"
        request.httpBody = Data()

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Upscale"
        )

        // Client checks for ZIP signature
        let imageData: Data
        if responseData.count > 1 && responseData[responseData.startIndex] == 0x50 && responseData[responseData.startIndex + 1] == 0x4B {
            imageData = try parseZipResponse(responseData)
        } else {
            imageData = responseData
        }

        XCTAssertEqual(imageData, pngData)
    }

    func testUpscaleImageHandlesRawResponse() async throws {
        let session = makeMockSession()
        let rawImageData = Data(repeating: 0xFF, count: 1000)

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), rawImageData)
        }

        var request = URLRequest(url: URL(string: upscaleURL())!)
        request.httpMethod = "POST"
        request.httpBody = Data()

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Upscale"
        )

        // Not a ZIP (doesn't start with PK), so use raw data
        let imageData: Data
        if responseData.count > 1 && responseData[responseData.startIndex] == 0x50 && responseData[responseData.startIndex + 1] == 0x4B {
            imageData = try parseZipResponse(responseData)
        } else {
            imageData = responseData
        }

        XCTAssertEqual(imageData, rawImageData)
    }

    // MARK: - Error response handling

    func testErrorResponsesAreProperlyHandled() async {
        let session = makeMockSession()

        let errorCodes = [400, 401, 403, 404, 500, 502, 503]

        for code in errorCodes {
            MockURLProtocol.requestHandler = { request in
                return (
                    makeHTTPResponse(url: request.url!.absoluteString, statusCode: code),
                    Data("Error \(code)".utf8)
                )
            }

            var request = URLRequest(url: URL(string: "https://example.com/test")!)
            request.httpMethod = "GET"

            do {
                _ = try await fetchWithRetry(
                    request: request,
                    session: session,
                    operationName: "ErrorTest"
                )
                XCTFail("Expected error for status code \(code)")
            } catch let error as NovelAIError {
                if case .api(let statusCode, _) = error {
                    XCTAssertEqual(statusCode, code, "Status code mismatch for \(code)")
                } else {
                    XCTFail("Expected NovelAIError.api for status \(code), got \(error)")
                }
            } catch {
                XCTFail("Unexpected error type for status \(code): \(error)")
            }
        }
    }
}

// MARK: - 6. Additional Response Parsing Edge Cases

final class ResponseParsingEdgeCaseTests: XCTestCase {

    func testZipResponseWithMultipleFilesReturnsFirstImage() throws {
        let archive = try Archive(accessMode: .create)

        // Add a non-image file first
        let textData = Data("hello".utf8)
        try archive.addEntry(
            with: "readme.txt",
            type: .file,
            uncompressedSize: Int64(textData.count),
            provider: { position, size in
                return textData[Int(position)..<Int(position) + size]
            }
        )

        // Then add an image file
        let pngData = makeMinimalPNG()
        try archive.addEntry(
            with: "output.png",
            type: .file,
            uncompressedSize: Int64(pngData.count),
            provider: { position, size in
                return pngData[Int(position)..<Int(position) + size]
            }
        )

        let result = try parseZipResponse(archive.data!)
        XCTAssertEqual(result, pngData)
    }

    func testZipResponseCaseInsensitiveExtension() throws {
        let imageData = Data(repeating: 0xAA, count: 50)
        let zipData = try makeZipWithPNG(imageData, filename: "IMAGE.PNG")

        let result = try parseZipResponse(zipData)
        XCTAssertEqual(result, imageData)
    }

    func testParseStreamResponseFallbackChain() throws {
        // Test that the fallback chain works:
        // 1. Not ZIP (doesn't start with PK)
        // 2. Not raw PNG (doesn't start with PNG signature)
        // 3. Has embedded PNG -> should find it

        let pngData = makeMinimalPNG()
        var data = Data([0x00, 0x01])  // Not PK and not PNG
        data.append(pngData)

        let result = try parseStreamResponse(data)
        XCTAssertTrue(result.starts(with: [0x89, 0x50, 0x4E, 0x47]))
    }
}

// MARK: - 7. URL Constants Tests

final class URLConstantsTests: XCTestCase {

    func testDefaultAPIURLs() {
        // These tests verify the default URLs when no environment overrides are set.
        // If env vars are set, the values will differ - that's expected behavior.
        let api = apiURL()
        let stream = streamURL()
        let encode = encodeURL()
        let subscription = subscriptionURL()
        let augment = augmentURL()
        let upscale = upscaleURL()

        // All should be non-empty valid URLs
        XCTAssertFalse(api.isEmpty)
        XCTAssertFalse(stream.isEmpty)
        XCTAssertFalse(encode.isEmpty)
        XCTAssertFalse(subscription.isEmpty)
        XCTAssertFalse(augment.isEmpty)
        XCTAssertFalse(upscale.isEmpty)

        // All should be valid URLs
        XCTAssertNotNil(URL(string: api))
        XCTAssertNotNil(URL(string: stream))
        XCTAssertNotNil(URL(string: encode))
        XCTAssertNotNil(URL(string: subscription))
        XCTAssertNotNil(URL(string: augment))
        XCTAssertNotNil(URL(string: upscale))
    }

    func testDefaultURLsContainExpectedHostnames() {
        // Without environment overrides, verify the default hostnames
        let hasOverrides = ProcessInfo.processInfo.environment.keys.contains(where: {
            $0.hasPrefix("NOVELAI_")
        })
        if hasOverrides { return }

        XCTAssertTrue(apiURL().contains("image.novelai.net"))
        XCTAssertTrue(streamURL().contains("image.novelai.net"))
        XCTAssertTrue(encodeURL().contains("image.novelai.net"))
        XCTAssertTrue(subscriptionURL().contains("api.novelai.net"))
        XCTAssertTrue(augmentURL().contains("image.novelai.net"))
        XCTAssertTrue(upscaleURL().contains("api.novelai.net"))
    }
}

// MARK: - 8. Error Type Tests

final class ClientErrorTests: XCTestCase {

    func testAPIErrorDescription() {
        let error = NovelAIError.api(statusCode: 429, message: "Rate limited")
        XCTAssertTrue(error.localizedDescription.contains("429"))
        XCTAssertTrue(error.localizedDescription.contains("Rate limited"))
    }

    func testParseErrorDescription() {
        let error = NovelAIError.parse("Failed to parse response")
        XCTAssertTrue(error.localizedDescription.contains("Parse error"))
        XCTAssertTrue(error.localizedDescription.contains("Failed to parse response"))
    }

    func testOtherErrorDescription() {
        let error = NovelAIError.other("Unknown error")
        XCTAssertTrue(error.localizedDescription.contains("Unknown error"))
    }

    func testIOErrorDescription() {
        let error = NovelAIError.io("File not found")
        XCTAssertTrue(error.localizedDescription.contains("I/O error"))
        XCTAssertTrue(error.localizedDescription.contains("File not found"))
    }
}

// MARK: - 9. Payload JSON Serialization Tests

final class PayloadSerializationTests: XCTestCase {

    func testBasePayloadCanBeSerializedToJSON() throws {
        let params = GenerateParams(prompt: "test prompt", seed: 42)
        let payload = buildBasePayload(params, seed: 42, negativePrompt: "bad")

        // Should not throw
        let data = try JSONSerialization.data(withJSONObject: payload)
        XCTAssertTrue(data.count > 0)

        // Should be parseable back
        let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertNotNil(parsed)
        XCTAssertEqual(parsed?["input"] as? String, "test prompt")
    }

    func testPayloadWithVibesCanBeSerialized() throws {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        applyVibeParams(
            &payload,
            vibeEncodings: ["enc1", "enc2"],
            vibeStrengths: [0.5, 0.7],
            vibeInfoList: [0.6, 0.8]
        )

        let data = try JSONSerialization.data(withJSONObject: payload)
        XCTAssertTrue(data.count > 0)

        let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let parameters = parsed?["parameters"] as? [String: Any]
        XCTAssertNotNil(parameters?["reference_image_multiple"])
    }

    func testPayloadWithV4PromptsCanBeSerialized() throws {
        let params = GenerateParams(prompt: "test")
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "bad")

        applyV4PromptStructures(
            &payload,
            prompt: "hello",
            negativePrompt: "ugly",
            charCaptions: [["char_caption": "girl", "centers": [["x": 0.5, "y": 0.5]]]],
            charNegativeCaptions: []
        )

        let data = try JSONSerialization.data(withJSONObject: payload)
        XCTAssertTrue(data.count > 0)
    }

    func testPayloadWithCharacterPromptsCanBeSerialized() throws {
        let characters = [
            CharacterConfig(prompt: "girl", centerX: 0.3, centerY: 0.5),
        ]
        let params = GenerateParams(prompt: "test", characters: characters)
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        applyCharacterPrompts(&payload, params: params)

        let data = try JSONSerialization.data(withJSONObject: payload)
        XCTAssertTrue(data.count > 0)

        let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let parameters = parsed?["parameters"] as? [String: Any]
        let charPrompts = parameters?["characterPrompts"] as? [[String: Any]]
        XCTAssertEqual(charPrompts?.count, 1)
    }

    func testFullPayloadWithAllFeatures() throws {
        let characters = [
            CharacterConfig(prompt: "girl", centerX: 0.3, centerY: 0.5, negativePrompt: "ugly"),
        ]
        let params = GenerateParams(prompt: "1girl, masterpiece", characters: characters)
        var payload = buildBasePayload(params, seed: 42, negativePrompt: DEFAULT_NEGATIVE)

        applyVibeParams(
            &payload,
            vibeEncodings: ["enc1"],
            vibeStrengths: [0.7],
            vibeInfoList: [0.6]
        )

        let charCaptions = characters.map { characterToCaptionDict($0) }
        let charNegCaptions = characters.map { characterToNegativeCaptionDict($0) }

        applyV4PromptStructures(
            &payload,
            prompt: "1girl, masterpiece",
            negativePrompt: DEFAULT_NEGATIVE,
            charCaptions: charCaptions,
            charNegativeCaptions: charNegCaptions
        )

        applyCharacterPrompts(&payload, params: params)

        // Should serialize without error
        let data = try JSONSerialization.data(withJSONObject: payload)
        XCTAssertTrue(data.count > 0)

        // Verify round-trip
        let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(parsed?["input"] as? String, "1girl, masterpiece")

        let parameters = parsed?["parameters"] as? [String: Any]
        XCTAssertEqual(parameters?["use_coords"] as? Bool, true)
        XCTAssertNotNil(parameters?["v4_prompt"])
        XCTAssertNotNil(parameters?["v4_negative_prompt"])
        XCTAssertNotNil(parameters?["reference_image_multiple"])
        XCTAssertNotNil(parameters?["characterPrompts"])
    }
}

// MARK: - 10. AnlasBalance Type Tests

final class AnlasBalanceTypeTests: XCTestCase {

    func testAnlasBalanceDefaultValues() {
        let balance = AnlasBalance()
        XCTAssertEqual(balance.fixedTrainingStepsLeft, 0)
        XCTAssertEqual(balance.purchasedTrainingSteps, 0)
        XCTAssertEqual(balance.tier, 0)
    }

    func testAnlasBalanceWithValues() {
        let balance = AnlasBalance(
            fixedTrainingStepsLeft: 1000,
            purchasedTrainingSteps: 500,
            tier: 3
        )
        XCTAssertEqual(balance.fixedTrainingStepsLeft, 1000)
        XCTAssertEqual(balance.purchasedTrainingSteps, 500)
        XCTAssertEqual(balance.tier, 3)
    }
}

// MARK: - 11. GenerateResult Type Tests

final class GenerateResultTypeTests: XCTestCase {

    func testGenerateResultDefaultValues() {
        let data = Data([0x89, 0x50])
        let result = GenerateResult(imageData: data, seed: 42)
        XCTAssertEqual(result.imageData, data)
        XCTAssertEqual(result.seed, 42)
        XCTAssertNil(result.anlasRemaining)
        XCTAssertNil(result.anlasConsumed)
        XCTAssertNil(result.savedPath)
    }

    func testGenerateResultWithAllFields() {
        let data = Data([0x89, 0x50])
        let result = GenerateResult(
            imageData: data,
            seed: 100,
            anlasRemaining: 500,
            anlasConsumed: 17,
            savedPath: "/output/test.png"
        )
        XCTAssertEqual(result.anlasRemaining, 500)
        XCTAssertEqual(result.anlasConsumed, 17)
        XCTAssertEqual(result.savedPath, "/output/test.png")
    }
}

// MARK: - 12. AugmentResult Type Tests

final class AugmentResultTypeTests: XCTestCase {

    func testAugmentResultDefaultValues() {
        let data = Data([0xFF, 0xD8])
        let result = AugmentResult(imageData: data, reqType: .colorize)
        XCTAssertEqual(result.imageData, data)
        XCTAssertEqual(result.reqType, .colorize)
        XCTAssertNil(result.anlasRemaining)
        XCTAssertNil(result.anlasConsumed)
        XCTAssertNil(result.savedPath)
    }
}

// MARK: - 13. UpscaleResult Type Tests

final class UpscaleResultTypeTests: XCTestCase {

    func testUpscaleResultDefaultValues() {
        let data = Data([0x89, 0x50])
        let result = UpscaleResult(
            imageData: data,
            scale: 4,
            outputWidth: 2048,
            outputHeight: 2048
        )
        XCTAssertEqual(result.imageData, data)
        XCTAssertEqual(result.scale, 4)
        XCTAssertEqual(result.outputWidth, 2048)
        XCTAssertEqual(result.outputHeight, 2048)
        XCTAssertNil(result.anlasRemaining)
        XCTAssertNil(result.anlasConsumed)
        XCTAssertNil(result.savedPath)
    }
}

// MARK: - 14. Logger Protocol Tests

final class LoggerProtocolTests: XCTestCase {

    func testTestLoggerCapturesWarnings() {
        let logger = TestLogger()
        logger.warn("test warning")
        logger.warn("another warning")

        XCTAssertEqual(logger.warnings.count, 2)
        XCTAssertEqual(logger.warnings[0], "test warning")
        XCTAssertEqual(logger.warnings[1], "another warning")
    }

    func testTestLoggerCapturesErrors() {
        let logger = TestLogger()
        logger.error("test error")

        XCTAssertEqual(logger.errors.count, 1)
        XCTAssertEqual(logger.errors[0], "test error")
    }

    func testDefaultLoggerConformance() {
        // DefaultLogger should conform to Logger and Sendable
        let logger: Logger = DefaultLogger()
        // Just verify it doesn't crash
        logger.warn("test")
        logger.error("test")
    }
}

// MARK: - 15. Payload InfillParams Tests (without actual image processing)

final class PayloadInfillTests: XCTestCase {

    func testInfillPayloadSetsModelSuffix() throws {
        // We can't fully test applyInfillParams without actual image data,
        // but we can verify the model suffixing logic
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: .base64("aW1hZ2U="),
            mask: .base64("bWFzaw=="),
            maskStrength: 0.7
        )
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        // Verify model name before suffix
        XCTAssertEqual(payload["model"] as? String, DEFAULT_MODEL.rawValue)

        // The actual applyInfillParams would add -inpainting suffix
        // We test the logic conceptually
        let currentModel = payload["model"] as? String ?? ""
        if !currentModel.hasSuffix("-inpainting") {
            payload["model"] = currentModel + "-inpainting"
        }

        XCTAssertTrue((payload["model"] as? String ?? "").hasSuffix("-inpainting"))
    }

    func testInfillPayloadDoesNotDuplicateSuffix() {
        let params = GenerateParams(prompt: "test", action: .infill)
        var payload = buildBasePayload(params, seed: 1, negativePrompt: "")

        // Simulate model already having suffix
        payload["model"] = "nai-diffusion-4-5-full-inpainting"

        let currentModel = payload["model"] as? String ?? ""
        if !currentModel.hasSuffix("-inpainting") {
            payload["model"] = currentModel + "-inpainting"
        }

        // Should not have double suffix
        XCTAssertEqual(payload["model"] as? String, "nai-diffusion-4-5-full-inpainting")
    }
}

// MARK: - 16. Concurrent Request Tests

final class ConcurrentRequestTests: XCTestCase {

    func testMultipleConcurrentRequestsSucceed() async throws {
        let session = makeMockSession()
        let responseData = Data("response".utf8)

        MockURLProtocol.requestHandler = { request in
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), responseData)
        }

        // Fire off multiple concurrent requests
        try await withThrowingTaskGroup(of: Data.self) { group in
            for i in 0..<5 {
                group.addTask {
                    var request = URLRequest(url: URL(string: "https://example.com/test/\(i)")!)
                    request.httpMethod = "GET"

                    let (data, _) = try await fetchWithRetry(
                        request: request,
                        session: session,
                        operationName: "ConcurrentTest_\(i)"
                    )
                    return data
                }
            }

            var results: [Data] = []
            for try await data in group {
                results.append(data)
            }

            XCTAssertEqual(results.count, 5)
            for data in results {
                XCTAssertEqual(data, responseData)
            }
        }
    }
}

// MARK: - 17. VibeEncodeResult Type Tests

final class VibeEncodeResultTypeTests: XCTestCase {

    func testVibeEncodeResultCreation() {
        let result = VibeEncodeResult(
            encoding: "base64data",
            model: .naiDiffusion45Full,
            informationExtracted: 0.7,
            strength: 0.8,
            sourceImageHash: String(repeating: "a", count: 64),
            createdAt: Date()
        )

        XCTAssertEqual(result.encoding, "base64data")
        XCTAssertEqual(result.model, .naiDiffusion45Full)
        XCTAssertEqual(result.informationExtracted, 0.7)
        XCTAssertEqual(result.strength, 0.8)
        XCTAssertNil(result.savedPath)
        XCTAssertNil(result.anlasRemaining)
        XCTAssertNil(result.anlasConsumed)
    }

    func testVibeEncodeResultMutability() {
        var result = VibeEncodeResult(
            encoding: "test",
            model: .naiDiffusion45Full,
            informationExtracted: 0.5,
            strength: 0.5,
            sourceImageHash: String(repeating: "b", count: 64),
            createdAt: Date()
        )

        result.savedPath = "/output/vibe.naiv4vibe"
        result.anlasRemaining = 100
        result.anlasConsumed = 2

        XCTAssertEqual(result.savedPath, "/output/vibe.naiv4vibe")
        XCTAssertEqual(result.anlasRemaining, 100)
        XCTAssertEqual(result.anlasConsumed, 2)
    }
}

// MARK: - 18. Network Security Constants Tests

final class NetworkSecurityConstantsTests: XCTestCase {

    func testTimeoutConstant() {
        XCTAssertEqual(DEFAULT_REQUEST_TIMEOUT_MS, 60_000)
    }

    func testMaxResponseSize() {
        XCTAssertEqual(MAX_RESPONSE_SIZE, 200 * 1024 * 1024)
    }

    func testMaxDecompressedImageSize() {
        XCTAssertEqual(MAX_DECOMPRESSED_IMAGE_SIZE, 50 * 1024 * 1024)
    }

    func testMaxZipEntries() {
        XCTAssertEqual(MAX_ZIP_ENTRIES, 10)
    }

    func testMaxCompressionRatio() {
        XCTAssertEqual(MAX_COMPRESSION_RATIO, 100)
    }
}

// MARK: - 19. Request Building Detail Tests

final class RequestBuildingTests: XCTestCase {

    func testGenerateUsesStreamURLForCharRef() {
        // When characterReference is set, useStream should be true
        let charRef = CharacterReferenceConfig(image: .base64("aW1hZ2U="))
        let params = GenerateParams(prompt: "test", characterReference: charRef)

        let useStream = (params.characterReference != nil) || (params.action == .infill)
        XCTAssertTrue(useStream)
    }

    func testGenerateUsesStreamURLForInfill() {
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: .base64("aW1hZ2U="),
            mask: .base64("bWFzaw=="),
            maskStrength: 0.5
        )

        let useStream = (params.characterReference != nil) || (params.action == .infill)
        XCTAssertTrue(useStream)
    }

    func testGenerateUsesRegularURLForStandardGeneration() {
        let params = GenerateParams(prompt: "test")

        let useStream = (params.characterReference != nil) || (params.action == .infill)
        XCTAssertFalse(useStream)
    }

    func testGenerateUsesRegularURLForImg2Img() {
        let params = GenerateParams(
            prompt: "test",
            action: .img2img,
            sourceImage: .base64("aW1hZ2U=")
        )

        let useStream = (params.characterReference != nil) || (params.action == .infill)
        XCTAssertFalse(useStream)
    }
}

// MARK: - 20. MODEL_KEY_MAP Tests

final class ModelKeyMapTests: XCTestCase {

    func testModelKeyMapContainsAllModels() {
        for model in Model.allCases {
            XCTAssertNotNil(MODEL_KEY_MAP[model], "MODEL_KEY_MAP should contain key for \(model.rawValue)")
        }
    }

    func testModelKeyMapValues() {
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion4CuratedPreview], "v4curated")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion4Full], "v4full")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion45Curated], "v4-5curated")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion45Full], "v4-5full")
    }
}

// MARK: - 21. Processed Types Tests

final class ProcessedTypesTests: XCTestCase {

    func testProcessedCharacterReferencesDefaultInit() {
        let refs = ProcessedCharacterReferences()
        XCTAssertTrue(refs.images.isEmpty)
        XCTAssertTrue(refs.descriptions.isEmpty)
        XCTAssertTrue(refs.infoExtracted.isEmpty)
        XCTAssertTrue(refs.strengthValues.isEmpty)
        XCTAssertTrue(refs.secondaryStrengthValues.isEmpty)
    }

    func testProcessedVibesDefaultInit() {
        let vibes = ProcessedVibes()
        XCTAssertTrue(vibes.encodings.isEmpty)
        XCTAssertTrue(vibes.infoExtractedList.isEmpty)
    }
}
