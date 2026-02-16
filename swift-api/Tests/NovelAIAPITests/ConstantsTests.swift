import XCTest
@testable import NovelAIAPI

final class ConstantsTests: XCTestCase {

    // MARK: - API URL Functions

    func testApiURLReturnsDefaultValue() {
        XCTAssertEqual(apiURL(), "https://image.novelai.net/ai/generate-image")
    }

    func testStreamURLReturnsDefaultValue() {
        XCTAssertEqual(streamURL(), "https://image.novelai.net/ai/generate-image-stream")
    }

    func testEncodeURLReturnsDefaultValue() {
        XCTAssertEqual(encodeURL(), "https://image.novelai.net/ai/encode-vibe")
    }

    func testSubscriptionURLReturnsDefaultValue() {
        XCTAssertEqual(subscriptionURL(), "https://api.novelai.net/user/subscription")
    }

    func testAugmentURLReturnsDefaultValue() {
        XCTAssertEqual(augmentURL(), "https://image.novelai.net/ai/augment-image")
    }

    func testUpscaleURLReturnsDefaultValue() {
        XCTAssertEqual(upscaleURL(), "https://api.novelai.net/ai/upscale")
    }

    // MARK: - Default Values

    func testDefaultNegativeContainsExpectedKeywords() {
        XCTAssertTrue(DEFAULT_NEGATIVE.contains("nsfw"))
        XCTAssertTrue(DEFAULT_NEGATIVE.contains("lowres"))
        XCTAssertTrue(DEFAULT_NEGATIVE.contains("worst quality"))
        XCTAssertTrue(DEFAULT_NEGATIVE.contains("bad quality"))
        XCTAssertTrue(DEFAULT_NEGATIVE.contains("jpeg artifacts"))
    }

    func testDefaultModel() {
        XCTAssertEqual(DEFAULT_MODEL, Model.naiDiffusion45Full)
    }

    func testDefaultWidth() {
        XCTAssertEqual(DEFAULT_WIDTH, 832)
    }

    func testDefaultHeight() {
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

    func testDefaultVibeStrength() {
        XCTAssertEqual(DEFAULT_VIBE_STRENGTH, 0.7)
    }

    func testDefaultVibeInfoExtracted() {
        XCTAssertEqual(DEFAULT_VIBE_INFO_EXTRACTED, 0.7)
    }

    func testDefaultImg2ImgStrength() {
        XCTAssertEqual(DEFAULT_IMG2IMG_STRENGTH, 0.62)
    }

    func testDefaultCfgRescale() {
        XCTAssertEqual(DEFAULT_CFG_RESCALE, 0)
    }

    func testDefaultInpaintStrength() {
        XCTAssertEqual(DEFAULT_INPAINT_STRENGTH, 0.7)
    }

    func testDefaultInpaintNoise() {
        XCTAssertEqual(DEFAULT_INPAINT_NOISE, 0)
    }

    func testDefaultInpaintColorCorrect() {
        XCTAssertTrue(DEFAULT_INPAINT_COLOR_CORRECT)
    }

    // MARK: - Model Enum

    func testModelHasFourCases() {
        XCTAssertEqual(Model.allCases.count, 4, "Model enum case count changed — update MODEL_KEY_MAP and related tests")
    }

    func testModelRawValues() {
        XCTAssertEqual(Model.naiDiffusion4CuratedPreview.rawValue, "nai-diffusion-4-curated-preview")
        XCTAssertEqual(Model.naiDiffusion4Full.rawValue, "nai-diffusion-4-full")
        XCTAssertEqual(Model.naiDiffusion45Curated.rawValue, "nai-diffusion-4-5-curated")
        XCTAssertEqual(Model.naiDiffusion45Full.rawValue, "nai-diffusion-4-5-full")
    }

    func testModelInitFromRawValue() {
        XCTAssertEqual(Model(rawValue: "nai-diffusion-4-curated-preview"), .naiDiffusion4CuratedPreview)
        XCTAssertEqual(Model(rawValue: "nai-diffusion-4-full"), .naiDiffusion4Full)
        XCTAssertEqual(Model(rawValue: "nai-diffusion-4-5-curated"), .naiDiffusion45Curated)
        XCTAssertEqual(Model(rawValue: "nai-diffusion-4-5-full"), .naiDiffusion45Full)
        XCTAssertNil(Model(rawValue: "invalid-model"))
    }

    // MARK: - Sampler Enum

    func testSamplerHasSixCases() {
        XCTAssertEqual(Sampler.allCases.count, 6, "Sampler enum case count changed — update related tests")
    }

    func testSamplerRawValues() {
        XCTAssertEqual(Sampler.kEuler.rawValue, "k_euler")
        XCTAssertEqual(Sampler.kEulerAncestral.rawValue, "k_euler_ancestral")
        XCTAssertEqual(Sampler.kDpmpp2sAncestral.rawValue, "k_dpmpp_2s_ancestral")
        XCTAssertEqual(Sampler.kDpmpp2mSde.rawValue, "k_dpmpp_2m_sde")
        XCTAssertEqual(Sampler.kDpmpp2m.rawValue, "k_dpmpp_2m")
        XCTAssertEqual(Sampler.kDpmppSde.rawValue, "k_dpmpp_sde")
    }

    // MARK: - NoiseSchedule Enum

    func testNoiseScheduleHasThreeCases() {
        XCTAssertEqual(NoiseSchedule.allCases.count, 3, "NoiseSchedule enum case count changed — update related tests")
    }

    func testNoiseScheduleRawValues() {
        XCTAssertEqual(NoiseSchedule.karras.rawValue, "karras")
        XCTAssertEqual(NoiseSchedule.exponential.rawValue, "exponential")
        XCTAssertEqual(NoiseSchedule.polyexponential.rawValue, "polyexponential")
    }

    // MARK: - AugmentReqType Enum

    func testAugmentReqTypeHasSixCases() {
        XCTAssertEqual(AugmentReqType.allCases.count, 6, "AugmentReqType enum case count changed — update validation logic and tests")
    }

    func testAugmentReqTypeRawValues() {
        XCTAssertEqual(AugmentReqType.colorize.rawValue, "colorize")
        XCTAssertEqual(AugmentReqType.declutter.rawValue, "declutter")
        XCTAssertEqual(AugmentReqType.emotion.rawValue, "emotion")
        XCTAssertEqual(AugmentReqType.sketch.rawValue, "sketch")
        XCTAssertEqual(AugmentReqType.lineart.rawValue, "lineart")
        XCTAssertEqual(AugmentReqType.bgRemoval.rawValue, "bg-removal")
    }

    // MARK: - EmotionKeyword Enum

    func testEmotionKeywordHas24Cases() {
        XCTAssertEqual(EmotionKeyword.allCases.count, 24, "EmotionKeyword enum case count changed — update emotion validation")
    }

    func testEmotionKeywordRawValues() {
        XCTAssertEqual(EmotionKeyword.neutral.rawValue, "neutral")
        XCTAssertEqual(EmotionKeyword.happy.rawValue, "happy")
        XCTAssertEqual(EmotionKeyword.sad.rawValue, "sad")
        XCTAssertEqual(EmotionKeyword.angry.rawValue, "angry")
        XCTAssertEqual(EmotionKeyword.scared.rawValue, "scared")
        XCTAssertEqual(EmotionKeyword.surprised.rawValue, "surprised")
        XCTAssertEqual(EmotionKeyword.tired.rawValue, "tired")
        XCTAssertEqual(EmotionKeyword.excited.rawValue, "excited")
        XCTAssertEqual(EmotionKeyword.nervous.rawValue, "nervous")
        XCTAssertEqual(EmotionKeyword.thinking.rawValue, "thinking")
        XCTAssertEqual(EmotionKeyword.confused.rawValue, "confused")
        XCTAssertEqual(EmotionKeyword.shy.rawValue, "shy")
        XCTAssertEqual(EmotionKeyword.disgusted.rawValue, "disgusted")
        XCTAssertEqual(EmotionKeyword.smug.rawValue, "smug")
        XCTAssertEqual(EmotionKeyword.bored.rawValue, "bored")
        XCTAssertEqual(EmotionKeyword.laughing.rawValue, "laughing")
        XCTAssertEqual(EmotionKeyword.irritated.rawValue, "irritated")
        XCTAssertEqual(EmotionKeyword.aroused.rawValue, "aroused")
        XCTAssertEqual(EmotionKeyword.embarrassed.rawValue, "embarrassed")
        XCTAssertEqual(EmotionKeyword.love.rawValue, "love")
        XCTAssertEqual(EmotionKeyword.worried.rawValue, "worried")
        XCTAssertEqual(EmotionKeyword.determined.rawValue, "determined")
        XCTAssertEqual(EmotionKeyword.hurt.rawValue, "hurt")
        XCTAssertEqual(EmotionKeyword.playful.rawValue, "playful")
    }

    // MARK: - MODEL_KEY_MAP

    func testModelKeyMapHasFourEntries() {
        XCTAssertEqual(MODEL_KEY_MAP.count, 4)
    }

    func testModelKeyMapValues() {
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion4CuratedPreview], "v4curated")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion4Full], "v4full")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion45Curated], "v4-5curated")
        XCTAssertEqual(MODEL_KEY_MAP[.naiDiffusion45Full], "v4-5full")
    }

    func testModelKeyMapCoversAllModels() {
        for model in Model.allCases {
            XCTAssertNotNil(MODEL_KEY_MAP[model], "MODEL_KEY_MAP should have entry for \(model)")
        }
    }

    // MARK: - Limit Constants

    func testMaxPixels() {
        XCTAssertEqual(MAX_PIXELS, 3_145_728)
    }

    func testMaxTokens() {
        XCTAssertEqual(MAX_TOKENS, 512)
    }

    func testMinDimension() {
        XCTAssertEqual(MIN_DIMENSION, 64)
    }

    func testMaxGenerationDimension() {
        XCTAssertEqual(MAX_GENERATION_DIMENSION, 2048)
    }

    func testMaxCharacters() {
        XCTAssertEqual(MAX_CHARACTERS, 6)
    }

    func testMaxVibes() {
        XCTAssertEqual(MAX_VIBES, 10)
    }

    func testMinSteps() {
        XCTAssertEqual(MIN_STEPS, 1)
    }

    func testMaxSteps() {
        XCTAssertEqual(MAX_STEPS, 50)
    }

    func testMinScale() {
        XCTAssertEqual(MIN_SCALE, 0.0)
    }

    func testMaxScale() {
        XCTAssertEqual(MAX_SCALE, 10.0)
    }

    func testMaxSeed() {
        XCTAssertEqual(MAX_SEED, 4_294_967_295)
    }

    func testMaxRefImageSizeMB() {
        XCTAssertEqual(MAX_REF_IMAGE_SIZE_MB, 10)
    }

    func testMaxRefImageDimension() {
        XCTAssertEqual(MAX_REF_IMAGE_DIMENSION, 4096)
    }

    // MARK: - Anlas Cost Constants

    func testOpusFreePixels() {
        XCTAssertEqual(OPUS_FREE_PIXELS, 1_048_576)
    }

    func testOpusFreeMaxSteps() {
        XCTAssertEqual(OPUS_FREE_MAX_STEPS, 28)
    }

    func testOpusMinTier() {
        XCTAssertEqual(OPUS_MIN_TIER, 3)
    }

    func testMaxCostPerImage() {
        XCTAssertEqual(MAX_COST_PER_IMAGE, 140)
    }

    func testMinCostPerImage() {
        XCTAssertEqual(MIN_COST_PER_IMAGE, 2)
    }

    func testGridSize() {
        XCTAssertEqual(GRID_SIZE, 64)
    }

    func testVibeBatchPrice() {
        XCTAssertEqual(VIBE_BATCH_PRICE, 2)
    }

    func testVibeFreeThreshold() {
        XCTAssertEqual(VIBE_FREE_THRESHOLD, 4)
    }

    func testVibeEncodePrice() {
        XCTAssertEqual(VIBE_ENCODE_PRICE, 2)
    }

    func testCharRefPrice() {
        XCTAssertEqual(CHAR_REF_PRICE, 5)
    }

    func testInpaintThresholdRatio() {
        XCTAssertEqual(INPAINT_THRESHOLD_RATIO, 0.8)
    }

    func testV4CostCoeffLinear() {
        XCTAssertEqual(V4_COST_COEFF_LINEAR, 2.951823174884865e-6, accuracy: 1e-18)
    }

    func testV4CostCoeffStep() {
        XCTAssertEqual(V4_COST_COEFF_STEP, 5.753298233447344e-7, accuracy: 1e-19)
    }

    func testAugmentFixedSteps() {
        XCTAssertEqual(AUGMENT_FIXED_STEPS, 28)
    }

    func testAugmentMinPixels() {
        XCTAssertEqual(AUGMENT_MIN_PIXELS, 1_048_576)
    }

    func testBgRemovalMultiplier() {
        XCTAssertEqual(BG_REMOVAL_MULTIPLIER, 3)
    }

    func testBgRemovalAddend() {
        XCTAssertEqual(BG_REMOVAL_ADDEND, 5)
    }

    // MARK: - Upscale Cost Table

    func testUpscaleCostTableHasFiveEntries() {
        XCTAssertEqual(UPSCALE_COST_TABLE.count, 5)
    }

    func testUpscaleCostTableValues() {
        XCTAssertEqual(UPSCALE_COST_TABLE[0].maxPixels, 262_144)
        XCTAssertEqual(UPSCALE_COST_TABLE[0].cost, 1)

        XCTAssertEqual(UPSCALE_COST_TABLE[1].maxPixels, 409_600)
        XCTAssertEqual(UPSCALE_COST_TABLE[1].cost, 2)

        XCTAssertEqual(UPSCALE_COST_TABLE[2].maxPixels, 524_288)
        XCTAssertEqual(UPSCALE_COST_TABLE[2].cost, 3)

        XCTAssertEqual(UPSCALE_COST_TABLE[3].maxPixels, 786_432)
        XCTAssertEqual(UPSCALE_COST_TABLE[3].cost, 5)

        XCTAssertEqual(UPSCALE_COST_TABLE[4].maxPixels, 1_048_576)
        XCTAssertEqual(UPSCALE_COST_TABLE[4].cost, 7)
    }

    func testUpscaleCostTableIsAscendingByMaxPixels() {
        for i in 1..<UPSCALE_COST_TABLE.count {
            XCTAssertGreaterThan(
                UPSCALE_COST_TABLE[i].maxPixels,
                UPSCALE_COST_TABLE[i - 1].maxPixels,
                "UPSCALE_COST_TABLE should be sorted ascending by maxPixels"
            )
        }
    }

    func testUpscaleOpusFreePixels() {
        XCTAssertEqual(UPSCALE_OPUS_FREE_PIXELS, 409_600)
    }

    // MARK: - Enhance Level Presets

    func testEnhanceLevelPresetsHasFiveLevels() {
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS.count, 5)
    }

    func testEnhanceLevelPresetValues() {
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[1]?.strength, 0.2)
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[1]?.noise, 0)

        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[2]?.strength, 0.4)
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[2]?.noise, 0)

        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[3]?.strength, 0.5)
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[3]?.noise, 0)

        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[4]?.strength, 0.6)
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[4]?.noise, 0)

        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[5]?.strength, 0.7)
        XCTAssertEqual(ENHANCE_LEVEL_PRESETS[5]?.noise, 0.1)
    }

    func testEnhanceLevelPresetsContainsLevels1Through5() {
        for level in 1...5 {
            XCTAssertNotNil(ENHANCE_LEVEL_PRESETS[level], "Missing enhance level preset \(level)")
        }
    }

    func testEnhanceLevelPresetsDoesNotContainLevel0() {
        XCTAssertNil(ENHANCE_LEVEL_PRESETS[0])
    }

    // MARK: - Valid Upscale Scales

    func testValidUpscaleScales() {
        XCTAssertEqual(VALID_UPSCALE_SCALES, [2, 4])
    }

    func testDefaultUpscaleScale() {
        XCTAssertEqual(DEFAULT_UPSCALE_SCALE, 4)
    }

    // MARK: - Character Reference Sizes

    func testCharRefPortraitSize() {
        XCTAssertEqual(CHARREF_PORTRAIT_SIZE.width, 1024)
        XCTAssertEqual(CHARREF_PORTRAIT_SIZE.height, 1536)
    }

    func testCharRefLandscapeSize() {
        XCTAssertEqual(CHARREF_LANDSCAPE_SIZE.width, 1536)
        XCTAssertEqual(CHARREF_LANDSCAPE_SIZE.height, 1024)
    }

    func testCharRefSquareSize() {
        XCTAssertEqual(CHARREF_SQUARE_SIZE.width, 1472)
        XCTAssertEqual(CHARREF_SQUARE_SIZE.height, 1472)
    }

    func testCharRefPortraitThreshold() {
        XCTAssertEqual(CHARREF_PORTRAIT_THRESHOLD, 0.8)
    }

    func testCharRefLandscapeThreshold() {
        XCTAssertEqual(CHARREF_LANDSCAPE_THRESHOLD, 1.25)
    }

    // MARK: - Defry Constants

    func testMinDefry() {
        XCTAssertEqual(MIN_DEFRY, 0)
    }

    func testMaxDefry() {
        XCTAssertEqual(MAX_DEFRY, 5)
    }

    func testDefaultDefry() {
        XCTAssertEqual(DEFAULT_DEFRY, 3)
    }

    // MARK: - Network & Security Constants

    func testDefaultRequestTimeoutMs() {
        XCTAssertEqual(DEFAULT_REQUEST_TIMEOUT_MS, 60_000)
    }

    func testMaxDecompressedImageSize() {
        XCTAssertEqual(MAX_DECOMPRESSED_IMAGE_SIZE, 50 * 1024 * 1024)
    }

    func testMaxResponseSize() {
        XCTAssertEqual(MAX_RESPONSE_SIZE, 200 * 1024 * 1024)
    }

    func testMaxZipEntries() {
        XCTAssertEqual(MAX_ZIP_ENTRIES, 10)
    }

    func testMaxCompressionRatio() {
        XCTAssertEqual(MAX_COMPRESSION_RATIO, 100)
    }

    func testMaxVibeEncodingLength() {
        XCTAssertEqual(MAX_VIBE_ENCODING_LENGTH, 5_000_000)
    }
}
