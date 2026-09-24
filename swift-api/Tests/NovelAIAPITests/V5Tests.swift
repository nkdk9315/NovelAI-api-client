import XCTest
@testable import NovelAIAPI

/// V5 support: model helpers, validation, cost, usage and payload.
final class V5Tests: XCTestCase {

    private func v5(_ prompt: String = "1girl") -> GenerateParams {
        GenerateParams(prompt: prompt, model: .naiDiffusion5Full, width: 512, height: 768)
    }

    func testModelHelpers() {
        XCTAssertTrue(Model.naiDiffusion5Full.isV5)
        XCTAssertTrue(Model.naiDiffusion5Curated.isV5)
        XCTAssertFalse(Model.naiDiffusion45Full.isV5)
        XCTAssertEqual(Model.naiDiffusion5Full.inpaintModel, "nai-diffusion-5-full-inpainting")
        XCTAssertEqual(Model.naiDiffusion5Curated.inpaintModel, "nai-diffusion-4-5-curated-inpainting")
        XCTAssertEqual(Model.naiDiffusion45Full.inpaintModel, "nai-diffusion-4-5-full-inpainting")
        XCTAssertEqual(Model.naiDiffusion4CuratedPreview.inpaintModel, "nai-diffusion-4-curated-inpainting")
        XCTAssertEqual(Model.naiDiffusion5Full.maxTokens, 1471)
        XCTAssertEqual(Model.naiDiffusion5Curated.maxTokens, 703)
        XCTAssertEqual(Model.naiDiffusion45Full.maxTokens, 512)
    }

    func testV5RejectsVibesAndCharRef() {
        var p = v5()
        p.vibes = [.filePath("vibes/input1.naiv4vibe")]
        XCTAssertThrowsError(try p.validate())

        p = v5()
        p.characterReference = CharacterReferenceConfig(image: .bytes(makeMinimalPNG()))
        XCTAssertThrowsError(try p.validate())
    }

    func testCharacterLimitsDependOnModel() {
        let chars = { (n: Int) in (0..<n).map { _ in CharacterConfig(prompt: "girl") } }
        var p = v5("2girls")
        p.characters = chars(32)
        XCTAssertNoThrow(try p.validate())
        p.characters = chars(33)
        XCTAssertThrowsError(try p.validate())

        var q = GenerateParams(prompt: "2girls")
        q.characters = chars(6)
        XCTAssertNoThrow(try q.validate())
        q.characters = chars(7)
        XCTAssertThrowsError(try q.validate())
    }

    func testTransparentBackgroundOnlyOnV5() {
        var p = v5()
        p.transparentBackground = true
        XCTAssertNoThrow(try p.validate())
        XCTAssertEqual(p.effectivePrompt, "1girl, transparent background")

        let q = GenerateParams(prompt: "1girl", transparentBackground: true)
        XCTAssertThrowsError(try q.validate())

        var r = v5("transparent background, 1girl")
        r.transparentBackground = true
        XCTAssertEqual(r.effectivePrompt, "transparent background, 1girl")
    }

    func testEncodeVibeRejectsV5() {
        let p = EncodeVibeParams(image: .bytes(makeMinimalPNG()), model: .naiDiffusion5Full)
        XCTAssertThrowsError(try p.validate())
    }

    func testV5CostMultiplierMatchesMeasurement() throws {
        // Measured: 1088x1024, steps 1 → V5 6 Anlas, V4.5 4 Anlas
        let v5 = try calculateGenerationCost(GenerationCostParams(width: 1088, height: 1024, steps: 1, isV5: true))
        let v45 = try calculateGenerationCost(GenerationCostParams(width: 1088, height: 1024, steps: 1))
        XCTAssertEqual(v5.totalCost, 6)
        XCTAssertEqual(v5.modelMultiplier, 1.5)
        XCTAssertEqual(v45.totalCost, 4)
    }

    func testV5OpusFreeRequiresRemainingUsage() throws {
        let free = try calculateGenerationCost(GenerationCostParams(width: 832, height: 1216, steps: 23, tier: 3, isV5: true))
        XCTAssertEqual(free.totalCost, 0)
        let exhausted = try calculateGenerationCost(GenerationCostParams(width: 832, height: 1216, steps: 23, tier: 3, isV5: true, opusUsageExhausted: true))
        XCTAssertFalse(exhausted.isOpusFree)
        XCTAssertEqual(exhausted.totalCost, 26) // ceil(17 * 1.5)
        let v45 = try calculateGenerationCost(GenerationCostParams(width: 832, height: 1216, steps: 23, tier: 3, opusUsageExhausted: true))
        XCTAssertEqual(v45.totalCost, 0)
    }

    func testUsageSummary() {
        let s = summarizeOpusUsage(OpusUsage(percent: 100, isNegative: false, timeUntilNextPercent: 7888))
        XCTAssertEqual(s.remainingPercent, 100)
        XCTAssertEqual(s.refillPercentPerDay, 11)
        XCTAssertEqual(s.estimatedImagesRemaining, 1730)
        XCTAssertFalse(s.isLow)

        let empty = summarizeOpusUsage(OpusUsage(percent: 3, isNegative: true, timeUntilNextPercent: 0))
        XCTAssertEqual(empty.remainingPercent, 0)
        XCTAssertTrue(empty.isLow)
        XCTAssertTrue(empty.isExhausted)
    }

    func testV5Payload() {
        var p = v5()
        p.transparentBackground = true
        p.noiseSchedule = .exponential
        let payload = buildBasePayload(p, seed: 1, negativePrompt: "neg")
        let parameters = payload["parameters"] as? [String: Any] ?? [:]
        XCTAssertEqual(payload["input"] as? String, "1girl, transparent background")
        XCTAssertEqual(parameters["params_version"] as? Int, 4)
        XCTAssertEqual(parameters["noise_schedule"] as? String, "karras")
        XCTAssertEqual(parameters["straight_alpha"] as? Bool, true)
        XCTAssertEqual(parameters["tag_hint_transparent_background"] as? Bool, true)

        let v45 = buildBasePayload(GenerateParams(prompt: "1girl"), seed: 1, negativePrompt: "neg")
        let v45Params = v45["parameters"] as? [String: Any] ?? [:]
        XCTAssertEqual(v45Params["params_version"] as? Int, 3)
        XCTAssertNil(v45Params["straight_alpha"])
    }

    func testV5CuratedInfillUsesV45CuratedInpainting() throws {
        let png = makeMinimalPNG()
        let p = GenerateParams(
            prompt: "1girl", action: .infill, sourceImage: .bytes(png), mask: .bytes(png), maskStrength: 0.7,
            model: .naiDiffusion5Curated, width: 512, height: 768
        )
        var payload = buildBasePayload(p, seed: 1, negativePrompt: "neg")
        try applyInfillParams(&payload, params: p)
        XCTAssertEqual(payload["model"] as? String, "nai-diffusion-4-5-curated-inpainting")
    }

    func testImageFormatPayloadAndDetection() {
        var p = GenerateParams(prompt: "1girl")
        XCTAssertEqual((buildBasePayload(p, seed: 1, negativePrompt: "neg")["parameters"] as? [String: Any])?["image_format"] as? String, "png")
        p.imageFormat = .webp
        XCTAssertEqual((buildBasePayload(p, seed: 1, negativePrompt: "neg")["parameters"] as? [String: Any])?["image_format"] as? String, "webp")

        XCTAssertEqual(ImageFormat.detect(makeMinimalPNG()), .png)
        var webp = Data("RIFF".utf8) + Data([0, 0, 0, 0]) + Data("WEBPVP8L".utf8)
        webp.append(Data(repeating: 0, count: 8))
        XCTAssertEqual(ImageFormat.detect(webp), .webp)
        XCTAssertNil(ImageFormat.detect(Data("not an image".utf8)))
        XCTAssertEqual(GenerateResult(imageData: webp, seed: 1).imageFormat, .webp)
    }
}
