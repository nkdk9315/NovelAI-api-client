import XCTest

@testable import NovelAIAPI

// MARK: - 1. GenerateParams Defaults

final class GenerateParamsDefaultsTests: XCTestCase {

    func testBuilderWithJustPromptCreatesValidParams() throws {
        let params = try GenerateParams.builder(prompt: "a beautiful landscape").build()
        XCTAssertEqual(params.prompt, "a beautiful landscape")
        XCTAssertEqual(params.model, DEFAULT_MODEL)
        XCTAssertEqual(params.width, DEFAULT_WIDTH)
        XCTAssertEqual(params.height, DEFAULT_HEIGHT)
        XCTAssertEqual(params.steps, DEFAULT_STEPS)
        XCTAssertEqual(params.scale, DEFAULT_SCALE)
        XCTAssertEqual(params.sampler, DEFAULT_SAMPLER)
        XCTAssertEqual(params.noiseSchedule, DEFAULT_NOISE_SCHEDULE)
    }

    func testDefaultModelIsNaiDiffusion45Full() {
        XCTAssertEqual(DEFAULT_MODEL, Model.naiDiffusion45Full)
    }

    func testDefaultDimensions() {
        XCTAssertEqual(DEFAULT_WIDTH, 832)
        XCTAssertEqual(DEFAULT_HEIGHT, 1216)
    }

    func testDefaultSteps() {
        XCTAssertEqual(DEFAULT_STEPS, 23)
    }

    func testDefaultScale() {
        XCTAssertEqual(DEFAULT_SCALE, 5.0)
    }

    func testDefaultSampler() {
        XCTAssertEqual(DEFAULT_SAMPLER, Sampler.kEulerAncestral)
    }

    func testDefaultNoiseSchedule() {
        XCTAssertEqual(DEFAULT_NOISE_SCHEDULE, NoiseSchedule.karras)
    }

    func testDefaultAction() throws {
        let params = try GenerateParams.builder(prompt: "test").build()
        XCTAssertEqual(params.action, .generate)
    }

    func testDefaultCfgRescale() throws {
        let params = try GenerateParams.builder(prompt: "test").build()
        XCTAssertEqual(params.cfgRescale, DEFAULT_CFG_RESCALE)
    }

    func testDefaultSeedIsNil() throws {
        let params = try GenerateParams.builder(prompt: "test").build()
        XCTAssertNil(params.seed)
    }

    func testDirectInitWithJustPromptUsesDefaults() {
        let params = GenerateParams(prompt: "test prompt")
        XCTAssertEqual(params.model, DEFAULT_MODEL)
        XCTAssertEqual(params.width, DEFAULT_WIDTH)
        XCTAssertEqual(params.height, DEFAULT_HEIGHT)
        XCTAssertEqual(params.steps, DEFAULT_STEPS)
        XCTAssertEqual(params.scale, DEFAULT_SCALE)
        XCTAssertEqual(params.sampler, DEFAULT_SAMPLER)
        XCTAssertEqual(params.noiseSchedule, DEFAULT_NOISE_SCHEDULE)
        XCTAssertEqual(params.action, .generate)
        XCTAssertNil(params.seed)
        XCTAssertNil(params.sourceImage)
        XCTAssertNil(params.mask)
        XCTAssertNil(params.vibes)
        XCTAssertNil(params.characterReference)
        XCTAssertNil(params.characters)
    }
}

// MARK: - 2. Width/Height Validation

final class DimensionValidationTests: XCTestCase {

    func testValidMultiplesOf64Accepted() {
        let validDimensions = [64, 128, 256, 512, 640, 768, 832, 896, 1024, 1216, 1280, 1536, 2048]
        for dim in validDimensions {
            // Use dimension as width, keep height at a value that won't exceed MAX_PIXELS
            let params = GenerateParams(prompt: "test", width: dim, height: 64)
            XCTAssertNoThrow(try params.validate(), "Width \(dim) should be valid")

            let params2 = GenerateParams(prompt: "test", width: 64, height: dim)
            XCTAssertNoThrow(try params2.validate(), "Height \(dim) should be valid")
        }
    }

    func testNonMultiplesOf64Rejected() {
        let invalidDimensions = [65, 100, 127, 500, 700, 833, 1000, 1215]
        for dim in invalidDimensions {
            let params = GenerateParams(prompt: "test", width: dim, height: 64)
            XCTAssertThrowsError(try params.validate(), "Width \(dim) should be rejected") { error in
                guard case NovelAIError.validation(let msg) = error else {
                    XCTFail("Expected validation error, got \(error)")
                    return
                }
                XCTAssertTrue(msg.contains("multiple of 64"))
            }
        }
    }

    func testNonMultipleOf64HeightRejected() {
        let params = GenerateParams(prompt: "test", width: 128, height: 100)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("multiple of 64"))
        }
    }

    func testBelowMinDimensionRejected() {
        let params = GenerateParams(prompt: "test", width: 32, height: 64)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("width"))
            XCTAssertTrue(msg.contains("\(MIN_DIMENSION)"))
        }

        let params2 = GenerateParams(prompt: "test", width: 64, height: 0)
        XCTAssertThrowsError(try params2.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("height"))
        }
    }

    func testAboveMaxGenerationDimensionRejected() {
        // 2112 = 2048 + 64
        let params = GenerateParams(prompt: "test", width: 2112, height: 64)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("width"))
            XCTAssertTrue(msg.contains("\(MAX_GENERATION_DIMENSION)"))
        }

        let params2 = GenerateParams(prompt: "test", width: 64, height: 2112)
        XCTAssertThrowsError(try params2.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("height"))
        }
    }

    func testTotalPixelsExceedingMaxRejected() {
        // 2048 * 2048 = 4_194_304 > MAX_PIXELS (3_145_728)
        let params = GenerateParams(prompt: "test", width: 2048, height: 2048)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("pixels"))
            XCTAssertTrue(msg.contains("exceeds"))
        }
    }

    func testMaxPixelsBoundaryAccepted() {
        // 2048 * 1536 = 3_145_728 == MAX_PIXELS
        let params = GenerateParams(prompt: "test", width: 2048, height: 1536)
        XCTAssertNoThrow(try params.validate())
    }

    func testMinDimensionBoundaryAccepted() {
        let params = GenerateParams(prompt: "test", width: 64, height: 64)
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 3. Steps Validation

final class StepsValidationTests: XCTestCase {

    func testValidStepsAccepted() {
        let validSteps = [1, 2, 10, 23, 28, 49, 50]
        for step in validSteps {
            let params = GenerateParams(prompt: "test", steps: step)
            XCTAssertNoThrow(try params.validate(), "Steps \(step) should be valid")
        }
    }

    func testStepsBelowMinRejected() {
        let params = GenerateParams(prompt: "test", steps: 0)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("steps"))
            XCTAssertTrue(msg.contains("\(MIN_STEPS)"))
        }
    }

    func testStepsAboveMaxRejected() {
        let params = GenerateParams(prompt: "test", steps: 51)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("steps"))
            XCTAssertTrue(msg.contains("\(MAX_STEPS)"))
        }
    }

    func testNegativeStepsRejected() {
        let params = GenerateParams(prompt: "test", steps: -1)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
        }
    }
}

// MARK: - 4. Scale Validation

final class ScaleValidationTests: XCTestCase {

    func testValidScaleAccepted() {
        let validScales = [0.0, 0.5, 1.0, 5.0, 7.5, 10.0]
        for s in validScales {
            let params = GenerateParams(prompt: "test", scale: s)
            XCTAssertNoThrow(try params.validate(), "Scale \(s) should be valid")
        }
    }

    func testScaleBelowMinRejected() {
        let params = GenerateParams(prompt: "test", scale: -0.1)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("scale"))
        }
    }

    func testScaleAboveMaxRejected() {
        let params = GenerateParams(prompt: "test", scale: 10.1)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("scale"))
        }
    }

    func testScaleExactBoundaries() {
        let paramsMin = GenerateParams(prompt: "test", scale: MIN_SCALE)
        XCTAssertNoThrow(try paramsMin.validate())

        let paramsMax = GenerateParams(prompt: "test", scale: MAX_SCALE)
        XCTAssertNoThrow(try paramsMax.validate())
    }
}

// MARK: - 5. Seed Validation

final class SeedValidationTests: XCTestCase {

    func testValidSeedAccepted() {
        let validSeeds: [UInt32] = [0, 1, 42, 1000, MAX_SEED]
        for s in validSeeds {
            let params = GenerateParams(prompt: "test", seed: s)
            XCTAssertNoThrow(try params.validate(), "Seed \(s) should be valid")
        }
    }

    func testNilSeedAccepted() {
        let params = GenerateParams(prompt: "test", seed: nil)
        XCTAssertNoThrow(try params.validate())
    }

    func testMaxSeedBoundary() {
        // UInt32.max is the same as MAX_SEED, so there's no way to exceed it
        // with UInt32 type. This test verifies the constant value.
        XCTAssertEqual(MAX_SEED, UInt32.max)

        let params = GenerateParams(prompt: "test", seed: MAX_SEED)
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 6. Enum Validation

final class EnumValidationTests: XCTestCase {

    func testAllModelCasesWork() {
        for model in Model.allCases {
            let params = GenerateParams(prompt: "test", model: model)
            XCTAssertNoThrow(try params.validate(), "Model \(model.rawValue) should be valid")
        }
    }

    func testModelRawValues() {
        XCTAssertEqual(Model.naiDiffusion4CuratedPreview.rawValue, "nai-diffusion-4-curated-preview")
        XCTAssertEqual(Model.naiDiffusion4Full.rawValue, "nai-diffusion-4-full")
        XCTAssertEqual(Model.naiDiffusion45Curated.rawValue, "nai-diffusion-4-5-curated")
        XCTAssertEqual(Model.naiDiffusion45Full.rawValue, "nai-diffusion-4-5-full")
    }

    func testAllSamplerCasesWork() {
        for sampler in Sampler.allCases {
            let params = GenerateParams(prompt: "test", sampler: sampler)
            XCTAssertNoThrow(try params.validate(), "Sampler \(sampler.rawValue) should be valid")
        }
    }

    func testSamplerRawValues() {
        XCTAssertEqual(Sampler.kEuler.rawValue, "k_euler")
        XCTAssertEqual(Sampler.kEulerAncestral.rawValue, "k_euler_ancestral")
        XCTAssertEqual(Sampler.kDpmpp2sAncestral.rawValue, "k_dpmpp_2s_ancestral")
        XCTAssertEqual(Sampler.kDpmpp2mSde.rawValue, "k_dpmpp_2m_sde")
        XCTAssertEqual(Sampler.kDpmpp2m.rawValue, "k_dpmpp_2m")
        XCTAssertEqual(Sampler.kDpmppSde.rawValue, "k_dpmpp_sde")
    }

    func testAllNoiseScheduleCasesWork() {
        for schedule in NoiseSchedule.allCases {
            let params = GenerateParams(prompt: "test", noiseSchedule: schedule)
            XCTAssertNoThrow(try params.validate(), "NoiseSchedule \(schedule.rawValue) should be valid")
        }
    }

    func testNoiseScheduleRawValues() {
        XCTAssertEqual(NoiseSchedule.karras.rawValue, "karras")
        XCTAssertEqual(NoiseSchedule.exponential.rawValue, "exponential")
        XCTAssertEqual(NoiseSchedule.polyexponential.rawValue, "polyexponential")
    }

    func testAllGenerateActionCases() {
        XCTAssertEqual(GenerateAction.allCases.count, 3)
        XCTAssertEqual(GenerateAction.generate.rawValue, "generate")
        XCTAssertEqual(GenerateAction.img2img.rawValue, "img2img")
        XCTAssertEqual(GenerateAction.infill.rawValue, "infill")
    }

    func testAllAugmentReqTypeCases() {
        XCTAssertEqual(AugmentReqType.allCases.count, 6)
        XCTAssertEqual(AugmentReqType.colorize.rawValue, "colorize")
        XCTAssertEqual(AugmentReqType.declutter.rawValue, "declutter")
        XCTAssertEqual(AugmentReqType.emotion.rawValue, "emotion")
        XCTAssertEqual(AugmentReqType.sketch.rawValue, "sketch")
        XCTAssertEqual(AugmentReqType.lineart.rawValue, "lineart")
        XCTAssertEqual(AugmentReqType.bgRemoval.rawValue, "bg-removal")
    }

    func testAllCharRefModeCases() {
        XCTAssertEqual(CharRefMode.allCases.count, 3)
        XCTAssertEqual(CharRefMode.character.rawValue, "character")
        XCTAssertEqual(CharRefMode.characterAndStyle.rawValue, "character&style")
        XCTAssertEqual(CharRefMode.style.rawValue, "style")
    }

    func testEmotionKeywordHas24Cases() {
        XCTAssertEqual(EmotionKeyword.allCases.count, 24)
    }
}

// MARK: - 7. Action Dependencies

final class ActionDependencyTests: XCTestCase {

    func testImg2imgWithoutSourceImageErrors() {
        let params = GenerateParams(prompt: "test", action: .img2img, sourceImage: nil)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("source_image"))
            XCTAssertTrue(msg.contains("img2img"))
        }
    }

    func testInfillWithoutSourceImageErrors() {
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: nil,
            mask: .base64("bWFzaw=="),
            maskStrength: 0.5
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("source_image"))
            XCTAssertTrue(msg.contains("infill"))
        }
    }

    func testInfillWithoutMaskErrors() {
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: .base64("aW1hZ2U="),
            mask: nil,
            maskStrength: 0.5
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("mask"))
            XCTAssertTrue(msg.contains("infill"))
        }
    }

    func testInfillWithoutMaskStrengthErrors() {
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: .base64("aW1hZ2U="),
            mask: .base64("bWFzaw=="),
            maskStrength: nil
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("mask_strength"))
            XCTAssertTrue(msg.contains("infill"))
        }
    }

    func testMaskWithoutInfillActionErrors() {
        let params = GenerateParams(
            prompt: "test",
            action: .generate,
            mask: .base64("bWFzaw==")
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("mask"))
            XCTAssertTrue(msg.contains("infill"))
        }
    }

    func testImg2imgWithSourceImageAccepted() {
        let params = GenerateParams(
            prompt: "test",
            action: .img2img,
            sourceImage: .base64("aW1hZ2U=")
        )
        XCTAssertNoThrow(try params.validate())
    }

    func testInfillWithAllRequiredFieldsAccepted() {
        let params = GenerateParams(
            prompt: "test",
            action: .infill,
            sourceImage: .base64("aW1hZ2U="),
            mask: .base64("bWFzaw=="),
            maskStrength: 0.5
        )
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 8. Vibe Parameter Validation

final class VibeValidationTests: XCTestCase {

    private func makeVibeItem() -> VibeItem {
        return .filePath("/path/to/vibe.png")
    }

    func testVibesWithMismatchedStrengthsCountErrors() {
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem(), makeVibeItem()],
            vibeStrengths: [0.5]  // count mismatch: 2 vibes vs 1 strength
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Mismatch"))
            XCTAssertTrue(msg.contains("vibe_strengths"))
        }
    }

    func testVibesWithMismatchedInfoExtractedCountErrors() {
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem(), makeVibeItem()],
            vibeInfoExtracted: [0.5, 0.6, 0.7]  // count mismatch: 2 vibes vs 3 info
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Mismatch"))
            XCTAssertTrue(msg.contains("vibe_info_extracted"))
        }
    }

    func testVibeStrengthsWithoutVibesErrors() {
        let params = GenerateParams(
            prompt: "test",
            vibes: nil,
            vibeStrengths: [0.5]
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("vibe_strengths"))
            XCTAssertTrue(msg.contains("without vibes"))
        }
    }

    func testVibeInfoExtractedWithoutVibesErrors() {
        let params = GenerateParams(
            prompt: "test",
            vibes: nil,
            vibeInfoExtracted: [0.5]
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("vibe_info_extracted"))
            XCTAssertTrue(msg.contains("without vibes"))
        }
    }

    func testVibesAndCharacterReferenceMutuallyExclusive() {
        let charRef = CharacterReferenceConfig(image: .base64("aW1hZ2U="))
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem()],
            characterReference: charRef
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("vibes"))
            XCTAssertTrue(msg.contains("character_reference"))
        }
    }

    func testVibesWithMatchingStrengthsAccepted() {
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem(), makeVibeItem()],
            vibeStrengths: [0.5, 0.7]
        )
        XCTAssertNoThrow(try params.validate())
    }

    func testVibesWithMatchingInfoExtractedAccepted() {
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem()],
            vibeInfoExtracted: [0.6]
        )
        XCTAssertNoThrow(try params.validate())
    }

    func testVibesWithoutOptionalArraysAccepted() {
        let params = GenerateParams(
            prompt: "test",
            vibes: [makeVibeItem()],
            vibeStrengths: nil,
            vibeInfoExtracted: nil
        )
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 9. save_path / save_dir Mutual Exclusion

final class SaveOptionsTests: XCTestCase {

    func testBothSavePathAndSaveDirErrors() {
        let params = GenerateParams(
            prompt: "test",
            savePath: "/output/image.png",
            saveDir: "/output/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_path"))
            XCTAssertTrue(msg.contains("save_dir"))
        }
    }

    func testOnlySavePathAccepted() {
        let params = GenerateParams(prompt: "test", savePath: "/output/image.png")
        XCTAssertNoThrow(try params.validate())
    }

    func testOnlySaveDirAccepted() {
        let params = GenerateParams(prompt: "test", saveDir: "/output/")
        XCTAssertNoThrow(try params.validate())
    }

    func testNeitherSavePathNorSaveDirAccepted() {
        let params = GenerateParams(prompt: "test")
        XCTAssertNoThrow(try params.validate())
    }

    func testPathTraversalInSavePathRejected() {
        let params = GenerateParams(prompt: "test", savePath: "/output/../etc/image.png")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
            XCTAssertTrue(msg.contains("path traversal"))
        }
    }

    func testPathTraversalInSaveDirRejected() {
        let params = GenerateParams(prompt: "test", saveDir: "/output/../secret/")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testSafePathAccepted() {
        let params = GenerateParams(prompt: "test", savePath: "/output/subdir/image.png")
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 10. Character Validation

final class CharacterValidationTests: XCTestCase {

    func testValidCharacterConfigAccepted() {
        let character = CharacterConfig(prompt: "a girl", centerX: 0.5, centerY: 0.5)
        XCTAssertNoThrow(try character.validate())
    }

    func testCharacterConfigDefaultValues() {
        let character = CharacterConfig(prompt: "a girl")
        XCTAssertEqual(character.centerX, 0.5)
        XCTAssertEqual(character.centerY, 0.5)
        XCTAssertEqual(character.negativePrompt, "")
    }

    func testEmptyPromptRejected() {
        let character = CharacterConfig(prompt: "")
        XCTAssertThrowsError(try character.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
            XCTAssertTrue(msg.contains("empty"))
        }
    }

    func testCenterXOutOfRangeRejected() {
        let charTooLow = CharacterConfig(prompt: "a girl", centerX: -0.1)
        XCTAssertThrowsError(try charTooLow.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("center_x"))
        }

        let charTooHigh = CharacterConfig(prompt: "a girl", centerX: 1.1)
        XCTAssertThrowsError(try charTooHigh.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("center_x"))
        }
    }

    func testCenterYOutOfRangeRejected() {
        let charTooLow = CharacterConfig(prompt: "a girl", centerY: -0.1)
        XCTAssertThrowsError(try charTooLow.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("center_y"))
        }

        let charTooHigh = CharacterConfig(prompt: "a girl", centerY: 1.5)
        XCTAssertThrowsError(try charTooHigh.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("center_y"))
        }
    }

    func testCenterBoundaryValuesAccepted() {
        let charMin = CharacterConfig(prompt: "a girl", centerX: 0.0, centerY: 0.0)
        XCTAssertNoThrow(try charMin.validate())

        let charMax = CharacterConfig(prompt: "a girl", centerX: 1.0, centerY: 1.0)
        XCTAssertNoThrow(try charMax.validate())
    }

    func testCharacterInGenerateParamsValidated() {
        let invalidChar = CharacterConfig(prompt: "")
        let params = GenerateParams(prompt: "test", characters: [invalidChar])
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
        }
    }

    func testValidCharacterInGenerateParamsAccepted() {
        let validChar = CharacterConfig(prompt: "a girl", centerX: 0.3, centerY: 0.7)
        let params = GenerateParams(prompt: "test", characters: [validChar])
        XCTAssertNoThrow(try params.validate())
    }
}

// MARK: - 11. EncodeVibeParams Validation

final class EncodeVibeParamsValidationTests: XCTestCase {

    func testValidParamsAccepted() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            informationExtracted: 0.7,
            strength: 0.7
        )
        XCTAssertNoThrow(try params.validate())
    }

    func testInformationExtractedOutOfRangeRejected() {
        let paramsTooLow = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            informationExtracted: -0.1
        )
        XCTAssertThrowsError(try paramsTooLow.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("information_extracted"))
        }

        let paramsTooHigh = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            informationExtracted: 1.1
        )
        XCTAssertThrowsError(try paramsTooHigh.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("information_extracted"))
        }
    }

    func testStrengthOutOfRangeRejected() {
        let paramsTooLow = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            strength: -0.1
        )
        XCTAssertThrowsError(try paramsTooLow.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("strength"))
        }

        let paramsTooHigh = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            strength: 1.1
        )
        XCTAssertThrowsError(try paramsTooHigh.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("strength"))
        }
    }

    func testSavePathAndSaveDirTogetherRejected() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            savePath: "/output/vibe.bin",
            saveDir: "/output/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_path"))
            XCTAssertTrue(msg.contains("save_dir"))
        }
    }

    func testSaveFilenameWithoutSaveDirRejected() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            saveFilename: "vibe.bin"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_filename"))
            XCTAssertTrue(msg.contains("save_dir"))
        }
    }

    func testSaveFilenameWithSavePathRejected() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            savePath: "/output/vibe.bin",
            saveFilename: "vibe.bin"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_filename"))
            XCTAssertTrue(msg.contains("save_path"))
        }
    }

    func testSaveFilenameWithSaveDirAccepted() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            saveDir: "/output/",
            saveFilename: "vibe.bin"
        )
        XCTAssertNoThrow(try params.validate())
    }

    func testPathTraversalInSavePathRejected() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            savePath: "/output/../etc/vibe.bin"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testPathTraversalInSaveDirRejected() {
        let params = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            saveDir: "/output/../secret/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testBoundaryValuesAccepted() {
        let paramsMin = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            informationExtracted: 0.0,
            strength: 0.0
        )
        XCTAssertNoThrow(try paramsMin.validate())

        let paramsMax = EncodeVibeParams(
            image: .base64("aW1hZ2U="),
            informationExtracted: 1.0,
            strength: 1.0
        )
        XCTAssertNoThrow(try paramsMax.validate())
    }

    func testEmptyImageRejected() {
        let params = EncodeVibeParams(image: .base64(""))
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("empty"))
        }
    }
}

// MARK: - 12. AugmentParams Validation

final class AugmentParamsValidationTests: XCTestCase {

    private let dummyImage: ImageInput = .base64("aW1hZ2U=")

    // MARK: Simple types accepted

    func testDeclutterAccepted() {
        let params = AugmentParams(reqType: .declutter, image: dummyImage)
        XCTAssertNoThrow(try params.validate())
    }

    func testSketchAccepted() {
        let params = AugmentParams(reqType: .sketch, image: dummyImage)
        XCTAssertNoThrow(try params.validate())
    }

    func testLineartAccepted() {
        let params = AugmentParams(reqType: .lineart, image: dummyImage)
        XCTAssertNoThrow(try params.validate())
    }

    func testBgRemovalAccepted() {
        let params = AugmentParams(reqType: .bgRemoval, image: dummyImage)
        XCTAssertNoThrow(try params.validate())
    }

    // MARK: Colorize requires defry

    func testColorizeWithoutDefryErrors() {
        let params = AugmentParams(reqType: .colorize, image: dummyImage)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
            XCTAssertTrue(msg.contains("colorize"))
        }
    }

    func testColorizeWithDefryAccepted() {
        let params = AugmentParams(reqType: .colorize, image: dummyImage, defry: 3)
        XCTAssertNoThrow(try params.validate())
    }

    // MARK: Emotion requires defry and prompt

    func testEmotionWithoutDefryErrors() {
        let params = AugmentParams(reqType: .emotion, image: dummyImage, prompt: "happy")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    func testEmotionWithoutPromptErrors() {
        let params = AugmentParams(reqType: .emotion, image: dummyImage, defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
            XCTAssertTrue(msg.contains("emotion"))
        }
    }

    func testEmotionWithEmptyPromptErrors() {
        let params = AugmentParams(reqType: .emotion, image: dummyImage, prompt: "", defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
        }
    }

    // MARK: All valid emotion keywords

    func testAllValidEmotionKeywordsAccepted() {
        for keyword in EmotionKeyword.allCases {
            let params = AugmentParams(
                reqType: .emotion,
                image: dummyImage,
                prompt: keyword.rawValue,
                defry: 3
            )
            XCTAssertNoThrow(
                try params.validate(),
                "Emotion keyword '\(keyword.rawValue)' should be accepted"
            )
        }
    }

    func testInvalidEmotionKeywordRejected() {
        let params = AugmentParams(
            reqType: .emotion,
            image: dummyImage,
            prompt: "invalid_emotion",
            defry: 3
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("Invalid emotion keyword"))
            XCTAssertTrue(msg.contains("invalid_emotion"))
        }
    }

    // MARK: Prompt not allowed for simple types

    func testPromptNotAllowedForDeclutter() {
        let params = AugmentParams(reqType: .declutter, image: dummyImage, prompt: "some prompt")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
            XCTAssertTrue(msg.contains("declutter"))
        }
    }

    func testPromptNotAllowedForSketch() {
        let params = AugmentParams(reqType: .sketch, image: dummyImage, prompt: "some prompt")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
        }
    }

    func testPromptNotAllowedForLineart() {
        let params = AugmentParams(reqType: .lineart, image: dummyImage, prompt: "some prompt")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
        }
    }

    func testPromptNotAllowedForBgRemoval() {
        let params = AugmentParams(reqType: .bgRemoval, image: dummyImage, prompt: "some prompt")
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("prompt"))
        }
    }

    // MARK: Defry not allowed for simple types

    func testDefryNotAllowedForDeclutter() {
        let params = AugmentParams(reqType: .declutter, image: dummyImage, defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    func testDefryNotAllowedForSketch() {
        let params = AugmentParams(reqType: .sketch, image: dummyImage, defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    func testDefryNotAllowedForLineart() {
        let params = AugmentParams(reqType: .lineart, image: dummyImage, defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    func testDefryNotAllowedForBgRemoval() {
        let params = AugmentParams(reqType: .bgRemoval, image: dummyImage, defry: 3)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    // MARK: Defry range (0-5)

    func testDefryValidRangeAccepted() {
        for d in 0...5 {
            let params = AugmentParams(reqType: .colorize, image: dummyImage, defry: d)
            XCTAssertNoThrow(try params.validate(), "Defry \(d) should be valid")
        }
    }

    func testDefryBelowMinRejected() {
        let params = AugmentParams(reqType: .colorize, image: dummyImage, defry: -1)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    func testDefryAboveMaxRejected() {
        let params = AugmentParams(reqType: .colorize, image: dummyImage, defry: 6)
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("defry"))
        }
    }

    // MARK: Save path/dir mutual exclusion

    func testAugmentSavePathAndSaveDirMutualExclusion() {
        let params = AugmentParams(
            reqType: .declutter,
            image: dummyImage,
            savePath: "/output/image.png",
            saveDir: "/output/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_path"))
            XCTAssertTrue(msg.contains("save_dir"))
        }
    }

    func testAugmentPathTraversalRejected() {
        let params = AugmentParams(
            reqType: .declutter,
            image: dummyImage,
            savePath: "/output/../etc/image.png"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testAugmentEmptyImageRejected() {
        let params = AugmentParams(reqType: .declutter, image: .base64(""))
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("empty"))
        }
    }
}

// MARK: - 13. UpscaleParams Validation

final class UpscaleParamsValidationTests: XCTestCase {

    private let dummyImage: ImageInput = .base64("aW1hZ2U=")

    func testScale2Accepted() {
        let params = UpscaleParams(image: dummyImage, scale: 2)
        XCTAssertNoThrow(try params.validate())
    }

    func testScale4Accepted() {
        let params = UpscaleParams(image: dummyImage, scale: 4)
        XCTAssertNoThrow(try params.validate())
    }

    func testDefaultScaleIs4() {
        let params = UpscaleParams(image: dummyImage)
        XCTAssertEqual(params.scale, DEFAULT_UPSCALE_SCALE)
        XCTAssertEqual(params.scale, 4)
        XCTAssertNoThrow(try params.validate())
    }

    func testInvalidScaleRejected() {
        let invalidScales = [0, 1, 3, 5, 6, 8, -1, 10]
        for s in invalidScales {
            let params = UpscaleParams(image: dummyImage, scale: s)
            XCTAssertThrowsError(try params.validate(), "Scale \(s) should be rejected") { error in
                guard case NovelAIError.validation(let msg) = error else {
                    XCTFail("Expected validation error for scale \(s), got \(error)")
                    return
                }
                XCTAssertTrue(msg.contains("scale"))
            }
        }
    }

    func testUpscaleSavePathAndSaveDirMutualExclusion() {
        let params = UpscaleParams(
            image: dummyImage,
            savePath: "/output/upscaled.png",
            saveDir: "/output/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("save_path"))
            XCTAssertTrue(msg.contains("save_dir"))
        }
    }

    func testUpscalePathTraversalInSavePathRejected() {
        let params = UpscaleParams(
            image: dummyImage,
            savePath: "/output/../etc/upscaled.png"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testUpscalePathTraversalInSaveDirRejected() {
        let params = UpscaleParams(
            image: dummyImage,
            scale: 2,
            saveDir: "/output/../secret/"
        )
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains(".."))
        }
    }

    func testUpscaleOnlySavePathAccepted() {
        let params = UpscaleParams(image: dummyImage, savePath: "/output/upscaled.png")
        XCTAssertNoThrow(try params.validate())
    }

    func testUpscaleOnlySaveDirAccepted() {
        let params = UpscaleParams(image: dummyImage, saveDir: "/output/")
        XCTAssertNoThrow(try params.validate())
    }

    func testUpscaleEmptyImageRejected() {
        let params = UpscaleParams(image: .base64(""))
        XCTAssertThrowsError(try params.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("empty"))
        }
    }
}

// MARK: - 14. Helper Functions

final class HelperFunctionTests: XCTestCase {

    func testCharacterToCaptionDictProducesCorrectOutput() {
        let config = CharacterConfig(
            prompt: "a girl with blue hair",
            centerX: 0.3,
            centerY: 0.7,
            negativePrompt: "ugly"
        )
        let dict = characterToCaptionDict(config)

        XCTAssertEqual(dict["char_caption"] as? String, "a girl with blue hair")

        let centers = dict["centers"] as? [[String: Double]]
        XCTAssertNotNil(centers)
        XCTAssertEqual(centers?.count, 1)
        XCTAssertEqual(centers?[0]["x"], 0.3)
        XCTAssertEqual(centers?[0]["y"], 0.7)
    }

    func testCharacterToNegativeCaptionDictProducesCorrectOutput() {
        let config = CharacterConfig(
            prompt: "a girl with blue hair",
            centerX: 0.3,
            centerY: 0.7,
            negativePrompt: "ugly, bad quality"
        )
        let dict = characterToNegativeCaptionDict(config)

        XCTAssertEqual(dict["char_caption"] as? String, "ugly, bad quality")

        let centers = dict["centers"] as? [[String: Double]]
        XCTAssertNotNil(centers)
        XCTAssertEqual(centers?.count, 1)
        XCTAssertEqual(centers?[0]["x"], 0.3)
        XCTAssertEqual(centers?[0]["y"], 0.7)
    }

    func testCaptionDictWithDefaultNegativePrompt() {
        let config = CharacterConfig(prompt: "a boy")
        let negDict = characterToNegativeCaptionDict(config)

        // Default negative prompt is ""
        XCTAssertEqual(negDict["char_caption"] as? String, "")

        let centers = negDict["centers"] as? [[String: Double]]
        XCTAssertNotNil(centers)
        XCTAssertEqual(centers?[0]["x"], 0.5)
        XCTAssertEqual(centers?[0]["y"], 0.5)
    }

    func testCaptionDictWithDefaultCenter() {
        let config = CharacterConfig(prompt: "test character")
        let dict = characterToCaptionDict(config)

        let centers = dict["centers"] as? [[String: Double]]
        XCTAssertNotNil(centers)
        XCTAssertEqual(centers?[0]["x"], 0.5)
        XCTAssertEqual(centers?[0]["y"], 0.5)
    }
}

// MARK: - 15. Builder Pattern

final class BuilderPatternTests: XCTestCase {

    func testBuilderCreatesValidParams() throws {
        let params = try GenerateParams.builder(prompt: "a beautiful landscape")
            .model(.naiDiffusion4Full)
            .width(1024)
            .height(1024)
            .steps(28)
            .scale(7.0)
            .sampler(.kEuler)
            .noiseSchedule(.exponential)
            .seed(42)
            .build()

        XCTAssertEqual(params.prompt, "a beautiful landscape")
        XCTAssertEqual(params.model, .naiDiffusion4Full)
        XCTAssertEqual(params.width, 1024)
        XCTAssertEqual(params.height, 1024)
        XCTAssertEqual(params.steps, 28)
        XCTAssertEqual(params.scale, 7.0)
        XCTAssertEqual(params.sampler, .kEuler)
        XCTAssertEqual(params.noiseSchedule, .exponential)
        XCTAssertEqual(params.seed, 42)
    }

    func testBuilderWithInvalidParamsThrowsOnBuild() {
        // Invalid width (not multiple of 64)
        XCTAssertThrowsError(
            try GenerateParams.builder(prompt: "test")
                .width(100)
                .build()
        ) { error in
            guard case NovelAIError.validation = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
        }

        // Invalid steps
        XCTAssertThrowsError(
            try GenerateParams.builder(prompt: "test")
                .steps(0)
                .build()
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
        }

        // Invalid scale
        XCTAssertThrowsError(
            try GenerateParams.builder(prompt: "test")
                .scale(11.0)
                .build()
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
        }
    }

    func testBuilderMethodChainingReturnsSelf() throws {
        // Verify that all builder methods can be chained fluently
        let params = try GenerateParams.builder(prompt: "chaining test")
            .action(.generate)
            .model(.naiDiffusion45Curated)
            .width(768)
            .height(768)
            .steps(20)
            .scale(5.0)
            .cfgRescale(0.5)
            .sampler(.kDpmpp2mSde)
            .noiseSchedule(.polyexponential)
            .negativePrompt("low quality")
            .savePath("/output/test.png")
            .seed(12345)
            .build()

        XCTAssertEqual(params.prompt, "chaining test")
        XCTAssertEqual(params.action, .generate)
        XCTAssertEqual(params.model, .naiDiffusion45Curated)
        XCTAssertEqual(params.width, 768)
        XCTAssertEqual(params.height, 768)
        XCTAssertEqual(params.steps, 20)
        XCTAssertEqual(params.scale, 5.0)
        XCTAssertEqual(params.cfgRescale, 0.5)
        XCTAssertEqual(params.sampler, .kDpmpp2mSde)
        XCTAssertEqual(params.noiseSchedule, .polyexponential)
        XCTAssertEqual(params.negativePrompt, "low quality")
        XCTAssertEqual(params.savePath, "/output/test.png")
        XCTAssertEqual(params.seed, 12345)
    }

    func testBuilderImg2imgChaining() throws {
        let params = try GenerateParams.builder(prompt: "enhance this")
            .action(.img2img)
            .sourceImage(.base64("aW1hZ2U="))
            .img2imgStrength(0.7)
            .img2imgNoise(0.1)
            .build()

        XCTAssertEqual(params.action, .img2img)
        XCTAssertEqual(params.img2imgStrength, 0.7)
        XCTAssertEqual(params.img2imgNoise, 0.1)
    }

    func testBuilderInfillChaining() throws {
        let params = try GenerateParams.builder(prompt: "inpaint this")
            .action(.infill)
            .sourceImage(.base64("aW1hZ2U="))
            .mask(.base64("bWFzaw=="))
            .maskStrength(0.5)
            .inpaintColorCorrect(false)
            .build()

        XCTAssertEqual(params.action, .infill)
        XCTAssertNotNil(params.mask)
        XCTAssertEqual(params.maskStrength, 0.5)
        XCTAssertFalse(params.inpaintColorCorrect)
    }

    func testBuilderVibeChaining() throws {
        let vibeItems: [VibeItem] = [.filePath("/path/to/vibe.png")]
        let params = try GenerateParams.builder(prompt: "vibe transfer")
            .vibes(vibeItems)
            .vibeStrengths([0.8])
            .vibeInfoExtracted([0.6])
            .build()

        XCTAssertEqual(params.vibes?.count, 1)
        XCTAssertEqual(params.vibeStrengths, [0.8])
        XCTAssertEqual(params.vibeInfoExtracted, [0.6])
    }

    func testBuilderCharacterReferenceChaining() throws {
        let charRef = CharacterReferenceConfig(
            image: .base64("aW1hZ2U="),
            strength: 0.8,
            fidelity: 0.9,
            mode: .style
        )
        let params = try GenerateParams.builder(prompt: "character ref test")
            .characterReference(charRef)
            .build()

        XCTAssertNotNil(params.characterReference)
        XCTAssertEqual(params.characterReference?.strength, 0.8)
        XCTAssertEqual(params.characterReference?.fidelity, 0.9)
        XCTAssertEqual(params.characterReference?.mode, .style)
    }

    func testBuilderCharactersChaining() throws {
        let characters = [
            CharacterConfig(prompt: "girl", centerX: 0.3, centerY: 0.5),
            CharacterConfig(prompt: "boy", centerX: 0.7, centerY: 0.5),
        ]
        let params = try GenerateParams.builder(prompt: "two characters")
            .characters(characters)
            .build()

        XCTAssertEqual(params.characters?.count, 2)
        XCTAssertEqual(params.characters?[0].prompt, "girl")
        XCTAssertEqual(params.characters?[1].prompt, "boy")
    }

    func testBuilderHybridModeChaining() throws {
        let params = try GenerateParams.builder(prompt: "hybrid test")
            .action(.img2img)
            .sourceImage(.base64("aW1hZ2U="))
            .hybridImg2imgStrength(0.5)
            .hybridImg2imgNoise(0.3)
            .build()

        XCTAssertEqual(params.hybridImg2imgStrength, 0.5)
        XCTAssertEqual(params.hybridImg2imgNoise, 0.3)
    }

    func testBuilderSaveDirChaining() throws {
        let params = try GenerateParams.builder(prompt: "save dir test")
            .saveDir("/output/images/")
            .build()

        XCTAssertEqual(params.saveDir, "/output/images/")
        XCTAssertNil(params.savePath)
    }

    func testBuilderOverwritesPreviousValues() throws {
        let params = try GenerateParams.builder(prompt: "test")
            .width(512)
            .width(768)  // overwrite
            .height(512)
            .height(1024)  // overwrite
            .build()

        XCTAssertEqual(params.width, 768)
        XCTAssertEqual(params.height, 1024)
    }

    func testStaticBuilderFactory() {
        let builder = GenerateParams.builder(prompt: "factory test")
        XCTAssertTrue(type(of: builder) == GenerateParamsBuilder.self)
    }
}

// MARK: - Additional Edge Case Tests

final class ImageInputTests: XCTestCase {

    func testFilePathInput() {
        let input = ImageInput.filePath("/path/to/image.png")
        if case .filePath(let path) = input {
            XCTAssertEqual(path, "/path/to/image.png")
        } else {
            XCTFail("Expected filePath case")
        }
    }

    func testBase64Input() {
        let input = ImageInput.base64("aW1hZ2U=")
        if case .base64(let str) = input {
            XCTAssertEqual(str, "aW1hZ2U=")
        } else {
            XCTFail("Expected base64 case")
        }
    }

    func testDataURLInput() {
        let input = ImageInput.dataURL("data:image/png;base64,aW1hZ2U=")
        if case .dataURL(let url) = input {
            XCTAssertEqual(url, "data:image/png;base64,aW1hZ2U=")
        } else {
            XCTFail("Expected dataURL case")
        }
    }

    func testBytesInput() {
        let data = Data([0x89, 0x50, 0x4E, 0x47])
        let input = ImageInput.bytes(data)
        if case .bytes(let d) = input {
            XCTAssertEqual(d, data)
        } else {
            XCTFail("Expected bytes case")
        }
    }
}

final class CharacterReferenceConfigTests: XCTestCase {

    func testDefaultValues() {
        let config = CharacterReferenceConfig(image: .base64("aW1hZ2U="))
        XCTAssertEqual(config.strength, 0.6)
        XCTAssertEqual(config.fidelity, 1.0)
        XCTAssertEqual(config.mode, .characterAndStyle)
    }

    func testValidConfigAccepted() {
        let config = CharacterReferenceConfig(
            image: .base64("aW1hZ2U="),
            strength: 0.5,
            fidelity: 0.8,
            mode: .character
        )
        XCTAssertNoThrow(try config.validate())
    }

    func testStrengthOutOfRangeRejected() {
        let config = CharacterReferenceConfig(
            image: .base64("aW1hZ2U="),
            strength: 1.5
        )
        XCTAssertThrowsError(try config.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("strength"))
        }
    }

    func testFidelityOutOfRangeRejected() {
        let config = CharacterReferenceConfig(
            image: .base64("aW1hZ2U="),
            fidelity: -0.1
        )
        XCTAssertThrowsError(try config.validate()) { error in
            guard case NovelAIError.range(let msg) = error else {
                XCTFail("Expected range error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("fidelity"))
        }
    }

    func testEmptyImageRejected() {
        let config = CharacterReferenceConfig(image: .base64(""))
        XCTAssertThrowsError(try config.validate()) { error in
            guard case NovelAIError.validation(let msg) = error else {
                XCTFail("Expected validation error, got \(error)")
                return
            }
            XCTAssertTrue(msg.contains("empty"))
        }
    }

    func testAllModesAccepted() {
        for mode in CharRefMode.allCases {
            let config = CharacterReferenceConfig(
                image: .base64("aW1hZ2U="),
                mode: mode
            )
            XCTAssertNoThrow(try config.validate(), "Mode \(mode.rawValue) should be accepted")
        }
    }
}

final class NovelAIErrorTests: XCTestCase {

    func testValidationErrorDescription() {
        let error = NovelAIError.validation("test message")
        XCTAssertTrue(error.localizedDescription.contains("Validation error"))
        XCTAssertTrue(error.localizedDescription.contains("test message"))
    }

    func testRangeErrorDescription() {
        let error = NovelAIError.range("out of range")
        XCTAssertTrue(error.localizedDescription.contains("Range error"))
        XCTAssertTrue(error.localizedDescription.contains("out of range"))
    }

    func testErrorConformsToError() {
        let error: Error = NovelAIError.validation("test")
        XCTAssertNotNil(error)
    }

    func testErrorConformsToLocalizedError() {
        let error: LocalizedError = NovelAIError.validation("test")
        XCTAssertNotNil(error.errorDescription)
    }
}
