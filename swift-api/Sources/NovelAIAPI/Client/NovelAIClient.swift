import Foundation
import CryptoKit

// MARK: - NovelAI Client

/// Main client for interacting with the NovelAI Image Generation API.
///
/// Provides methods for image generation, vibe encoding, image augmentation,
/// image upscaling, and balance checking. All API calls include automatic
/// retry logic with exponential backoff for rate limiting and transient errors.
///
/// Usage:
/// ```swift
/// let client = try NovelAIClient(apiKey: "your-api-key")
/// let result = try await client.generate(GenerateParams(prompt: "1girl, masterpiece"))
/// ```
public final class NovelAIClient: @unchecked Sendable {
    private let apiKey: String
    private let logger: Logger
    private let session: URLSession

    // MARK: - Initialization

    /// Create a new NovelAI client.
    ///
    /// - Parameters:
    ///   - apiKey: NovelAI API key. Falls back to `NAI_API_KEY` environment variable.
    ///   - logger: Logger for warnings and errors. Defaults to `DefaultLogger`.
    /// - Throws: `NovelAIError.api` if no API key is found.
    public init(apiKey: String? = nil, logger: Logger? = nil) throws {
        let resolvedKey = apiKey
            ?? ProcessInfo.processInfo.environment["NAI_API_KEY"]
            ?? ProcessInfo.processInfo.environment["NOVELAI_API_KEY"]
            ?? ""

        if resolvedKey.isEmpty {
            throw NovelAIError.api(
                statusCode: 0,
                message: "API key is required. Set NAI_API_KEY or NOVELAI_API_KEY environment variable or pass apiKey parameter."
            )
        }

        self.apiKey = resolvedKey
        self.logger = logger ?? DefaultLogger()
        self.session = URLSession.shared
    }

    // MARK: - Public API: Anlas Balance

    /// Get the current Anlas (training steps) balance.
    ///
    /// - Returns: The current balance including fixed, purchased, and total amounts.
    /// - Throws: `NovelAIError.api` on HTTP errors, `NovelAIError.parse` on response parsing failure.
    public func getAnlasBalance() async throws -> AnlasBalance {
        guard let url = URL(string: subscriptionURL()) else {
            throw NovelAIError.other("Invalid subscription URL")
        }
        var request = buildRequest(url: url, method: "GET", body: nil, contentType: nil)
        request.setValue("application/json", forHTTPHeaderField: "Accept")

        let (data, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "GetAnlasBalance",
            logger: logger
        )

        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw NovelAIError.parse("Failed to parse subscription response as JSON")
        }

        guard let trainingStepsLeft = json["trainingStepsLeft"] as? [String: Any] else {
            throw NovelAIError.parse("Missing trainingStepsLeft in subscription response")
        }

        let fixed = trainingStepsLeft["fixedTrainingStepsLeft"] as? Int ?? 0
        let purchased = trainingStepsLeft["purchasedTrainingSteps"] as? Int ?? 0
        let tier = json["tier"] as? Int ?? 0

        return AnlasBalance(
            fixedTrainingStepsLeft: fixed,
            purchasedTrainingSteps: purchased,
            tier: tier
        )
    }

    // MARK: - Public API: Vibe Encode

    /// Encode an image for Vibe Transfer (costs 2 Anlas).
    ///
    /// - Parameter params: Encoding parameters including image and model.
    /// - Returns: The encoded vibe data with optional save path.
    /// - Throws: `NovelAIError` on validation, API, or I/O errors.
    public func encodeVibe(_ params: EncodeVibeParams) async throws -> VibeEncodeResult {
        // Validate parameters
        try params.validate()

        // Get image data
        let imageBuffer = try getImageBuffer(params.image)
        let b64Image = imageBuffer.base64EncodedString()

        // Calculate hash
        let sourceHash = sha256Hex(imageBuffer)

        // Get initial balance
        let anlasBefore = await tryGetBalance()

        // Build payload
        let payload: [String: Any] = [
            "image": b64Image,
            "information_extracted": params.informationExtracted,
            "model": params.model.rawValue,
        ]

        let payloadData = try JSONSerialization.data(withJSONObject: payload)

        guard let url = URL(string: encodeURL()) else {
            throw NovelAIError.other("Invalid encode URL")
        }
        var request = buildRequest(url: url, method: "POST", body: payloadData, contentType: "application/json")
        request.setValue("*/*", forHTTPHeaderField: "Accept")

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "VibeEncode",
            logger: logger
        )

        // Validate response size
        try validateResponseSize(responseData)

        let encoding = responseData.base64EncodedString()

        // Get final balance
        let anlasAfter = await tryGetBalance()
        let anlasRemaining = anlasAfter
        let anlasConsumed: Int?
        if let before = anlasBefore, let after = anlasAfter {
            anlasConsumed = before - after
        } else {
            anlasConsumed = nil
        }

        var result = VibeEncodeResult(
            encoding: encoding,
            model: params.model,
            informationExtracted: params.informationExtracted,
            strength: params.strength,
            sourceImageHash: sourceHash,
            createdAt: Date(),
            savedPath: nil,
            anlasRemaining: anlasRemaining,
            anlasConsumed: anlasConsumed
        )

        // Save if requested
        do {
            if let savePath = params.savePath {
                try saveVibe(result, path: savePath)
                result.savedPath = savePath
            } else if let saveDir = params.saveDir {
                try ensureDirectory(saveDir)

                let filename: String
                if let customName = params.saveFilename {
                    let baseName = customName.hasSuffix(".naiv4vibe")
                        ? String(customName.dropLast(".naiv4vibe".count))
                        : customName
                    filename = "\(baseName).naiv4vibe"
                } else {
                    let timestamp = formatTimestamp()
                    filename = "\(String(sourceHash.prefix(12)))_\(timestamp).naiv4vibe"
                }

                let savePath = (saveDir as NSString).appendingPathComponent(filename)
                try saveVibe(result, path: savePath)
                result.savedPath = savePath
            }
        } catch {
            logger.warn("[NovelAI] Failed to save vibe file: \(error.localizedDescription)")
        }

        return result
    }

    // MARK: - Public API: Generate

    /// Generate an image using the NovelAI image generation API.
    ///
    /// Supports text-to-image, image-to-image (img2img), and inpainting (infill) modes.
    /// Can optionally use vibe transfer, character references, and character prompts.
    ///
    /// - Parameter params: Generation parameters including prompt, model, dimensions, etc.
    /// - Returns: The generated image data with metadata.
    /// - Throws: `NovelAIError` on validation, API, or I/O errors.
    public func generate(_ params: GenerateParams) async throws -> GenerateResult {
        // Validate parameters (synchronous validation)
        try params.validate()

        // Defaults
        let negativePrompt = params.negativePrompt ?? DEFAULT_NEGATIVE
        let seed = params.seed ?? UInt32.random(in: 0...MAX_SEED)

        // Process Character References
        var charRefData: ProcessedCharacterReferences?
        if let charRef = params.characterReference {
            charRefData = try processCharacterReferences([charRef])
        }

        // Process Vibes
        var vibeEncodings: [String] = []
        var vibeInfoList: [Double] = []
        var vibeStrengths = params.vibeStrengths

        if let vibes = params.vibes, !vibes.isEmpty {
            let processed = try processVibes(vibes, model: params.model)
            vibeEncodings = processed.encodings
            vibeInfoList = params.vibeInfoExtracted ?? processed.infoExtractedList

            if vibeStrengths == nil {
                vibeStrengths = Array(repeating: DEFAULT_VIBE_STRENGTH, count: vibeEncodings.count)
            }
        }

        // Character Configs
        let charConfigs = params.characters ?? []
        var charCaptions: [[String: Any]] = []
        var charNegativeCaptions: [[String: Any]] = []

        if !charConfigs.isEmpty {
            charCaptions = charConfigs.map { characterToCaptionDict($0) }
            charNegativeCaptions = charConfigs.map { characterToNegativeCaptionDict($0) }
        }

        // Build payload using helper methods
        var payload = buildBasePayload(params, seed: seed, negativePrompt: negativePrompt)

        // Apply action-specific parameters
        try applyImg2ImgParams(&payload, params: params, seed: seed)
        try applyInfillParams(&payload, params: params, seed: seed)

        // Apply additional features
        applyVibeParams(&payload, vibeEncodings: vibeEncodings, vibeStrengths: vibeStrengths, vibeInfoList: vibeInfoList)

        if let charRefs = charRefData {
            applyCharRefParams(&payload, charRefs: charRefs)
        }

        // Build prompt structures
        applyV4PromptStructures(
            &payload,
            prompt: params.prompt,
            negativePrompt: negativePrompt,
            charCaptions: charCaptions,
            charNegativeCaptions: charNegativeCaptions
        )
        applyCharacterPrompts(&payload, params: params)

        // Get initial balance
        let anlasBefore = await tryGetBalance()

        // Choose endpoint: stream for charRef or infill
        let useStream = (params.characterReference != nil) || (params.action == .infill)
        let apiUrlString = useStream ? streamURL() : apiURL()

        let payloadData = try JSONSerialization.data(withJSONObject: payload)
        guard let url = URL(string: apiUrlString) else {
            throw NovelAIError.other("Invalid API URL")
        }
        let request = buildRequest(url: url, method: "POST", body: payloadData, contentType: "application/json")

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Generation",
            logger: logger
        )

        // Validate response size
        try validateResponseSize(responseData)

        // Parse response
        let imageData: Data
        if useStream {
            imageData = try parseStreamResponse(responseData, logger: logger)
        } else {
            imageData = try parseZipResponse(responseData)
        }

        // Get final balance
        let anlasAfter = await tryGetBalance()
        let anlasRemaining = anlasAfter
        let anlasConsumed: Int?
        if let before = anlasBefore, let after = anlasAfter {
            anlasConsumed = before - after
        } else {
            anlasConsumed = nil
        }

        var result = GenerateResult(
            imageData: imageData,
            seed: seed,
            anlasRemaining: anlasRemaining,
            anlasConsumed: anlasConsumed,
            savedPath: nil
        )

        // Save if requested
        // Note: File operations here have an inherent TOCTOU race (directory check vs write).
        // This is acceptable because save failures are non-fatal and logged as warnings.
        do {
            if let savePath = params.savePath {
                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            } else if let saveDir = params.saveDir {
                try ensureDirectory(saveDir)

                var prefix = params.action == .img2img ? "img2img" : "gen"
                if !charConfigs.isEmpty { prefix += "_multi" }

                let timestamp = formatTimestamp()
                let filename = "\(prefix)_\(timestamp)_\(seed).png"
                let savePath = (saveDir as NSString).appendingPathComponent(filename)

                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            }
        } catch {
            logger.warn("[NovelAI] Failed to save image: \(error.localizedDescription)")
        }

        return result
    }

    // MARK: - Public API: Augment Image

    /// Augment an image using NovelAI's image processing tools.
    ///
    /// Supports colorize, declutter, emotion, sketch, lineart, and background removal.
    ///
    /// - Parameter params: Augment parameters including image and request type.
    /// - Returns: The augmented image data with metadata.
    /// - Throws: `NovelAIError` on validation, API, or I/O errors.
    public func augmentImage(_ params: AugmentParams) async throws -> AugmentResult {
        // Validate parameters
        try params.validate()

        // Get image data and auto-detect dimensions
        let dims = try getImageDimensions(params.image)
        let b64Image = dims.buffer.base64EncodedString()

        // Get initial balance
        let anlasBefore = await tryGetBalance()

        // Build payload with auto-detected dimensions
        var payload: [String: Any] = [
            "req_type": params.reqType.rawValue,
            "use_new_shared_trial": true,
            "width": dims.width,
            "height": dims.height,
            "image": b64Image,
        ]

        // Add prompt and defry only for colorize and emotion
        if params.reqType == .colorize {
            if let prompt = params.prompt {
                payload["prompt"] = prompt
            }
            payload["defry"] = params.defry
        } else if params.reqType == .emotion {
            if let prompt = params.prompt {
                payload["prompt"] = "\(prompt);;"
            }
            payload["defry"] = params.defry
        }

        let payloadData = try JSONSerialization.data(withJSONObject: payload)
        guard let url = URL(string: augmentURL()) else {
            throw NovelAIError.other("Invalid augment URL")
        }
        let request = buildRequest(url: url, method: "POST", body: payloadData, contentType: "application/json")

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Augment",
            logger: logger
        )

        // Validate response size
        try validateResponseSize(responseData)

        let imageData = try parseZipResponse(responseData)

        // Get final balance
        let anlasAfter = await tryGetBalance()
        let anlasRemaining = anlasAfter
        let anlasConsumed: Int?
        if let before = anlasBefore, let after = anlasAfter {
            anlasConsumed = before - after
        } else {
            anlasConsumed = nil
        }

        var result = AugmentResult(
            imageData: imageData,
            reqType: params.reqType,
            anlasRemaining: anlasRemaining,
            anlasConsumed: anlasConsumed,
            savedPath: nil
        )

        // Save if requested
        do {
            if let savePath = params.savePath {
                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            } else if let saveDir = params.saveDir {
                try ensureDirectory(saveDir)

                let timestamp = formatTimestamp()
                let rand = randomHex(bytes: 2)
                let filename = "\(params.reqType.rawValue)_\(timestamp)_\(rand).png"
                let savePath = (saveDir as NSString).appendingPathComponent(filename)

                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            }
        } catch {
            logger.warn("[NovelAI] Failed to save augmented image: \(error.localizedDescription)")
        }

        return result
    }

    // MARK: - Public API: Upscale Image

    /// Upscale an image using NovelAI's upscaling API.
    ///
    /// - Parameter params: Upscale parameters including image and scale factor.
    /// - Returns: The upscaled image data with metadata.
    /// - Throws: `NovelAIError` on validation, API, or I/O errors.
    public func upscaleImage(_ params: UpscaleParams) async throws -> UpscaleResult {
        // Validate parameters
        try params.validate()

        // Get image data and auto-detect dimensions
        let dims = try getImageDimensions(params.image)
        let b64Image = dims.buffer.base64EncodedString()

        // Get initial balance
        let anlasBefore = await tryGetBalance()

        let payload: [String: Any] = [
            "image": b64Image,
            "width": dims.width,
            "height": dims.height,
            "scale": params.scale,
        ]

        let payloadData = try JSONSerialization.data(withJSONObject: payload)
        guard let url = URL(string: upscaleURL()) else {
            throw NovelAIError.other("Invalid upscale URL")
        }
        let request = buildRequest(url: url, method: "POST", body: payloadData, contentType: "application/json")

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Upscale",
            logger: logger
        )

        // Validate response size
        try validateResponseSize(responseData)

        // Response can be ZIP or raw image
        let imageData: Data
        if responseData.count > 1 && responseData[responseData.startIndex] == 0x50 && responseData[responseData.startIndex + 1] == 0x4B {
            imageData = try parseZipResponse(responseData)
        } else {
            imageData = responseData
        }

        // Get final balance
        let anlasAfter = await tryGetBalance()
        let anlasRemaining = anlasAfter
        let anlasConsumed: Int?
        if let before = anlasBefore, let after = anlasAfter {
            anlasConsumed = before - after
        } else {
            anlasConsumed = nil
        }

        let outputWidth = dims.width * params.scale
        let outputHeight = dims.height * params.scale

        var result = UpscaleResult(
            imageData: imageData,
            scale: params.scale,
            outputWidth: outputWidth,
            outputHeight: outputHeight,
            anlasRemaining: anlasRemaining,
            anlasConsumed: anlasConsumed,
            savedPath: nil
        )

        // Save if requested
        do {
            if let savePath = params.savePath {
                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            } else if let saveDir = params.saveDir {
                try ensureDirectory(saveDir)

                let timestamp = formatTimestamp()
                let rand = randomHex(bytes: 2)
                let filename = "upscale_\(params.scale)x_\(timestamp)_\(rand).png"
                let savePath = (saveDir as NSString).appendingPathComponent(filename)

                try saveImage(data: imageData, path: savePath)
                result.savedPath = savePath
            }
        } catch {
            logger.warn("[NovelAI] Failed to save upscaled image: \(error.localizedDescription)")
        }

        return result
    }

    // MARK: - Private Helpers: Request Building

    /// Build an HTTP request with authorization header and optional content type.
    private func buildRequest(url: URL, method: String, body: Data?, contentType: String?) -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        if let contentType = contentType {
            request.setValue(contentType, forHTTPHeaderField: "Content-Type")
        }
        if let body = body {
            request.httpBody = body
        }
        return request
    }

    // MARK: - Private Helpers: Balance

    /// Try to get the current Anlas balance, returning nil on error.
    private func tryGetBalance() async -> Int? {
        do {
            let balance = try await getAnlasBalance()
            return balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
        } catch {
            logger.warn("[NovelAI] Failed to get Anlas balance: \(error.localizedDescription)")
            return nil
        }
    }

    // MARK: - Private Helpers: File I/O

    /// Validate a save path to prevent path traversal (delegates to centralized validateSafePath).
    private func validateSavePathTraversal(_ savePath: String) throws -> String {
        try validateSafePath(savePath)
        return (savePath as NSString).resolvingSymlinksInPath
    }

    /// Ensure a directory exists, creating it and any intermediate directories if necessary.
    private func ensureDirectory(_ dir: String) throws {
        try FileManager.default.createDirectory(
            atPath: dir,
            withIntermediateDirectories: true,
            attributes: nil
        )
    }

    /// Save image data to a file, creating parent directories as needed.
    private func saveImage(data: Data, path: String) throws {
        let normalized = try validateSavePathTraversal(path)
        let parentDir = (normalized as NSString).deletingLastPathComponent
        try ensureDirectory(parentDir)
        try data.write(to: URL(fileURLWithPath: normalized))
    }

    /// Save vibe encode result as a .naiv4vibe JSON file.
    private func saveVibe(_ result: VibeEncodeResult, path: String) throws {
        let normalized = try validateSavePathTraversal(path)
        let parentDir = (normalized as NSString).deletingLastPathComponent
        try ensureDirectory(parentDir)

        guard let modelKey = MODEL_KEY_MAP[result.model] else {
            throw NovelAIError.validation("Unknown model for vibe save: \(result.model.rawValue)")
        }

        let vibeData: [String: Any] = [
            "identifier": "novelai-vibe-transfer",
            "version": 1,
            "type": "encoding",
            "id": result.sourceImageHash,
            "encodings": [
                modelKey: [
                    "unknown": [
                        "encoding": result.encoding,
                        "params": [
                            "information_extracted": result.informationExtracted,
                        ] as [String: Any],
                    ] as [String: Any],
                ] as [String: Any],
            ] as [String: Any],
            "name": "\(String(result.sourceImageHash.prefix(6)))-\(String(result.sourceImageHash.suffix(6)))",
            "createdAt": ISO8601DateFormatter().string(from: result.createdAt),
            "importInfo": [
                "model": result.model.rawValue,
                "information_extracted": result.informationExtracted,
                "strength": result.strength,
            ] as [String: Any],
        ]

        let jsonData = try JSONSerialization.data(withJSONObject: vibeData, options: [.prettyPrinted, .sortedKeys])
        try jsonData.write(to: URL(fileURLWithPath: normalized))
    }

    // MARK: - Private Helpers: Utilities

    /// Validate response data does not exceed maximum size.
    private func validateResponseSize(_ data: Data) throws {
        if data.count > MAX_RESPONSE_SIZE {
            throw NovelAIError.parse("Response too large: \(data.count) bytes (max \(MAX_RESPONSE_SIZE))")
        }
    }

    /// Calculate SHA256 hex string of data.
    private func sha256Hex(_ data: Data) -> String {
        let digest = SHA256.hash(data: data)
        return digest.map { String(format: "%02x", $0) }.joined()
    }

    /// Format current timestamp for filenames (YYYYMMDDHHmmssS format).
    private func formatTimestamp() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyyMMddHHmmssS"
        formatter.timeZone = TimeZone(identifier: "UTC")
        return formatter.string(from: Date())
    }

    /// Generate random hex string.
    private func randomHex(bytes count: Int) -> String {
        return (0..<count).map { _ in
            String(format: "%02x", UInt8.random(in: 0...255))
        }.joined()
    }
}
