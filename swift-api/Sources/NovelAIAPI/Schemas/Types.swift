import Foundation

// MARK: - Action & Mode Enums

/// Generate action type
public enum GenerateAction: String, CaseIterable, Codable, Sendable {
    case generate
    case img2img
    case infill
}

/// Character reference mode
public enum CharRefMode: String, CaseIterable, Codable, Sendable {
    case character
    case characterAndStyle = "character&style"
    case style
}

// MARK: - Image Input

/// Image input source (file path, base64 string, data URL, or raw bytes)
public enum ImageInput: Sendable {
    case filePath(String)
    case base64(String)
    case dataURL(String)
    case bytes(Data)
}

// MARK: - Character Configuration

/// Configuration for a character prompt with position
public struct CharacterConfig: Sendable {
    public var prompt: String
    public var centerX: Double
    public var centerY: Double
    public var negativePrompt: String

    public init(
        prompt: String,
        centerX: Double = 0.5,
        centerY: Double = 0.5,
        negativePrompt: String = ""
    ) {
        self.prompt = prompt
        self.centerX = centerX
        self.centerY = centerY
        self.negativePrompt = negativePrompt
    }
}

/// Convert CharacterConfig to caption dict format
public func characterToCaptionDict(_ config: CharacterConfig) -> [String: Any] {
    return [
        "char_caption": config.prompt,
        "centers": [["x": config.centerX, "y": config.centerY]],
    ]
}

/// Convert CharacterConfig to negative caption dict format
public func characterToNegativeCaptionDict(_ config: CharacterConfig) -> [String: Any] {
    return [
        "char_caption": config.negativePrompt,
        "centers": [["x": config.centerX, "y": config.centerY]],
    ]
}

// MARK: - Character Reference Configuration

/// Configuration for character reference image
public struct CharacterReferenceConfig: Sendable {
    public var image: ImageInput
    public var strength: Double
    public var fidelity: Double
    public var mode: CharRefMode

    public init(
        image: ImageInput,
        strength: Double = 0.6,
        fidelity: Double = 1.0,
        mode: CharRefMode = .characterAndStyle
    ) {
        self.image = image
        self.strength = strength
        self.fidelity = fidelity
        self.mode = mode
    }
}

// MARK: - Vibe Types

/// Vibe encode result (pre-encoded vibe data)
public struct VibeEncodeResult: Sendable {
    public var encoding: String
    public var model: Model
    public var informationExtracted: Double
    public var strength: Double
    public var sourceImageHash: String
    public var createdAt: Date
    public var savedPath: String?
    public var anlasRemaining: Int?
    public var anlasConsumed: Int?

    public init(
        encoding: String,
        model: Model,
        informationExtracted: Double,
        strength: Double,
        sourceImageHash: String,
        createdAt: Date,
        savedPath: String? = nil,
        anlasRemaining: Int? = nil,
        anlasConsumed: Int? = nil
    ) {
        self.encoding = encoding
        self.model = model
        self.informationExtracted = informationExtracted
        self.strength = strength
        self.sourceImageHash = sourceImageHash
        self.createdAt = createdAt
        self.savedPath = savedPath
        self.anlasRemaining = anlasRemaining
        self.anlasConsumed = anlasConsumed
    }
}

/// Vibe item: either a pre-encoded result or a file path string
public enum VibeItem: Sendable {
    case encoded(VibeEncodeResult)
    case filePath(String)
}

// MARK: - Generate Parameters

/// Parameters for image generation.
/// All properties are mutable (`var`) for builder-style configuration.
/// Call `validate()` before use to enforce constraints.
public struct GenerateParams: Sendable {
    // Basic prompt
    public var prompt: String

    // Action & Image2Image
    public var action: GenerateAction
    public var sourceImage: ImageInput?
    public var img2imgStrength: Double
    public var img2imgNoise: Double

    // Inpaint/Mask
    public var mask: ImageInput?
    public var maskStrength: Double?
    public var inpaintColorCorrect: Bool

    // Hybrid Mode
    public var hybridImg2imgStrength: Double?
    public var hybridImg2imgNoise: Double?

    // Characters
    public var characters: [CharacterConfig]?

    // Vibe Transfer
    public var vibes: [VibeItem]?
    public var vibeStrengths: [Double]?
    public var vibeInfoExtracted: [Double]?

    // Character Reference
    public var characterReference: CharacterReferenceConfig?

    // Negative prompt
    public var negativePrompt: String?

    // Output options
    public var savePath: String?
    public var saveDir: String?

    // Generation parameters
    public var model: Model
    public var width: Int
    public var height: Int
    public var steps: Int
    public var scale: Double
    public var cfgRescale: Double
    public var seed: UInt32?
    public var sampler: Sampler
    public var noiseSchedule: NoiseSchedule

    public init(
        prompt: String,
        action: GenerateAction = .generate,
        sourceImage: ImageInput? = nil,
        img2imgStrength: Double = DEFAULT_IMG2IMG_STRENGTH,
        img2imgNoise: Double = 0.0,
        mask: ImageInput? = nil,
        maskStrength: Double? = nil,
        inpaintColorCorrect: Bool = DEFAULT_INPAINT_COLOR_CORRECT,
        hybridImg2imgStrength: Double? = nil,
        hybridImg2imgNoise: Double? = nil,
        characters: [CharacterConfig]? = nil,
        vibes: [VibeItem]? = nil,
        vibeStrengths: [Double]? = nil,
        vibeInfoExtracted: [Double]? = nil,
        characterReference: CharacterReferenceConfig? = nil,
        negativePrompt: String? = nil,
        savePath: String? = nil,
        saveDir: String? = nil,
        model: Model = DEFAULT_MODEL,
        width: Int = DEFAULT_WIDTH,
        height: Int = DEFAULT_HEIGHT,
        steps: Int = DEFAULT_STEPS,
        scale: Double = DEFAULT_SCALE,
        cfgRescale: Double = DEFAULT_CFG_RESCALE,
        seed: UInt32? = nil,
        sampler: Sampler = DEFAULT_SAMPLER,
        noiseSchedule: NoiseSchedule = DEFAULT_NOISE_SCHEDULE
    ) {
        self.prompt = prompt
        self.action = action
        self.sourceImage = sourceImage
        self.img2imgStrength = img2imgStrength
        self.img2imgNoise = img2imgNoise
        self.mask = mask
        self.maskStrength = maskStrength
        self.inpaintColorCorrect = inpaintColorCorrect
        self.hybridImg2imgStrength = hybridImg2imgStrength
        self.hybridImg2imgNoise = hybridImg2imgNoise
        self.characters = characters
        self.vibes = vibes
        self.vibeStrengths = vibeStrengths
        self.vibeInfoExtracted = vibeInfoExtracted
        self.characterReference = characterReference
        self.negativePrompt = negativePrompt
        self.savePath = savePath
        self.saveDir = saveDir
        self.model = model
        self.width = width
        self.height = height
        self.steps = steps
        self.scale = scale
        self.cfgRescale = cfgRescale
        self.seed = seed
        self.sampler = sampler
        self.noiseSchedule = noiseSchedule
    }
}

// MARK: - Generate Result

/// Result of image generation
public struct GenerateResult: Sendable {
    public var imageData: Data
    public var seed: UInt32
    public var anlasRemaining: Int?
    public var anlasConsumed: Int?
    public var savedPath: String?

    public init(
        imageData: Data,
        seed: UInt32,
        anlasRemaining: Int? = nil,
        anlasConsumed: Int? = nil,
        savedPath: String? = nil
    ) {
        self.imageData = imageData
        self.seed = seed
        self.anlasRemaining = anlasRemaining
        self.anlasConsumed = anlasConsumed
        self.savedPath = savedPath
    }
}

// MARK: - Encode Vibe Parameters

/// Parameters for vibe encoding
public struct EncodeVibeParams: Sendable {
    public var image: ImageInput
    public var model: Model
    public var informationExtracted: Double
    public var strength: Double
    public var savePath: String?
    public var saveDir: String?
    public var saveFilename: String?

    public init(
        image: ImageInput,
        model: Model = DEFAULT_MODEL,
        informationExtracted: Double = 0.7,
        strength: Double = 0.7,
        savePath: String? = nil,
        saveDir: String? = nil,
        saveFilename: String? = nil
    ) {
        self.image = image
        self.model = model
        self.informationExtracted = informationExtracted
        self.strength = strength
        self.savePath = savePath
        self.saveDir = saveDir
        self.saveFilename = saveFilename
    }
}

// MARK: - Augment Parameters & Result

/// Parameters for image augmentation
public struct AugmentParams: Sendable {
    public var reqType: AugmentReqType
    public var image: ImageInput
    public var prompt: String?
    public var defry: Int?
    public var savePath: String?
    public var saveDir: String?

    public init(
        reqType: AugmentReqType,
        image: ImageInput,
        prompt: String? = nil,
        defry: Int? = nil,
        savePath: String? = nil,
        saveDir: String? = nil
    ) {
        self.reqType = reqType
        self.image = image
        self.prompt = prompt
        self.defry = defry
        self.savePath = savePath
        self.saveDir = saveDir
    }
}

/// Result of image augmentation
public struct AugmentResult: Sendable {
    public var imageData: Data
    public var reqType: AugmentReqType
    public var anlasRemaining: Int?
    public var anlasConsumed: Int?
    public var savedPath: String?

    public init(
        imageData: Data,
        reqType: AugmentReqType,
        anlasRemaining: Int? = nil,
        anlasConsumed: Int? = nil,
        savedPath: String? = nil
    ) {
        self.imageData = imageData
        self.reqType = reqType
        self.anlasRemaining = anlasRemaining
        self.anlasConsumed = anlasConsumed
        self.savedPath = savedPath
    }
}

// MARK: - Upscale Parameters & Result

/// Parameters for image upscaling
public struct UpscaleParams: Sendable {
    public var image: ImageInput
    public var scale: Int
    public var savePath: String?
    public var saveDir: String?

    public init(
        image: ImageInput,
        scale: Int = DEFAULT_UPSCALE_SCALE,
        savePath: String? = nil,
        saveDir: String? = nil
    ) {
        self.image = image
        self.scale = scale
        self.savePath = savePath
        self.saveDir = saveDir
    }
}

/// Result of image upscaling
public struct UpscaleResult: Sendable {
    public var imageData: Data
    public var scale: Int
    public var outputWidth: Int
    public var outputHeight: Int
    public var anlasRemaining: Int?
    public var anlasConsumed: Int?
    public var savedPath: String?

    public init(
        imageData: Data,
        scale: Int,
        outputWidth: Int,
        outputHeight: Int,
        anlasRemaining: Int? = nil,
        anlasConsumed: Int? = nil,
        savedPath: String? = nil
    ) {
        self.imageData = imageData
        self.scale = scale
        self.outputWidth = outputWidth
        self.outputHeight = outputHeight
        self.anlasRemaining = anlasRemaining
        self.anlasConsumed = anlasConsumed
        self.savedPath = savedPath
    }
}

// MARK: - Anlas Balance

/// API response for Anlas balance
public struct AnlasBalance: Sendable {
    public var fixedTrainingStepsLeft: Int
    public var purchasedTrainingSteps: Int
    public var tier: Int

    public init(
        fixedTrainingStepsLeft: Int = 0,
        purchasedTrainingSteps: Int = 0,
        tier: Int = 0
    ) {
        self.fixedTrainingStepsLeft = fixedTrainingStepsLeft
        self.purchasedTrainingSteps = purchasedTrainingSteps
        self.tier = tier
    }
}
