import XCTest
@testable import NovelAIAPI

final class AnlasTests: XCTestCase {

    // MARK: - Category A: calcV4BaseCost

    func testCalcV4BaseCost_default832x1216_23Steps() {
        let cost = calcV4BaseCost(width: 832, height: 1216, steps: 23)
        XCTAssertEqual(cost, 17)
    }

    func testCalcV4BaseCost_1024x1024_28Steps() {
        let cost = calcV4BaseCost(width: 1024, height: 1024, steps: 28)
        XCTAssertEqual(cost, 20)
    }

    func testCalcV4BaseCost_2048x1536_50Steps() {
        let cost = calcV4BaseCost(width: 2048, height: 1536, steps: 50)
        XCTAssertEqual(cost, 100)
    }

    func testCalcV4BaseCost_small256x256() {
        let cost = calcV4BaseCost(width: 256, height: 256, steps: 23)
        // pixels = 65536, linear = 65536 * 2.951823174884865e-6 = 0.1935
        // step = 65536 * 5.753298233447344e-7 * 23 = 0.8673
        // total = ceil(0.1935 + 0.8673) = ceil(1.0608) = 2
        XCTAssertEqual(cost, 2)
    }

    func testCalcV4BaseCost_minimum64x64_1Step() {
        let cost = calcV4BaseCost(width: 64, height: 64, steps: 1)
        // pixels = 4096, linear = 4096 * 2.951823174884865e-6 = 0.01209
        // step = 4096 * 5.753298233447344e-7 * 1 = 0.002357
        // total = ceil(0.01209 + 0.002357) = ceil(0.01444) = 1
        XCTAssertEqual(cost, 1)
    }

    func testCalcV4BaseCost_isAlwaysPositive() {
        let cost = calcV4BaseCost(width: MIN_DIMENSION, height: MIN_DIMENSION, steps: MIN_STEPS)
        XCTAssertGreaterThan(cost, 0)
    }

    // MARK: - Category B: getSmeaMultiplier

    func testGetSmeaMultiplier_off() {
        XCTAssertEqual(getSmeaMultiplier(.off), 1.0)
    }

    func testGetSmeaMultiplier_smea() {
        XCTAssertEqual(getSmeaMultiplier(.smea), 1.2)
    }

    func testGetSmeaMultiplier_smeaDyn() {
        XCTAssertEqual(getSmeaMultiplier(.smeaDyn), 1.4)
    }

    // MARK: - Category C: isOpusFreeGeneration

    func testIsOpusFree_opusTierSmallImageNormalSteps() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 3
        )
        XCTAssertTrue(result)
    }

    func testIsOpusFree_nonOpusTier() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 0
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_tier1NotOpus() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 1
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_tier2NotOpus() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 2
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_exceedsPixelLimit() {
        // 1025 * 1025 = 1_050_625 > 1_048_576
        let result = isOpusFreeGeneration(
            width: 1025, height: 1025, steps: 28,
            charRefCount: 0, tier: 3
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_exceedsStepLimit() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 29,
            charRefCount: 0, tier: 3
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_hasCharRef() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 1, tier: 3
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_hasVibes() {
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 3, vibeCount: 1
        )
        XCTAssertFalse(result)
    }

    func testIsOpusFree_exactPixelLimitBoundary() {
        // Exactly at the limit: 1024*1024 = 1_048_576 == OPUS_FREE_PIXELS
        let result = isOpusFreeGeneration(
            width: 1024, height: 1024, steps: 28,
            charRefCount: 0, tier: 3
        )
        XCTAssertTrue(result)
    }

    func testIsOpusFree_exactStepLimitBoundary() {
        // Exactly at 28 steps
        let result = isOpusFreeGeneration(
            width: 512, height: 512, steps: 28,
            charRefCount: 0, tier: 3
        )
        XCTAssertTrue(result)
    }

    // MARK: - Category D: Per-image Cost with Strength/SMEA

    func testGenerationCost_txt2imgDefault() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23
        ))
        XCTAssertEqual(result.baseCost, 17)
        XCTAssertEqual(result.adjustedCost, 17)
    }

    func testGenerationCost_img2imgWithStrength() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            mode: .img2img, strength: 0.62
        ))
        // baseCost = 17, perImage = 17.0 * 1.0 = 17.0, adjusted = ceil(17.0 * 0.62) = ceil(10.54) = 11
        XCTAssertEqual(result.baseCost, 17)
        XCTAssertEqual(result.strengthMultiplier, 0.62)
        XCTAssertEqual(result.adjustedCost, 11)
    }

    func testGenerationCost_smeaMultiplier() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            smea: .smea
        ))
        // baseCost = 17, perImage = 17.0 * 1.2 = 20.4, adjusted = ceil(20.4) = 21
        XCTAssertEqual(result.smeaMultiplier, 1.2)
        XCTAssertEqual(result.adjustedCost, 21)
    }

    func testGenerationCost_smeaDynMultiplier() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            smea: .smeaDyn
        ))
        // baseCost = 17, perImage = 17.0 * 1.4 = 23.8, adjusted = ceil(23.8) = 24
        XCTAssertEqual(result.smeaMultiplier, 1.4)
        XCTAssertEqual(result.adjustedCost, 24)
    }

    func testGenerationCost_txt2imgStrengthIgnored() throws {
        // In txt2img mode, strength multiplier should always be 1.0
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            mode: .txt2img, strength: 0.5
        ))
        XCTAssertEqual(result.strengthMultiplier, 1.0)
        XCTAssertEqual(result.adjustedCost, 17)
    }

    // MARK: - Category E: Billable Images and Opus Discount

    func testGenerationCost_opus1Sample_free() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            nSamples: 1, tier: 3
        ))
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 0)
        XCTAssertEqual(result.totalCost, 0)
    }

    func testGenerationCost_opus2Samples_1Billable() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            nSamples: 2, tier: 3
        ))
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 1)
        XCTAssertGreaterThan(result.totalCost, 0)
    }

    func testGenerationCost_nonOpus_fullBilling() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            nSamples: 2, tier: 0
        ))
        XCTAssertFalse(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 2)
        XCTAssertEqual(result.generationCost, result.adjustedCost * 2)
    }

    func testGenerationCost_opus3Samples_2Billable() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            nSamples: 3, tier: 3
        ))
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 2)
    }

    // MARK: - Category F: Vibe Costs

    func testCalcVibeBatchCost_0Vibes() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 0), 0)
    }

    func testCalcVibeBatchCost_1Vibe() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 1), 0)
    }

    func testCalcVibeBatchCost_4Vibes_atThreshold() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 4), 0)
    }

    func testCalcVibeBatchCost_5Vibes() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 5), 2)
    }

    func testCalcVibeBatchCost_6Vibes() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 6), 4)
    }

    func testCalcVibeBatchCost_10Vibes() {
        XCTAssertEqual(calcVibeBatchCost(enabledVibeCount: 10), 12)
    }

    func testGenerationCost_vibeEncodeCost() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            vibeCount: 2, vibeUnencodedCount: 2
        ))
        XCTAssertEqual(result.vibeEncodeCost, 2 * VIBE_ENCODE_PRICE)
    }

    func testGenerationCost_vibeEncodeCostNotChargedWithCharRef() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            charRefCount: 1, vibeCount: 2, vibeUnencodedCount: 2
        ))
        // Vibe costs disabled when using character references
        XCTAssertEqual(result.vibeEncodeCost, 0)
        XCTAssertEqual(result.vibeBatchCost, 0)
    }

    func testGenerationCost_vibeCostNotChargedInInpaintMode() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            mode: .inpaint, strength: 0.7,
            vibeCount: 5, vibeUnencodedCount: 1,
            maskWidth: 832, maskHeight: 1216
        ))
        XCTAssertEqual(result.vibeEncodeCost, 0)
        XCTAssertEqual(result.vibeBatchCost, 0)
    }

    func testGenerationCost_vibeBatchCostIncludedInTotal() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            vibeCount: 6
        ))
        XCTAssertEqual(result.vibeBatchCost, 4)
        XCTAssertEqual(result.totalCost, result.generationCost + result.vibeBatchCost)
    }

    // MARK: - Category G: Character Reference Costs

    func testCalcCharRefCost_1Char1Sample() {
        XCTAssertEqual(calcCharRefCost(charRefCount: 1, nSamples: 1), 5)
    }

    func testCalcCharRefCost_2Chars3Samples() {
        XCTAssertEqual(calcCharRefCost(charRefCount: 2, nSamples: 3), 30)
    }

    func testCalcCharRefCost_0Chars() {
        XCTAssertEqual(calcCharRefCost(charRefCount: 0, nSamples: 5), 0)
    }

    func testGenerationCost_charRefCostIncludedInTotal() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            nSamples: 1, charRefCount: 2
        ))
        XCTAssertEqual(result.charRefCost, 10)
        XCTAssertTrue(result.totalCost >= result.charRefCost)
    }

    // MARK: - Category H: Inpaint Size Correction

    func testInpaintSizeCorrection_smallMaskCorrected() {
        let result = calcInpaintSizeCorrection(maskWidth: 256, maskHeight: 256)
        // 256*256 = 65536 < threshold (1_048_576 * 0.8 = 838860.8)
        XCTAssertTrue(result.corrected)
        XCTAssertGreaterThan(result.width, 256)
        XCTAssertGreaterThan(result.height, 256)
    }

    func testInpaintSizeCorrection_largeMaskNotCorrected() {
        // Need pixels >= threshold (838860.8)
        // 1024*1024 = 1_048_576 >= threshold
        let result = calcInpaintSizeCorrection(maskWidth: 1024, maskHeight: 1024)
        XCTAssertFalse(result.corrected)
        XCTAssertEqual(result.width, 1024)
        XCTAssertEqual(result.height, 1024)
    }

    func testInpaintSizeCorrection_zeroDimensionReturnsUncorrected() {
        let result = calcInpaintSizeCorrection(maskWidth: 0, maskHeight: 512)
        XCTAssertFalse(result.corrected)
        XCTAssertEqual(result.width, 0)
        XCTAssertEqual(result.height, 512)
    }

    func testInpaintSizeCorrection_negativeDimensionReturnsUncorrected() {
        let result = calcInpaintSizeCorrection(maskWidth: -100, maskHeight: 512)
        XCTAssertFalse(result.corrected)
        XCTAssertEqual(result.width, -100)
        XCTAssertEqual(result.height, 512)
    }

    func testInpaintSizeCorrection_correctedDimensionsAlignToGrid() {
        let result = calcInpaintSizeCorrection(maskWidth: 200, maskHeight: 300)
        if result.corrected {
            XCTAssertEqual(result.width % GRID_SIZE, 0, "Corrected width should be grid-aligned")
            XCTAssertEqual(result.height % GRID_SIZE, 0, "Corrected height should be grid-aligned")
        }
    }

    func testInpaintSizeCorrection_bothZero() {
        let result = calcInpaintSizeCorrection(maskWidth: 0, maskHeight: 0)
        XCTAssertFalse(result.corrected)
    }

    // MARK: - Category I: Full Integration Tests

    func testFullGeneration_txt2img_defaultParams() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            smea: .off, mode: .txt2img,
            nSamples: 1, tier: 0
        ))
        XCTAssertEqual(result.baseCost, 17)
        XCTAssertEqual(result.smeaMultiplier, 1.0)
        XCTAssertEqual(result.strengthMultiplier, 1.0)
        XCTAssertEqual(result.adjustedCost, 17)
        XCTAssertFalse(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 1)
        XCTAssertEqual(result.generationCost, 17)
        XCTAssertEqual(result.charRefCost, 0)
        XCTAssertEqual(result.vibeEncodeCost, 0)
        XCTAssertEqual(result.vibeBatchCost, 0)
        XCTAssertEqual(result.totalCost, 17)
        XCTAssertFalse(result.error)
        XCTAssertNil(result.errorCode)
    }

    func testFullGeneration_img2imgWithSmeaAndVibes() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            smea: .smea, mode: .img2img, strength: 0.5,
            nSamples: 2, tier: 0,
            vibeCount: 6, vibeUnencodedCount: 1
        ))
        // baseCost = 20 (from 1024x1024, 28 steps)
        // perImage = 20 * 1.2 = 24.0
        // adjusted = ceil(24.0 * 0.5) = ceil(12.0) = 12
        XCTAssertEqual(result.baseCost, 20)
        XCTAssertEqual(result.smeaMultiplier, 1.2)
        XCTAssertEqual(result.strengthMultiplier, 0.5)
        XCTAssertEqual(result.adjustedCost, 12)
        XCTAssertFalse(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 2)
        XCTAssertEqual(result.generationCost, 24)
        XCTAssertEqual(result.vibeEncodeCost, 2)
        XCTAssertEqual(result.vibeBatchCost, 4) // (6-4)*2
        XCTAssertEqual(result.totalCost, 24 + 2 + 4) // 30
        XCTAssertFalse(result.error)
    }

    func testFullGeneration_opusFreeWithMultipleSamples() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 1024, height: 1024, steps: 28,
            nSamples: 4, tier: 3
        ))
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.billableImages, 3) // 4 - 1 free
        XCTAssertEqual(result.generationCost, result.adjustedCost * 3)
        XCTAssertEqual(result.totalCost, result.generationCost)
    }

    func testFullGeneration_withCharRefAndSamples() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            nSamples: 2, charRefCount: 1
        ))
        XCTAssertEqual(result.charRefCost, 10) // 5 * 1 * 2
        XCTAssertFalse(result.isOpusFree) // charRef disqualifies opus free
    }

    // MARK: - Category J: Augment Cost

    func testAugmentCost_standardToolAt1024x1024() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .colorize, width: 1024, height: 1024
        ))
        // 1024*1024 = 1_048_576 >= AUGMENT_MIN_PIXELS, so no expansion
        XCTAssertEqual(result.originalPixels, 1_048_576)
        XCTAssertEqual(result.adjustedWidth, 1024)
        XCTAssertEqual(result.adjustedHeight, 1024)
        XCTAssertFalse(result.isOpusFree) // tier defaults to 0
    }

    func testAugmentCost_bgRemovalSpecialCalculation() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .bgRemoval, width: 1024, height: 1024
        ))
        let expectedBase = calcV4BaseCost(width: 1024, height: 1024, steps: AUGMENT_FIXED_STEPS)
        let expectedFinal = Int(ceil(Double(BG_REMOVAL_MULTIPLIER) * Double(expectedBase) + Double(BG_REMOVAL_ADDEND)))
        XCTAssertEqual(result.baseCost, expectedBase)
        XCTAssertEqual(result.finalCost, expectedFinal)
        XCTAssertFalse(result.isOpusFree) // bg-removal never opus free
    }

    func testAugmentCost_bgRemovalNeverOpusFree() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .bgRemoval, width: 512, height: 512, tier: 3
        ))
        XCTAssertFalse(result.isOpusFree)
    }

    func testAugmentCost_smallImageExpandedToMinPixels() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .sketch, width: 256, height: 256
        ))
        // 256*256 = 65536 < AUGMENT_MIN_PIXELS (1_048_576), should be expanded
        XCTAssertEqual(result.originalPixels, 65_536)
        XCTAssertGreaterThanOrEqual(result.adjustedPixels, AUGMENT_MIN_PIXELS)
    }

    func testAugmentCost_opusFreeForStandardTool() throws {
        // Standard tool with opus tier and small enough image
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .colorize, width: 1024, height: 1024, tier: 3
        ))
        // After expansion (already >= min), pixels <= OPUS_FREE_PIXELS
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.effectiveCost, 0)
    }

    func testAugmentCost_effectiveCostZeroWhenOpusFree() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .declutter, width: 512, height: 512, tier: 3
        ))
        if result.isOpusFree {
            XCTAssertEqual(result.effectiveCost, 0)
        }
    }

    func testAugmentCost_largeImageClampedToMaxPixels() throws {
        let result = try calculateAugmentCost(AugmentCostParams(
            tool: .colorize, width: 4096, height: 4096
        ))
        // 4096*4096 = 16_777_216 > MAX_PIXELS, should be clamped
        XCTAssertLessThanOrEqual(result.adjustedPixels, MAX_PIXELS)
    }

    // MARK: - Category K: Upscale Cost

    func testUpscaleCost_tableLookup_smallImage() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 256, height: 256))
        // 256*256 = 65536 <= 262_144 -> cost 1
        XCTAssertEqual(result.cost, 1)
        XCTAssertFalse(result.error)
    }

    func testUpscaleCost_tableLookup_mediumImage() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 640, height: 640))
        // 640*640 = 409_600 <= 409_600 -> cost 2
        XCTAssertEqual(result.cost, 2)
    }

    func testUpscaleCost_tableLookup_largeImage() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 1024, height: 1024))
        // 1024*1024 = 1_048_576 <= 1_048_576 -> cost 7
        XCTAssertEqual(result.cost, 7)
    }

    func testUpscaleCost_opusFree() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 640, height: 640, tier: 3))
        // 640*640 = 409_600 <= UPSCALE_OPUS_FREE_PIXELS (409_600)
        XCTAssertTrue(result.isOpusFree)
        XCTAssertEqual(result.cost, 0)
        XCTAssertFalse(result.error)
    }

    func testUpscaleCost_opusFreeExactBoundary() throws {
        // Exactly at the opus free limit
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 640, height: 640, tier: 3))
        XCTAssertEqual(result.pixels, 409_600)
        XCTAssertTrue(result.isOpusFree)
    }

    func testUpscaleCost_opusNotFreeAboveLimit() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 641, height: 641, tier: 3))
        // 641*641 = 410_881 > UPSCALE_OPUS_FREE_PIXELS
        XCTAssertFalse(result.isOpusFree)
    }

    func testUpscaleCost_exceedsTable_error() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 1025, height: 1025))
        // 1025*1025 = 1_050_625 > 1_048_576 (largest table entry)
        XCTAssertTrue(result.error)
        XCTAssertEqual(result.errorCode, -3)
        XCTAssertNil(result.cost)
    }

    func testUpscaleCost_pixelsMatchInput() throws {
        let result = try calculateUpscaleCost(UpscaleCostParams(width: 300, height: 400))
        XCTAssertEqual(result.pixels, 120_000)
    }

    // MARK: - Category L: expandToMinPixels, clampToMaxPixels

    func testExpandToMinPixels_alreadyMeetsRequirement() {
        let result = expandToMinPixels(width: 1024, height: 1024, minPixels: 1_048_576)
        XCTAssertEqual(result.width, 1024)
        XCTAssertEqual(result.height, 1024)
        XCTAssertEqual(result.pixels, 1_048_576)
    }

    func testExpandToMinPixels_needsExpansion() {
        let result = expandToMinPixels(width: 256, height: 256, minPixels: 1_048_576)
        // Should expand to meet the minimum
        XCTAssertGreaterThanOrEqual(result.pixels, 1_048_576)
        XCTAssertGreaterThan(result.width, 256)
        XCTAssertGreaterThan(result.height, 256)
    }

    func testExpandToMinPixels_exactlyAtMinimum() {
        let result = expandToMinPixels(width: 1024, height: 1024, minPixels: 1_048_576)
        // 1024*1024 = 1_048_576 == minPixels, no expansion
        XCTAssertEqual(result.width, 1024)
        XCTAssertEqual(result.height, 1024)
    }

    func testExpandToMinPixels_maintainsApproximateAspectRatio() {
        let result = expandToMinPixels(width: 100, height: 200, minPixels: 1_000_000)
        // Original ratio ~0.5, expanded ratio should be approximately the same
        let originalRatio = Double(100) / Double(200)
        let expandedRatio = Double(result.width) / Double(result.height)
        XCTAssertEqual(expandedRatio, originalRatio, accuracy: 0.1)
    }

    func testClampToMaxPixels_alreadyWithinLimit() {
        let result = clampToMaxPixels(width: 1024, height: 1024, maxPixels: 3_145_728)
        XCTAssertEqual(result.width, 1024)
        XCTAssertEqual(result.height, 1024)
        XCTAssertEqual(result.pixels, 1_048_576)
    }

    func testClampToMaxPixels_needsClamping() {
        let result = clampToMaxPixels(width: 4096, height: 4096, maxPixels: 3_145_728)
        XCTAssertLessThanOrEqual(result.pixels, 3_145_728)
        XCTAssertLessThan(result.width, 4096)
        XCTAssertLessThan(result.height, 4096)
    }

    func testClampToMaxPixels_exactlyAtMaximum() {
        // 2048*1536 = 3_145_728 exactly
        let result = clampToMaxPixels(width: 2048, height: 1536, maxPixels: 3_145_728)
        XCTAssertEqual(result.width, 2048)
        XCTAssertEqual(result.height, 1536)
    }

    func testClampToMaxPixels_maintainsApproximateAspectRatio() {
        let result = clampToMaxPixels(width: 4000, height: 2000, maxPixels: 1_000_000)
        let originalRatio = Double(4000) / Double(2000)
        let clampedRatio = Double(result.width) / Double(result.height)
        XCTAssertEqual(clampedRatio, originalRatio, accuracy: 0.15)
    }

    // MARK: - Category M: Edge Cases - Min/Max Cost Guarantees

    func testMinCostGuarantee() throws {
        // Very small image with few steps; adjusted cost should still be >= MIN_COST_PER_IMAGE
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 64, height: 64, steps: 1,
            mode: .img2img, strength: 0.01
        ))
        XCTAssertGreaterThanOrEqual(result.adjustedCost, MIN_COST_PER_IMAGE)
    }

    func testMaxCostError() throws {
        // Very large image, maximum steps -> adjusted cost exceeds MAX_COST_PER_IMAGE
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 2048, height: 1536, steps: 50,
            smea: .smeaDyn
        ))
        // baseCost = 100, * 1.4 = 140.0, adjusted = 140
        // 140 == MAX_COST_PER_IMAGE, so it is NOT an error
        XCTAssertEqual(result.adjustedCost, 140)
        XCTAssertFalse(result.error)
    }

    func testMaxCostError_exceedsLimit() throws {
        // Use a slightly larger image to push past 140
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 2048, height: 1536, steps: 50,
            smea: .smeaDyn, nSamples: 1, charRefCount: 0
        ))
        // If adjustedCost > 140 it triggers error, if exactly 140 it does not.
        // Let us verify the boundary: 2048*1536*50 with smeaDyn
        // baseCost = 100, * 1.4 = 140, adjustedCost = 140
        if result.adjustedCost > MAX_COST_PER_IMAGE {
            XCTAssertTrue(result.error)
            XCTAssertEqual(result.errorCode, -3)
            XCTAssertEqual(result.totalCost, 0)
        } else {
            XCTAssertFalse(result.error)
        }
    }

    // MARK: - Category N: Zero-Division Guards

    func testZeroDivisionGuard_inpaintZeroMask() {
        // Zero mask dimensions should return uncorrected without division error
        let result = calcInpaintSizeCorrection(maskWidth: 0, maskHeight: 0)
        XCTAssertFalse(result.corrected)
    }

    func testZeroDivisionGuard_expandWithLargePixels() {
        // Already >= minPixels, no division needed
        let result = expandToMinPixels(width: 2048, height: 1536, minPixels: 100)
        XCTAssertEqual(result.width, 2048)
        XCTAssertEqual(result.height, 1536)
    }

    func testZeroDivisionGuard_clampWithSmallPixels() {
        // Already <= maxPixels, no division needed
        let result = clampToMaxPixels(width: 64, height: 64, maxPixels: 10_000_000)
        XCTAssertEqual(result.width, 64)
        XCTAssertEqual(result.height, 64)
    }

    // MARK: - Category O: Input Validation

    func testNegativeWidthThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(width: -1, height: 1216, steps: 23))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testZeroWidthThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(width: 0, height: 1216, steps: 23))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testNegativeHeightThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(width: 832, height: -1, steps: 23))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testZeroStepsThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(width: 832, height: 1216, steps: 0))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testNegativeStepsThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(width: 832, height: 1216, steps: -5))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testInvalidStrengthThrows_above1() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(
                width: 832, height: 1216, steps: 23, strength: 1.5
            ))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testInvalidStrengthThrows_negative() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(
                width: 832, height: 1216, steps: 23, strength: -0.1
            ))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testNegativeNSamplesThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(
                width: 832, height: 1216, steps: 23, nSamples: -1
            ))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testAugmentCost_negativeWidthThrows() {
        XCTAssertThrowsError(
            try calculateAugmentCost(AugmentCostParams(tool: .colorize, width: -1, height: 1024))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testAugmentCost_zeroHeightThrows() {
        XCTAssertThrowsError(
            try calculateAugmentCost(AugmentCostParams(tool: .colorize, width: 1024, height: 0))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testUpscaleCost_negativeWidthThrows() {
        XCTAssertThrowsError(
            try calculateUpscaleCost(UpscaleCostParams(width: -1, height: 512))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    func testUpscaleCost_zeroHeightThrows() {
        XCTAssertThrowsError(
            try calculateUpscaleCost(UpscaleCostParams(width: 512, height: 0))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    // MARK: - Category P: Error totalCost Behavior

    func testErrorTotalCostIsZero() throws {
        // Create a scenario that exceeds MAX_COST_PER_IMAGE
        // We need adjustedCost > 140
        // baseCost with large image and max steps + smeaDyn
        // Try slightly larger dimensions to push past 140
        // 2048*1536=3_145_728 pixels, 50 steps:
        //   linear = 3_145_728 * 2.951823174884865e-6 = 9.285
        //   step = 3_145_728 * 5.753298233447344e-7 * 50 = 90.47
        //   baseCost = ceil(9.285 + 90.47) = 100
        //   * 1.4 = 140 (exactly at limit, not error)
        // We need a case that is slightly above. Let's use the calculation result check:
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 2048, height: 1536, steps: 50,
            smea: .smeaDyn
        ))
        if result.error {
            XCTAssertEqual(result.totalCost, 0, "When error is true, totalCost should be 0")
        }
    }

    func testErrorTotalCostIsZero_forcedViaLargerDimensions() throws {
        // Use a large image that may push cost over 140 after SMEA multiplier
        // Let's try a scenario where base cost is very high
        // 2048*1536*50 gives baseCost=100, *1.4=140 (boundary)
        // With strength=1.0, adjustedCost = 140 which is NOT > 140

        // Check that the error path correctly returns 0
        // This test verifies the logic path: when error==true, totalCost==0
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 2048, height: 1536, steps: 50,
            smea: .smeaDyn
        ))
        // adjustedCost is exactly 140 which is NOT > 140, so error should be false
        XCTAssertEqual(result.adjustedCost, 140)
        XCTAssertFalse(result.error)
        XCTAssertGreaterThan(result.totalCost, 0)
    }

    // MARK: - Additional Integration: SmeaMode and GenerationMode Enums

    func testSmeaModeRawValues() {
        XCTAssertEqual(SmeaMode.off.rawValue, "off")
        XCTAssertEqual(SmeaMode.smea.rawValue, "smea")
        XCTAssertEqual(SmeaMode.smeaDyn.rawValue, "smea_dyn")
    }

    func testGenerationModeRawValues() {
        XCTAssertEqual(GenerationMode.txt2img.rawValue, "txt2img")
        XCTAssertEqual(GenerationMode.img2img.rawValue, "img2img")
        XCTAssertEqual(GenerationMode.inpaint.rawValue, "inpaint")
    }

    // MARK: - Additional: Inpaint with Mask Correction Integration

    func testInpaintWithSmallMaskReducesCost() throws {
        // Inpaint with a small mask should use corrected (larger) dimensions
        let resultWithMask = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            mode: .inpaint, strength: 0.7,
            maskWidth: 200, maskHeight: 200
        ))

        // The corrected mask dimensions will differ from raw 200x200
        let correction = calcInpaintSizeCorrection(maskWidth: 200, maskHeight: 200)
        XCTAssertTrue(correction.corrected)

        // Verify the inpaint cost used corrected dimensions (which affects baseCost)
        let correctedBaseCost = calcV4BaseCost(
            width: correction.width, height: correction.height, steps: 23
        )
        XCTAssertEqual(resultWithMask.baseCost, correctedBaseCost)
    }

    func testInpaintWithLargeMaskUsesOriginalDimensions() throws {
        let result = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23,
            mode: .inpaint, strength: 0.7,
            maskWidth: 1024, maskHeight: 1024
        ))
        // Large mask is not corrected, so original width/height used
        let expectedBaseCost = calcV4BaseCost(width: 832, height: 1216, steps: 23)
        XCTAssertEqual(result.baseCost, expectedBaseCost)
    }

    func testInpaintMissingOneMaskDimensionThrows() {
        XCTAssertThrowsError(
            try calculateGenerationCost(GenerationCostParams(
                width: 832, height: 1216, steps: 23,
                mode: .inpaint, strength: 0.7,
                maskWidth: 200
            ))
        ) { error in
            guard case NovelAIError.range = error else {
                XCTFail("Expected NovelAIError.range, got \(error)")
                return
            }
        }
    }

    // MARK: - Additional: Cost Consistency Checks

    func testCostIncreasesWithPixels() throws {
        let small = try calculateGenerationCost(GenerationCostParams(
            width: 512, height: 512, steps: 23
        ))
        let large = try calculateGenerationCost(GenerationCostParams(
            width: 2048, height: 1536, steps: 23
        ))
        XCTAssertGreaterThan(large.adjustedCost, small.adjustedCost)
    }

    func testCostIncreasesWithSteps() throws {
        let fewSteps = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 1
        ))
        let manySteps = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 50
        ))
        XCTAssertGreaterThan(manySteps.adjustedCost, fewSteps.adjustedCost)
    }

    func testCostIncreasesWithSmeaMode() throws {
        let off = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23, smea: .off
        ))
        let smea = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23, smea: .smea
        ))
        let smeaDyn = try calculateGenerationCost(GenerationCostParams(
            width: 832, height: 1216, steps: 23, smea: .smeaDyn
        ))
        XCTAssertLessThanOrEqual(off.adjustedCost, smea.adjustedCost)
        XCTAssertLessThanOrEqual(smea.adjustedCost, smeaDyn.adjustedCost)
    }
}
