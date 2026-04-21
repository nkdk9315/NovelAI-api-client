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

    /// Internal initializer for testing - allows injecting a custom URLSession.
    internal init(apiKey: String, session: URLSession, logger: Logger? = nil) {
        self.apiKey = apiKey
        self.session = session
        self.logger = logger ?? DefaultLogger()
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
        let balanceBefore = await tryGetBalance()

        // Pre-flight balance check
        if let balance = balanceBefore {
            let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
            if VIBE_ENCODE_PRICE > total {
                throw NovelAIError.insufficientAnlas(required: VIBE_ENCODE_PRICE, available: total)
            }
        }

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
        let balanceAfter = await tryGetBalance()
        let anlasRemaining: Int? = balanceAfter.map { $0.fixedTrainingStepsLeft + $0.purchasedTrainingSteps }
        let anlasConsumed: Int?
        if let before = balanceBefore, let after = balanceAfter {
            let beforeTotal = before.fixedTrainingStepsLeft + before.purchasedTrainingSteps
            let afterTotal = after.fixedTrainingStepsLeft + after.purchasedTrainingSteps
            anlasConsumed = beforeTotal - afterTotal
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
        try applyImg2ImgParams(&payload, params: params)
        try applyInfillParams(&payload, params: params)

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
        let balanceBefore = await tryGetBalance()

        // Pre-flight balance check
        if let balance = balanceBefore {
            let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
            let genMode: GenerationMode
            let genStrength: Double
            switch params.action {
            case .img2img:
                genMode = .img2img
                genStrength = params.img2imgStrength
            case .infill:
                genMode = .inpaint
                genStrength = params.maskStrength ?? 1.0
            default:
                genMode = .txt2img
                genStrength = 1.0
            }
            if let costResult = try? calculateGenerationCost(GenerationCostParams(
                width: params.width,
                height: params.height,
                steps: params.steps,
                smea: .off,
                mode: genMode,
                strength: genStrength,
                nSamples: 1,
                tier: balance.tier,
                vibeCount: vibeEncodings.count,
                vibeUnencodedCount: 0
            )), !costResult.error, costResult.totalCost > total {
                throw NovelAIError.insufficientAnlas(required: costResult.totalCost, available: total)
            }
        }

        // 公式サイトはすべての generate フローを stream エンドポイントに送り、
        // multipart/form-data の `request` フィールド (filename `blob`) に
        // JSON ペイロードを格納する。レガシー非 stream 経路は早期/中間フレームを
        // 返すケースがありノイズ・低解像度状の出力につながるため使用しない。
        let payloadData = try JSONSerialization.data(withJSONObject: payload)
        guard let url = URL(string: streamURL()) else {
            throw NovelAIError.other("Invalid API URL")
        }
        let (multipartBody, contentType) = buildMultipartRequestBody(jsonPayload: payloadData)
        let request = buildRequest(url: url, method: "POST", body: multipartBody, contentType: contentType)

        let (responseData, _) = try await fetchWithRetry(
            request: request,
            session: session,
            operationName: "Generation",
            logger: logger
        )

        // Validate response size
        try validateResponseSize(responseData)

        // Parse response (always streamed msgpack)
        let imageData = try parseStreamResponse(responseData, logger: logger)

        // Get final balance
        let balanceAfter = await tryGetBalance()
        let anlasRemaining: Int? = balanceAfter.map { $0.fixedTrainingStepsLeft + $0.purchasedTrainingSteps }
        let anlasConsumed: Int?
        if let before = balanceBefore, let after = balanceAfter {
            let beforeTotal = before.fixedTrainingStepsLeft + before.purchasedTrainingSteps
            let afterTotal = after.fixedTrainingStepsLeft + after.purchasedTrainingSteps
            anlasConsumed = beforeTotal - afterTotal
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

        // Reject images exceeding MAX_PIXELS (matches official site behavior)
        let totalPixels = dims.width * dims.height
        if totalPixels > MAX_PIXELS {
            throw NovelAIError.validation(
                "Image resolution too high for augment (\(dims.width)x\(dims.height) = \(totalPixels) pixels, max: \(MAX_PIXELS)). " +
                "Resize the image to \(MAX_PIXELS) pixels or fewer before augmenting."
            )
        }
        let b64Image = dims.buffer.base64EncodedString()

        // Get initial balance
        let balanceBefore = await tryGetBalance()

        // Pre-flight balance check
        if let balance = balanceBefore {
            let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
            if let costResult = try? calculateAugmentCost(AugmentCostParams(
                tool: params.reqType,
                width: dims.width,
                height: dims.height,
                tier: balance.tier
            )), costResult.effectiveCost > total {
                throw NovelAIError.insufficientAnlas(required: costResult.effectiveCost, available: total)
            }
        }

        // Build payload
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
        let balanceAfter = await tryGetBalance()
        let anlasRemaining: Int? = balanceAfter.map { $0.fixedTrainingStepsLeft + $0.purchasedTrainingSteps }
        let anlasConsumed: Int?
        if let before = balanceBefore, let after = balanceAfter {
            let beforeTotal = before.fixedTrainingStepsLeft + before.purchasedTrainingSteps
            let afterTotal = after.fixedTrainingStepsLeft + after.purchasedTrainingSteps
            anlasConsumed = beforeTotal - afterTotal
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

        // Validate upscale pixel limit (matches official site behavior)
        let pixels = dims.width * dims.height
        if pixels > UPSCALE_MAX_PIXELS {
            throw NovelAIError.validation(
                "Image resolution too high for upscale (\(dims.width)x\(dims.height) = \(pixels) pixels, max: \(UPSCALE_MAX_PIXELS)). " +
                "Resize the image to \(UPSCALE_MAX_PIXELS) pixels or fewer before upscaling."
            )
        }

        let b64Image = dims.buffer.base64EncodedString()

        // Get initial balance
        let balanceBefore = await tryGetBalance()

        // Pre-flight balance check
        if let balance = balanceBefore {
            let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
            if let costResult = try? calculateUpscaleCost(UpscaleCostParams(
                width: dims.width,
                height: dims.height,
                tier: balance.tier
            )), !costResult.error, let cost = costResult.cost, cost > total {
                throw NovelAIError.insufficientAnlas(required: cost, available: total)
            }
        }

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
        let balanceAfter = await tryGetBalance()
        let anlasRemaining: Int? = balanceAfter.map { $0.fixedTrainingStepsLeft + $0.purchasedTrainingSteps }
        let anlasConsumed: Int?
        if let before = balanceBefore, let after = balanceAfter {
            let beforeTotal = before.fixedTrainingStepsLeft + before.purchasedTrainingSteps
            let afterTotal = after.fixedTrainingStepsLeft + after.purchasedTrainingSteps
            anlasConsumed = beforeTotal - afterTotal
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

    /// Build a multipart/form-data body containing the JSON payload as a `request`
    /// field with filename `blob` (matches the official site's request format
    /// for `/ai/generate-image-stream`). Returns the body bytes and the
    /// `Content-Type` header value (with the generated boundary).
    private func buildMultipartRequestBody(jsonPayload: Data) -> (Data, String) {
        let boundary = "----NovelAIAPIBoundary\(UUID().uuidString)"
        var body = Data()
        let lineBreak = "\r\n"
        body.append("--\(boundary)\(lineBreak)".data(using: .utf8)!)
        body.append("Content-Disposition: form-data; name=\"request\"; filename=\"blob\"\(lineBreak)".data(using: .utf8)!)
        body.append("Content-Type: application/json\(lineBreak)\(lineBreak)".data(using: .utf8)!)
        body.append(jsonPayload)
        body.append("\(lineBreak)--\(boundary)--\(lineBreak)".data(using: .utf8)!)
        return (body, "multipart/form-data; boundary=\(boundary)")
    }

    /// Build an HTTP request with authorization header and optional content type.
    private func buildRequest(url: URL, method: String, body: Data?, contentType: String?) -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue(USER_AGENT, forHTTPHeaderField: "User-Agent")
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
    private func tryGetBalance() async -> AnlasBalance? {
        do {
            return try await getAnlasBalance()
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
