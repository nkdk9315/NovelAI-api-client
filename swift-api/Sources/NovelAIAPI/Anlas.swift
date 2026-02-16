import Foundation

// MARK: - Types

/// Subscription tier (0=Free, 1=Tablet, 2=Scroll, 3=Opus)
public typealias SubscriptionTier = Int

/// SMEA mode
public enum SmeaMode: String, Sendable {
    case off
    case smea
    case smeaDyn = "smea_dyn"
}

/// Generation mode
public enum GenerationMode: String, Sendable {
    case txt2img
    case img2img
    case inpaint
}

// MARK: - Cost Parameter Types

/// Parameters for generation cost calculation
public struct GenerationCostParams: Sendable {
    public var width: Int
    public var height: Int
    public var steps: Int
    public var smea: SmeaMode?
    public var mode: GenerationMode?
    public var strength: Double?
    public var nSamples: Int?
    public var tier: SubscriptionTier?
    public var charRefCount: Int?
    public var vibeCount: Int?
    public var vibeUnencodedCount: Int?
    public var maskWidth: Int?
    public var maskHeight: Int?

    public init(
        width: Int,
        height: Int,
        steps: Int,
        smea: SmeaMode? = nil,
        mode: GenerationMode? = nil,
        strength: Double? = nil,
        nSamples: Int? = nil,
        tier: SubscriptionTier? = nil,
        charRefCount: Int? = nil,
        vibeCount: Int? = nil,
        vibeUnencodedCount: Int? = nil,
        maskWidth: Int? = nil,
        maskHeight: Int? = nil
    ) {
        self.width = width
        self.height = height
        self.steps = steps
        self.smea = smea
        self.mode = mode
        self.strength = strength
        self.nSamples = nSamples
        self.tier = tier
        self.charRefCount = charRefCount
        self.vibeCount = vibeCount
        self.vibeUnencodedCount = vibeUnencodedCount
        self.maskWidth = maskWidth
        self.maskHeight = maskHeight
    }
}

/// Parameters for augment cost calculation
public struct AugmentCostParams: Sendable {
    public var tool: AugmentReqType
    public var width: Int
    public var height: Int
    public var tier: SubscriptionTier?

    public init(tool: AugmentReqType, width: Int, height: Int, tier: SubscriptionTier? = nil) {
        self.tool = tool
        self.width = width
        self.height = height
        self.tier = tier
    }
}

/// Parameters for upscale cost calculation
public struct UpscaleCostParams: Sendable {
    public var width: Int
    public var height: Int
    public var tier: SubscriptionTier?

    public init(width: Int, height: Int, tier: SubscriptionTier? = nil) {
        self.width = width
        self.height = height
        self.tier = tier
    }
}

// MARK: - Cost Result Types

/// Generation cost calculation result with breakdown
public struct GenerationCostResult: Sendable {
    public let baseCost: Int
    public let smeaMultiplier: Double
    public let perImageCost: Double
    public let strengthMultiplier: Double
    public let adjustedCost: Int
    public let isOpusFree: Bool
    public let billableImages: Int
    public let generationCost: Int
    public let charRefCost: Int
    public let vibeEncodeCost: Int
    public let vibeBatchCost: Int
    public let totalCost: Int
    public let error: Bool
    public let errorCode: Int?
}

/// Augment cost calculation result
public struct AugmentCostResult: Sendable {
    public let originalPixels: Int
    public let adjustedWidth: Int
    public let adjustedHeight: Int
    public let adjustedPixels: Int
    public let baseCost: Int
    public let finalCost: Int
    public let isOpusFree: Bool
    public let effectiveCost: Int
}

/// Upscale cost calculation result
public struct UpscaleCostResult: Sendable {
    public let pixels: Int
    public let cost: Int?
    public let isOpusFree: Bool
    public let error: Bool
    public let errorCode: Int?
}

/// Inpaint size correction result
public struct InpaintCorrectionResult: Sendable {
    public let corrected: Bool
    public let width: Int
    public let height: Int
}

// MARK: - Validation Helpers

private func assertPositiveFiniteInt(_ value: Int, _ name: String) throws {
    if value <= 0 {
        throw NovelAIError.range("\(name) must be a positive integer, got \(value)")
    }
}

private func assertFiniteRange(_ value: Double, _ min: Double, _ max: Double, _ name: String) throws {
    if value.isNaN || value.isInfinite || value < min || value > max {
        throw NovelAIError.range("\(name) must be a finite number between \(min) and \(max), got \(value)")
    }
}

private func assertNonNegativeFiniteInt(_ value: Int, _ name: String) throws {
    if value < 0 {
        throw NovelAIError.range("\(name) must be a non-negative integer, got \(value)")
    }
}

// MARK: - Basic Calculation Functions

/// Calculate V4 model base cost from pixel count and steps.
/// Note: `width * height` is computed in Int (64-bit on all supported platforms),
/// which is safe for values up to MAX_PIXELS (3,145,728).
public func calcV4BaseCost(width: Int, height: Int, steps: Int) -> Int {
    let pixels = Double(width * height)
    return Int(ceil(V4_COST_COEFF_LINEAR * pixels + V4_COST_COEFF_STEP * pixels * Double(steps)))
}

/// Get SMEA mode cost multiplier
public func getSmeaMultiplier(_ mode: SmeaMode) -> Double {
    switch mode {
    case .smeaDyn: return 1.4
    case .smea: return 1.2
    case .off: return 1.0
    }
}

/// Check if generation qualifies for Opus free tier
public func isOpusFreeGeneration(
    width: Int,
    height: Int,
    steps: Int,
    charRefCount: Int,
    tier: SubscriptionTier,
    vibeCount: Int = 0
) -> Bool {
    return charRefCount == 0
        && vibeCount == 0
        && width * height <= OPUS_FREE_PIXELS
        && steps <= OPUS_FREE_MAX_STEPS
        && tier >= OPUS_MIN_TIER
}

/// Calculate vibe batch cost (free threshold then per-vibe charge)
public func calcVibeBatchCost(enabledVibeCount: Int) -> Int {
    return max(0, enabledVibeCount - VIBE_FREE_THRESHOLD) * VIBE_BATCH_PRICE
}

/// Calculate character reference cost
public func calcCharRefCost(charRefCount: Int, nSamples: Int) -> Int {
    return CHAR_REF_PRICE * charRefCount * nSamples
}

// MARK: - Pixel Adjustment Functions

/// Expand dimensions to meet minimum pixel count (maintaining aspect ratio)
public func expandToMinPixels(
    width: Int,
    height: Int,
    minPixels: Int
) -> (width: Int, height: Int, pixels: Int) {
    let pixels = width * height
    if pixels >= minPixels {
        return (width, height, pixels)
    }
    let scale = sqrt(Double(minPixels) / Double(pixels))
    let newW = Int(ceil(Double(width) * scale))
    var newH = Int(floor(Double(height) * scale))
    // Ensure we actually meet the minPixels requirement
    if newW * newH < minPixels {
        newH = Int(ceil(Double(height) * scale))
    }
    return (newW, newH, newW * newH)
}

/// Clamp dimensions to maximum pixel count (maintaining aspect ratio)
public func clampToMaxPixels(
    width: Int,
    height: Int,
    maxPixels: Int
) -> (width: Int, height: Int, pixels: Int) {
    let pixels = width * height
    if pixels <= maxPixels {
        return (width, height, pixels)
    }
    let scale = sqrt(Double(maxPixels) / Double(pixels))
    let newW = Int(floor(Double(width) * scale))
    let newH = Int(floor(Double(height) * scale))
    return (newW, newH, newW * newH)
}

// MARK: - Inpaint Correction

/// Calculate inpaint mask size correction
public func calcInpaintSizeCorrection(
    maskWidth: Int,
    maskHeight: Int
) -> InpaintCorrectionResult {
    if maskWidth <= 0 || maskHeight <= 0 {
        return InpaintCorrectionResult(corrected: false, width: maskWidth, height: maskHeight)
    }

    let pixels = maskWidth * maskHeight
    let threshold = Double(OPUS_FREE_PIXELS) * INPAINT_THRESHOLD_RATIO

    if Double(pixels) >= threshold {
        return InpaintCorrectionResult(corrected: false, width: maskWidth, height: maskHeight)
    }

    let scale = sqrt(Double(OPUS_FREE_PIXELS) / Double(pixels))
    // Scale dimensions then snap down to nearest GRID_SIZE multiple
    let scaledW = Int(floor(Double(maskWidth) * scale))
    let scaledH = Int(floor(Double(maskHeight) * scale))
    let newW = (scaledW / GRID_SIZE) * GRID_SIZE
    let newH = (scaledH / GRID_SIZE) * GRID_SIZE

    return InpaintCorrectionResult(corrected: true, width: newW, height: newH)
}

// MARK: - Main: Generation Cost Calculation

/// Calculate image generation Anlas cost (main orchestrator)
public func calculateGenerationCost(_ params: GenerationCostParams) throws -> GenerationCostResult {
    // Input validation
    try assertPositiveFiniteInt(params.width, "width")
    try assertPositiveFiniteInt(params.height, "height")
    try assertPositiveFiniteInt(params.steps, "steps")
    if let strength = params.strength {
        try assertFiniteRange(strength, 0, 1, "strength")
    }
    if let nSamples = params.nSamples {
        try assertNonNegativeFiniteInt(nSamples, "nSamples")
    }

    // Apply defaults
    let smea = params.smea ?? .off
    let mode = params.mode ?? .txt2img
    let strength = params.strength ?? 1.0
    let nSamples = params.nSamples ?? 1
    let tier = params.tier ?? 0
    let charRefCount = params.charRefCount ?? 0
    let vibeCount = params.vibeCount ?? 0
    let vibeUnencodedCount = params.vibeUnencodedCount ?? 0

    // Validate maskWidth/maskHeight pair
    if mode == .inpaint {
        let hasMaskW = params.maskWidth != nil
        let hasMaskH = params.maskHeight != nil
        if hasMaskW != hasMaskH {
            throw NovelAIError.range(
                "maskWidth and maskHeight must both be specified or both omitted for inpaint mode"
            )
        }
    }

    // Determine effective dimensions (inpaint mask correction)
    var effectiveWidth = params.width
    var effectiveHeight = params.height

    if mode == .inpaint, let maskW = params.maskWidth, let maskH = params.maskHeight {
        let correction = calcInpaintSizeCorrection(maskWidth: maskW, maskHeight: maskH)
        if correction.corrected {
            effectiveWidth = correction.width
            effectiveHeight = correction.height
        }
    }

    // Base cost calculation
    let baseCost = calcV4BaseCost(width: effectiveWidth, height: effectiveHeight, steps: params.steps)

    // SMEA multiplier
    let smeaMultiplier = getSmeaMultiplier(smea)
    let perImageCost = Double(baseCost) * smeaMultiplier

    // Strength multiplier (txt2img is always 1.0)
    let strengthMultiplier: Double
    switch mode {
    case .txt2img:
        strengthMultiplier = 1.0
    case .img2img, .inpaint:
        strengthMultiplier = strength
    }

    // Adjusted cost (minimum MIN_COST_PER_IMAGE guaranteed)
    let adjustedCost = max(Int(ceil(perImageCost * strengthMultiplier)), MIN_COST_PER_IMAGE)

    // Error check (max cost exceeded)
    let error = adjustedCost > MAX_COST_PER_IMAGE
    let errorCode: Int? = error ? -3 : nil

    // Opus free check (uses original request size)
    let isOpusFree = isOpusFreeGeneration(
        width: params.width,
        height: params.height,
        steps: params.steps,
        charRefCount: charRefCount,
        tier: tier,
        vibeCount: vibeCount
    )

    // Billable images
    let billableImages = max(0, nSamples - (isOpusFree ? 1 : 0))
    let generationCost = adjustedCost * billableImages

    // Vibe costs (disabled when using character references or inpaint)
    var vibeEncodeCost = 0
    var vibeBatchCost = 0
    if charRefCount == 0 && mode != .inpaint {
        vibeEncodeCost = vibeUnencodedCount * VIBE_ENCODE_PRICE
        vibeBatchCost = calcVibeBatchCost(enabledVibeCount: vibeCount)
    }

    // Character reference cost
    let charRefCost = charRefCount > 0 ? calcCharRefCost(charRefCount: charRefCount, nSamples: nSamples) : 0

    // Total cost (0 on error since value is unreliable)
    let totalCost = error ? 0 : generationCost + charRefCost + vibeEncodeCost + vibeBatchCost

    return GenerationCostResult(
        baseCost: baseCost,
        smeaMultiplier: smeaMultiplier,
        perImageCost: perImageCost,
        strengthMultiplier: strengthMultiplier,
        adjustedCost: adjustedCost,
        isOpusFree: isOpusFree,
        billableImages: billableImages,
        generationCost: generationCost,
        charRefCost: charRefCost,
        vibeEncodeCost: vibeEncodeCost,
        vibeBatchCost: vibeBatchCost,
        totalCost: totalCost,
        error: error,
        errorCode: errorCode
    )
}

// MARK: - Augment Cost Calculation

/// Calculate augment tool Anlas cost
public func calculateAugmentCost(_ params: AugmentCostParams) throws -> AugmentCostResult {
    // Input validation
    try assertPositiveFiniteInt(params.width, "width")
    try assertPositiveFiniteInt(params.height, "height")

    let tier = params.tier ?? 0
    // Safe: width * height is bounded by MAX_PIXELS validation upstream
    let originalPixels = params.width * params.height

    // Clamp to MAX_PIXELS
    let clamped = clampToMaxPixels(width: params.width, height: params.height, maxPixels: MAX_PIXELS)

    // Expand to AUGMENT_MIN_PIXELS
    let expanded = expandToMinPixels(width: clamped.width, height: clamped.height, minPixels: AUGMENT_MIN_PIXELS)

    // Base cost calculation (fixed steps)
    let baseCost = calcV4BaseCost(width: expanded.width, height: expanded.height, steps: AUGMENT_FIXED_STEPS)

    // bg-removal has special calculation
    let finalCost: Int
    if params.tool == .bgRemoval {
        finalCost = Int(ceil(Double(BG_REMOVAL_MULTIPLIER) * Double(baseCost) + Double(BG_REMOVAL_ADDEND)))
    } else {
        finalCost = baseCost
    }

    // Opus free check (not bg-removal, and expanded pixels <= OPUS_FREE_PIXELS)
    let isOpusFree = params.tool != .bgRemoval
        && expanded.width * expanded.height <= OPUS_FREE_PIXELS
        && tier >= OPUS_MIN_TIER

    let effectiveCost = isOpusFree ? 0 : finalCost

    return AugmentCostResult(
        originalPixels: originalPixels,
        adjustedWidth: expanded.width,
        adjustedHeight: expanded.height,
        adjustedPixels: expanded.width * expanded.height,
        baseCost: baseCost,
        finalCost: finalCost,
        isOpusFree: isOpusFree,
        effectiveCost: effectiveCost
    )
}

// MARK: - Upscale Cost Calculation

/// Calculate upscale Anlas cost using lookup table
public func calculateUpscaleCost(_ params: UpscaleCostParams) throws -> UpscaleCostResult {
    // Input validation
    try assertPositiveFiniteInt(params.width, "width")
    try assertPositiveFiniteInt(params.height, "height")

    let tier = params.tier ?? 0
    let pixels = params.width * params.height

    // Opus free check
    if tier >= OPUS_MIN_TIER && pixels <= UPSCALE_OPUS_FREE_PIXELS {
        return UpscaleCostResult(pixels: pixels, cost: 0, isOpusFree: true, error: false, errorCode: nil)
    }

    // Look up cost from table (ascending order, first match wins)
    for entry in UPSCALE_COST_TABLE {
        if pixels <= entry.maxPixels {
            return UpscaleCostResult(pixels: pixels, cost: entry.cost, isOpusFree: false, error: false, errorCode: nil)
        }
    }

    // No match in table -> error
    return UpscaleCostResult(pixels: pixels, cost: nil, isOpusFree: false, error: true, errorCode: -3)
}
