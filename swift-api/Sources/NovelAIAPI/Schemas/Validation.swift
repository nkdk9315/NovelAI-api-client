import Foundation

// MARK: - Static Regex Constants

// Patterns are compile-time known literals; try! is safe.
// swiftlint:disable force_try
private let base64LikeRegex = try! NSRegularExpression(pattern: "^[A-Za-z0-9+/\\-_]+=*$")
private let base64StrictRegex = try! NSRegularExpression(pattern: "^[A-Za-z0-9+/]+={0,2}$")
private let sha256HexRegex = try! NSRegularExpression(pattern: "^[a-fA-F0-9]{64}$")
// swiftlint:enable force_try

// MARK: - Shared Validation Helpers

/// Validate path does not contain path traversal.
/// Uses component-level check on the raw path so that legitimate directory names
/// containing ".." substrings (e.g., "my..folder") are not rejected,
/// while actual traversal components ("..") are caught before symlink resolution.
func validateSafePath(_ path: String) throws {
    // Check raw path components before any resolution (prevents symlink bypass)
    let components = path.replacingOccurrences(of: "\\", with: "/")
        .components(separatedBy: "/")
    if components.contains("..") {
        throw NovelAIError.validation("Path must not contain '..' (path traversal)")
    }
}

/// Validate save_path and save_dir are mutually exclusive
func validateSaveOptionsExclusive(savePath: String?, saveDir: String?) throws {
    if savePath != nil && saveDir != nil {
        throw NovelAIError.validation(
            "save_path and save_dir cannot be specified together. Use one or the other."
        )
    }
}

/// Validate ImageInput is not empty
func validateImageInputNotEmpty(_ input: ImageInput) throws {
    switch input {
    case .filePath(let path):
        if path.isEmpty {
            throw NovelAIError.validation("Image file path must not be empty")
        }
    case .base64(let str):
        if str.isEmpty {
            throw NovelAIError.validation("Image base64 string must not be empty")
        }
    case .dataURL(let url):
        if url.isEmpty {
            throw NovelAIError.validation("Image data URL must not be empty")
        }
    case .bytes(let data):
        if data.isEmpty {
            throw NovelAIError.validation("Image data must not be empty")
        }
    }
}

/// Validate ImageInput path traversal (for string-based inputs)
func validateImageInputPath(_ input: ImageInput) throws {
    switch input {
    case .filePath(let path):
        try validateSafePath(path)
    case .dataURL:
        break  // data URLs don't need path traversal check
    case .base64(let str):
        // Skip traversal check for base64 strings (long, base64-like)
        let range = NSRange(str.startIndex..., in: str)
        if base64LikeRegex.firstMatch(in: str, range: range) != nil && str.count > 64 {
            break
        }
        // For file-path-like strings, check for path traversal
        try validateSafePath(str)
    case .bytes:
        break  // bytes don't need path traversal check
    }
}

// MARK: - GenerateParams Validation

extension GenerateParams {
    /// Validate all synchronous constraints
    public func validate() throws {
        try validateDimensions()
        try validateGenerationParameters()
        try validateImg2ImgParameters()
        try validateActionDependencies()
        try validateVibeParams()
        try validatePixelConstraints()
        try validateSaveOptions()
        try validateCharacters()
        try validateCharacterReference()
        try validateVibes()
    }

    private func validateDimensions() throws {
        if width < MIN_DIMENSION || width > MAX_GENERATION_DIMENSION {
            throw NovelAIError.range(
                "width must be between \(MIN_DIMENSION) and \(MAX_GENERATION_DIMENSION), got \(width)"
            )
        }
        if height < MIN_DIMENSION || height > MAX_GENERATION_DIMENSION {
            throw NovelAIError.range(
                "height must be between \(MIN_DIMENSION) and \(MAX_GENERATION_DIMENSION), got \(height)"
            )
        }
        if width % 64 != 0 {
            throw NovelAIError.validation("Width must be a multiple of 64")
        }
        if height % 64 != 0 {
            throw NovelAIError.validation("Height must be a multiple of 64")
        }
    }

    private func validateGenerationParameters() throws {
        if steps < MIN_STEPS || steps > MAX_STEPS {
            throw NovelAIError.range(
                "steps must be between \(MIN_STEPS) and \(MAX_STEPS), got \(steps)"
            )
        }
        if scale < MIN_SCALE || scale > MAX_SCALE {
            throw NovelAIError.range(
                "scale must be between \(MIN_SCALE) and \(MAX_SCALE), got \(scale)"
            )
        }
        if let seed = seed {
            if seed > MAX_SEED {
                throw NovelAIError.range(
                    "seed must be between 0 and \(MAX_SEED), got \(seed)"
                )
            }
        }
        if cfgRescale < 0 || cfgRescale > 1 {
            throw NovelAIError.range(
                "cfg_rescale must be between 0 and 1, got \(cfgRescale)"
            )
        }
    }

    private func validateImg2ImgParameters() throws {
        if img2imgStrength < 0.0 || img2imgStrength > 1.0 {
            throw NovelAIError.range(
                "img2img_strength must be between 0.0 and 1.0, got \(img2imgStrength)"
            )
        }
        if img2imgNoise < 0.0 || img2imgNoise > 1.0 {
            throw NovelAIError.range(
                "img2img_noise must be between 0.0 and 1.0, got \(img2imgNoise)"
            )
        }
        if let maskStrength = maskStrength {
            if maskStrength < 0.01 || maskStrength > 1.0 {
                throw NovelAIError.range(
                    "mask_strength must be between 0.01 and 1.0, got \(maskStrength)"
                )
            }
        }
        if let hybridStrength = hybridImg2imgStrength {
            if hybridStrength < 0.01 || hybridStrength > 0.99 {
                throw NovelAIError.range(
                    "hybrid_img2img_strength must be between 0.01 and 0.99, got \(hybridStrength)"
                )
            }
        }
        if let hybridNoise = hybridImg2imgNoise {
            if hybridNoise < 0.0 || hybridNoise > 0.99 {
                throw NovelAIError.range(
                    "hybrid_img2img_noise must be between 0.0 and 0.99, got \(hybridNoise)"
                )
            }
        }
    }

    private func validateActionDependencies() throws {
        // vibes and character_reference cannot be used together
        let hasVibes = vibes.map { !$0.isEmpty } ?? false
        if hasVibes && characterReference != nil {
            throw NovelAIError.validation(
                "vibes and character_reference cannot be used together."
            )
        }

        // action=img2img requires source_image
        if action == .img2img && sourceImage == nil {
            throw NovelAIError.validation(
                "source_image is required for img2img action"
            )
        }

        // action=infill requires source_image, mask, mask_strength
        if action == .infill {
            if sourceImage == nil {
                throw NovelAIError.validation(
                    "source_image is required for infill action"
                )
            }
            if mask == nil {
                throw NovelAIError.validation(
                    "mask is required for infill action"
                )
            }
            if maskStrength == nil {
                throw NovelAIError.validation(
                    "mask_strength is required for infill action"
                )
            }
        }

        // mask can only be used with action=infill
        if mask != nil && action != .infill {
            throw NovelAIError.validation(
                "mask can only be used with action='infill'"
            )
        }
    }

    private func validateVibeParams() throws {
        let hasVibes = vibes.map { !$0.isEmpty } ?? false

        // vibe_strengths without vibes
        let hasVibeStrengths = vibeStrengths.map { !$0.isEmpty } ?? false
        if hasVibeStrengths && !hasVibes {
            throw NovelAIError.validation(
                "vibe_strengths cannot be specified without vibes"
            )
        }

        // vibe_info_extracted without vibes
        let hasVibeInfo = vibeInfoExtracted.map { !$0.isEmpty } ?? false
        if hasVibeInfo && !hasVibes {
            throw NovelAIError.validation(
                "vibe_info_extracted cannot be specified without vibes"
            )
        }

        // Length mismatch checks
        if let vibes = vibes {
            if let strengths = vibeStrengths {
                if vibes.count != strengths.count {
                    throw NovelAIError.validation(
                        "Mismatch between vibes count (\(vibes.count)) and vibe_strengths count (\(strengths.count))"
                    )
                }
            }
            if let info = vibeInfoExtracted {
                if vibes.count != info.count {
                    throw NovelAIError.validation(
                        "Mismatch between vibes count (\(vibes.count)) and vibe_info_extracted count (\(info.count))"
                    )
                }
            }
        }
    }

    private func validatePixelConstraints() throws {
        let totalPixels = width * height
        if totalPixels > MAX_PIXELS {
            throw NovelAIError.validation(
                "Total pixels (\(totalPixels)) exceeds limit (\(MAX_PIXELS)). Current: \(width)x\(height)"
            )
        }
    }

    private func validateSaveOptions() throws {
        try validateSaveOptionsExclusive(savePath: savePath, saveDir: saveDir)
        if let path = savePath {
            try validateSafePath(path)
        }
        if let dir = saveDir {
            try validateSafePath(dir)
        }
    }

    private func validateCharacters() throws {
        if let characters = characters {
            if characters.count > MAX_CHARACTERS {
                throw NovelAIError.validation(
                    "characters count (\(characters.count)) exceeds maximum (\(MAX_CHARACTERS))"
                )
            }
            for character in characters {
                try character.validate()
            }
        }
    }

    private func validateCharacterReference() throws {
        if let charRef = characterReference {
            try charRef.validate()
        }
    }

    private func validateVibes() throws {
        if let vibes = vibes {
            if vibes.count > MAX_VIBES {
                throw NovelAIError.validation(
                    "vibes count (\(vibes.count)) exceeds maximum (\(MAX_VIBES))"
                )
            }
            for vibe in vibes {
                switch vibe {
                case .filePath(let path):
                    if path.isEmpty {
                        throw NovelAIError.validation("vibe file path must not be empty")
                    }
                case .encoded(let result):
                    try result.validate()
                }
            }
        }
    }
}

// MARK: - CharacterConfig Validation

extension CharacterConfig {
    public func validate() throws {
        if prompt.isEmpty {
            throw NovelAIError.validation("Character prompt must not be empty")
        }
        if centerX < 0.0 || centerX > 1.0 {
            throw NovelAIError.range("center_x must be between 0.0 and 1.0, got \(centerX)")
        }
        if centerY < 0.0 || centerY > 1.0 {
            throw NovelAIError.range("center_y must be between 0.0 and 1.0, got \(centerY)")
        }
    }
}

// MARK: - CharacterReferenceConfig Validation

extension CharacterReferenceConfig {
    public func validate() throws {
        try validateImageInputNotEmpty(image)
        try validateImageInputPath(image)
        if strength < 0.0 || strength > 1.0 {
            throw NovelAIError.range("strength must be between 0.0 and 1.0, got \(strength)")
        }
        if fidelity < 0.0 || fidelity > 1.0 {
            throw NovelAIError.range("fidelity must be between 0.0 and 1.0, got \(fidelity)")
        }
    }
}

// MARK: - VibeEncodeResult Validation

extension VibeEncodeResult {
    public func validate() throws {
        if encoding.isEmpty {
            throw NovelAIError.validation("encoding must not be empty")
        }
        if encoding.count > MAX_VIBE_ENCODING_LENGTH {
            throw NovelAIError.validation(
                "encoding length (\(encoding.count)) exceeds maximum (\(MAX_VIBE_ENCODING_LENGTH))"
            )
        }
        // Check base64 format (padding is at most 2 '=' characters)
        let range = NSRange(encoding.startIndex..., in: encoding)
        if base64StrictRegex.firstMatch(in: encoding, range: range) == nil {
            throw NovelAIError.validation("encoding must be valid base64")
        }
        if informationExtracted < 0.0 || informationExtracted > 1.0 {
            throw NovelAIError.range(
                "information_extracted must be between 0.0 and 1.0, got \(informationExtracted)"
            )
        }
        if strength < 0.0 || strength > 1.0 {
            throw NovelAIError.range("strength must be between 0.0 and 1.0, got \(strength)")
        }
        // Check SHA256 hash format
        let hashRange = NSRange(sourceImageHash.startIndex..., in: sourceImageHash)
        if sha256HexRegex.firstMatch(in: sourceImageHash, range: hashRange) == nil {
            throw NovelAIError.validation("source_image_hash must be a valid SHA256 hex string")
        }
    }
}

// MARK: - EncodeVibeParams Validation

extension EncodeVibeParams {
    public func validate() throws {
        try validateImageInputNotEmpty(image)
        try validateImageInputPath(image)

        if informationExtracted < 0.0 || informationExtracted > 1.0 {
            throw NovelAIError.range(
                "information_extracted must be between 0.0 and 1.0, got \(informationExtracted)"
            )
        }
        if strength < 0.0 || strength > 1.0 {
            throw NovelAIError.range("strength must be between 0.0 and 1.0, got \(strength)")
        }

        // save_path and save_dir are mutually exclusive
        try validateSaveOptionsExclusive(savePath: savePath, saveDir: saveDir)

        // Path traversal checks
        if let path = savePath {
            try validateSafePath(path)
        }
        if let dir = saveDir {
            try validateSafePath(dir)
        }

        // save_filename and save_path cannot be used together
        if saveFilename != nil && savePath != nil {
            throw NovelAIError.validation(
                "save_filename and save_path cannot be specified together. Use save_dir with save_filename instead."
            )
        }

        // save_filename requires save_dir
        if saveFilename != nil && saveDir == nil {
            throw NovelAIError.validation(
                "save_filename requires save_dir to be specified."
            )
        }
    }
}

// MARK: - AugmentParams Validation

extension AugmentParams {
    public func validate() throws {
        try validateImageInputNotEmpty(image)
        try validateImageInputPath(image)

        let reqTypeStr = reqType.rawValue

        // Types that require defry
        let requiresDefry = reqType == .colorize || reqType == .emotion

        // Types that disallow prompt and defry
        let noExtraParams = reqType == .declutter || reqType == .sketch
            || reqType == .lineart || reqType == .bgRemoval

        // colorize / emotion: defry is required
        if requiresDefry && defry == nil {
            throw NovelAIError.validation("defry (0-5) is required for \(reqTypeStr)")
        }

        // emotion: prompt is required and must be valid keyword
        if reqType == .emotion {
            guard let prompt = prompt, !prompt.isEmpty else {
                throw NovelAIError.validation(
                    "prompt is required for emotion (e.g., 'happy;;', 'sad;;')"
                )
            }
            let validKeywords = EmotionKeyword.allCases.map { $0.rawValue }
            if !validKeywords.contains(prompt) {
                throw NovelAIError.validation(
                    "Invalid emotion keyword '\(prompt)'. Valid: \(validKeywords.joined(separator: ", "))"
                )
            }
        }

        // declutter, sketch, lineart, bg-removal: prompt and defry must not be specified
        if noExtraParams {
            if let prompt = prompt, !prompt.isEmpty {
                throw NovelAIError.validation("prompt cannot be used with \(reqTypeStr)")
            }
            if defry != nil {
                throw NovelAIError.validation("defry cannot be used with \(reqTypeStr)")
            }
        }

        // defry range check
        if let defry = defry {
            if defry < MIN_DEFRY || defry > MAX_DEFRY {
                throw NovelAIError.range("defry must be between \(MIN_DEFRY) and \(MAX_DEFRY), got \(defry)")
            }
        }

        // save_path and save_dir exclusivity + path traversal
        try validateSaveOptionsExclusive(savePath: savePath, saveDir: saveDir)
        if let path = savePath {
            try validateSafePath(path)
        }
        if let dir = saveDir {
            try validateSafePath(dir)
        }
    }
}

// MARK: - UpscaleParams Validation

extension UpscaleParams {
    public func validate() throws {
        try validateImageInputNotEmpty(image)
        try validateImageInputPath(image)

        if !VALID_UPSCALE_SCALES.contains(scale) {
            throw NovelAIError.validation("scale must be one of: \(VALID_UPSCALE_SCALES.map(String.init).joined(separator: ", "))")
        }

        // save_path and save_dir exclusivity + path traversal
        try validateSaveOptionsExclusive(savePath: savePath, saveDir: saveDir)
        if let path = savePath {
            try validateSafePath(path)
        }
        if let dir = saveDir {
            try validateSafePath(dir)
        }
    }
}
