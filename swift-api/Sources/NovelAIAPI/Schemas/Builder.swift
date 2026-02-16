import Foundation

/// Builder for GenerateParams with method chaining and validation on build
public class GenerateParamsBuilder {
    private var params: GenerateParams

    /// Create a new builder with the given prompt
    public init(prompt: String) {
        self.params = GenerateParams(prompt: prompt)
    }

    // MARK: - Action & Image

    @discardableResult
    public func action(_ action: GenerateAction) -> Self {
        params.action = action
        return self
    }

    @discardableResult
    public func sourceImage(_ image: ImageInput) -> Self {
        params.sourceImage = image
        return self
    }

    @discardableResult
    public func img2imgStrength(_ strength: Double) -> Self {
        params.img2imgStrength = strength
        return self
    }

    @discardableResult
    public func img2imgNoise(_ noise: Double) -> Self {
        params.img2imgNoise = noise
        return self
    }

    // MARK: - Inpaint/Mask

    @discardableResult
    public func mask(_ mask: ImageInput) -> Self {
        params.mask = mask
        return self
    }

    @discardableResult
    public func maskStrength(_ strength: Double) -> Self {
        params.maskStrength = strength
        return self
    }

    @discardableResult
    public func inpaintColorCorrect(_ enabled: Bool) -> Self {
        params.inpaintColorCorrect = enabled
        return self
    }

    // MARK: - Hybrid Mode

    @discardableResult
    public func hybridImg2imgStrength(_ strength: Double) -> Self {
        params.hybridImg2imgStrength = strength
        return self
    }

    @discardableResult
    public func hybridImg2imgNoise(_ noise: Double) -> Self {
        params.hybridImg2imgNoise = noise
        return self
    }

    // MARK: - Characters & References

    @discardableResult
    public func characters(_ characters: [CharacterConfig]) -> Self {
        params.characters = characters
        return self
    }

    @discardableResult
    public func vibes(_ vibes: [VibeItem]) -> Self {
        params.vibes = vibes
        return self
    }

    @discardableResult
    public func vibeStrengths(_ strengths: [Double]) -> Self {
        params.vibeStrengths = strengths
        return self
    }

    @discardableResult
    public func vibeInfoExtracted(_ info: [Double]) -> Self {
        params.vibeInfoExtracted = info
        return self
    }

    @discardableResult
    public func characterReference(_ charRef: CharacterReferenceConfig) -> Self {
        params.characterReference = charRef
        return self
    }

    // MARK: - Prompts

    @discardableResult
    public func negativePrompt(_ prompt: String) -> Self {
        params.negativePrompt = prompt
        return self
    }

    // MARK: - Output Options

    @discardableResult
    public func savePath(_ path: String) -> Self {
        params.savePath = path
        return self
    }

    @discardableResult
    public func saveDir(_ dir: String) -> Self {
        params.saveDir = dir
        return self
    }

    // MARK: - Generation Parameters

    @discardableResult
    public func model(_ model: Model) -> Self {
        params.model = model
        return self
    }

    @discardableResult
    public func width(_ width: Int) -> Self {
        params.width = width
        return self
    }

    @discardableResult
    public func height(_ height: Int) -> Self {
        params.height = height
        return self
    }

    @discardableResult
    public func steps(_ steps: Int) -> Self {
        params.steps = steps
        return self
    }

    @discardableResult
    public func scale(_ scale: Double) -> Self {
        params.scale = scale
        return self
    }

    @discardableResult
    public func cfgRescale(_ cfgRescale: Double) -> Self {
        params.cfgRescale = cfgRescale
        return self
    }

    @discardableResult
    public func seed(_ seed: UInt32) -> Self {
        params.seed = seed
        return self
    }

    @discardableResult
    public func sampler(_ sampler: Sampler) -> Self {
        params.sampler = sampler
        return self
    }

    @discardableResult
    public func noiseSchedule(_ noiseSchedule: NoiseSchedule) -> Self {
        params.noiseSchedule = noiseSchedule
        return self
    }

    // MARK: - Build

    /// Build and validate the params. Throws if validation fails.
    public func build() throws -> GenerateParams {
        try params.validate()
        return params
    }
}

// MARK: - Convenience

extension GenerateParams {
    /// Create a builder with the given prompt
    public static func builder(prompt: String) -> GenerateParamsBuilder {
        return GenerateParamsBuilder(prompt: prompt)
    }
}
