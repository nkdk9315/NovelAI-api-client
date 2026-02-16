import Foundation

// MARK: - Processed Vibes Result

/// Result of processing vibes for API payload construction.
public struct ProcessedVibes: Sendable {
    public var encodings: [String]
    public var infoExtractedList: [Double]

    public init(encodings: [String] = [], infoExtractedList: [Double] = []) {
        self.encodings = encodings
        self.infoExtractedList = infoExtractedList
    }
}

// MARK: - Public Functions

/// Load and parse a .naiv4vibe JSON file.
public func loadVibeFile(_ vibePath: String) throws -> [String: Any] {
    do {
        try validateSafePath(vibePath)
    } catch {
        throw NovelAIError.image("Invalid file path (path traversal detected): \(vibePath)")
    }
    let normalized = (vibePath as NSString).resolvingSymlinksInPath

    guard let data = FileManager.default.contents(atPath: normalized) else {
        throw NovelAIError.image("Failed to read vibe file '\(vibePath)'")
    }

    guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw NovelAIError.image("Failed to parse vibe file '\(vibePath)'")
    }

    return json
}

/// Extract encoding and information_extracted from vibe data for a given model.
public func extractEncoding(
    _ vibeData: [String: Any],
    model: Model = .naiDiffusion45Full
) throws -> (encoding: String, informationExtracted: Double) {
    guard let modelKey = MODEL_KEY_MAP[model] else {
        throw NovelAIError.validation("Unknown model for vibe encoding: \(model.rawValue)")
    }

    let encodings = vibeData["encodings"] as? [String: Any] ?? [:]
    let modelEncodings = encodings[modelKey] as? [String: Any] ?? [:]

    guard let firstKey = modelEncodings.keys.first,
          let encodingData = modelEncodings[firstKey] as? [String: Any] else {
        throw NovelAIError.image("No encoding found for model key: \(modelKey)")
    }

    let encoding = encodingData["encoding"] as? String ?? ""

    let params = encodingData["params"] as? [String: Any] ?? [:]
    var informationExtracted = params["information_extracted"] as? Double ?? 1.0

    // importInfo.information_extracted takes priority
    if let importInfo = vibeData["importInfo"] as? [String: Any],
       let importInfoExtracted = importInfo["information_extracted"] as? Double {
        informationExtracted = importInfoExtracted
    }

    return (encoding: encoding, informationExtracted: informationExtracted)
}

/// Process an array of vibe items into encodings and information_extracted lists.
public func processVibes(_ vibes: [VibeItem], model: Model) throws -> ProcessedVibes {
    var encodings: [String] = []
    var infoExtractedList: [Double] = []

    for vibe in vibes {
        switch vibe {
        case .encoded(let result):
            encodings.append(result.encoding)
            infoExtractedList.append(result.informationExtracted)
        case .filePath(let path):
            if path.hasSuffix(".naiv4vibe") {
                let data = try loadVibeFile(path)
                let (encoding, info) = try extractEncoding(data, model: model)
                encodings.append(encoding)
                infoExtractedList.append(info)
            } else {
                // Raw base64 string
                encodings.append(path)
                infoExtractedList.append(1.0)
            }
        }
    }

    return ProcessedVibes(encodings: encodings, infoExtractedList: infoExtractedList)
}
