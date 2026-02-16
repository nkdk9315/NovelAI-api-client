import Foundation
import Compression

// MARK: - Constants

private let CACHE_DIR: String = {
    let home = FileManager.default.homeDirectoryForCurrentUser.path
    return "\(home)/.cache/tokenizers"
}()

private let CACHE_TTL: TimeInterval = 7 * 24 * 60 * 60 // 7 days in seconds
private let MAX_RESPONSE_SIZE_TOKENIZER = 50 * 1024 * 1024 // 50MB

// MARK: - TokenizerCacheManager

/// Thread-safe tokenizer cache manager using Swift actor.
/// Note: Actor serialization means concurrent callers queue for tokenizer initialization.
/// This is intentional — tokenizer creation is a one-time cost, and subsequent calls
/// return the cached instance without contention.
public actor TokenizerCacheManager {
    public static let shared = TokenizerCacheManager()

    private var clipTokenizer: NovelAIClipTokenizer?
    private var t5Tokenizer: NovelAIT5Tokenizer?

    private init() {}

    /// Get or create CLIP tokenizer (cached).
    public func getClipTokenizer(forceRefresh: Bool = false) async throws -> NovelAIClipTokenizer {
        if let cached = clipTokenizer, !forceRefresh {
            return cached
        }

        let tokenUrl = "https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true"
        let dataStr = try await fetchData(targetUrl: tokenUrl, forceRefresh: forceRefresh)

        guard let jsonData = dataStr.data(using: .utf8),
              let json = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
              let text = json["text"] as? String else {
            throw NovelAIError.tokenizer("CLIP tokenizer data missing \"text\" field or failed to parse as JSON")
        }

        let tokenizer = NovelAIClipTokenizer(definitionText: text)
        self.clipTokenizer = tokenizer
        return tokenizer
    }

    /// Get or create T5 tokenizer (cached).
    public func getT5Tokenizer(forceRefresh: Bool = false) async throws -> NovelAIT5Tokenizer {
        if let cached = t5Tokenizer, !forceRefresh {
            return cached
        }

        let tokenUrl = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true"
        let dataStr = try await fetchData(targetUrl: tokenUrl, forceRefresh: forceRefresh)

        guard let jsonData = dataStr.data(using: .utf8),
              let json = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
              let model = json["model"] as? [String: Any],
              let vocabArray = model["vocab"] as? [[Any]],
              let unkId = model["unk_id"] as? Int else {
            throw NovelAIError.tokenizer("T5 tokenizer data missing or invalid structure")
        }

        var vocabEntries: [(String, Double)] = []
        for entry in vocabArray {
            guard entry.count >= 2,
                  let piece = entry[0] as? String,
                  let score = entry[1] as? Double else {
                continue
            }
            vocabEntries.append((piece, score))
        }

        let unigram = PureUnigram(vocabEntries: vocabEntries, unkId: unkId)
        let tokenizer = NovelAIT5Tokenizer(backend: unigram)
        self.t5Tokenizer = tokenizer
        return tokenizer
    }

    /// Validate that the token count does not exceed MAX_TOKENS.
    public func validateTokenCount(_ text: String) async throws -> Int {
        let tokenizer = try await getT5Tokenizer()
        let tokenCount = tokenizer.countTokens(text)

        if tokenCount > MAX_TOKENS {
            throw NovelAIError.tokenValidation(
                "Token count (\(tokenCount)) exceeds maximum allowed (\(MAX_TOKENS))"
            )
        }

        return tokenCount
    }

    /// Clear cached tokenizers.
    public func clearCache() {
        clipTokenizer = nil
        t5Tokenizer = nil
    }
}

// MARK: - Cache File Helpers

/// Sanitize a string to only allow safe filename characters.
private func sanitizeFilenameComponent(_ s: String) -> String {
    return s.replacingOccurrences(of: "[^a-zA-Z0-9._-]", with: "", options: .regularExpression)
}

/// Generate a cache filename from a URL.
func getCacheFilename(_ urlString: String) throws -> String {
    guard let url = URL(string: urlString) else {
        throw NovelAIError.tokenizer("Invalid tokenizer URL: \(urlString)")
    }

    let pathname = url.path
    let lastComponent = (pathname as NSString).lastPathComponent
    // Manually strip .def extension (NSString.deletingPathExtension won't strip from ".def" hidden files)
    let rawBasename: String
    if lastComponent.hasSuffix(".def") {
        rawBasename = String(lastComponent.dropLast(4))
    } else {
        rawBasename = (lastComponent as NSString).deletingPathExtension
    }
    let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
    let rawVersion = components?.queryItems?.first(where: { $0.name == "v" })?.value ?? "unknown"

    let basename = sanitizeFilenameComponent(rawBasename)
    let version = sanitizeFilenameComponent(rawVersion)

    if basename.isEmpty {
        throw NovelAIError.tokenizer("Invalid tokenizer URL: empty basename after sanitization")
    }

    return "\(basename)_v\(version).json"
}

/// Validate that a resolved cache path is within CACHE_DIR.
private func validateCachePath(_ cachePath: String) throws {
    let resolved = (cachePath as NSString).standardizingPath
    let cacheDir = (CACHE_DIR as NSString).standardizingPath
    if !resolved.hasPrefix(cacheDir + "/") && resolved != cacheDir {
        throw NovelAIError.tokenizer("Cache path traversal detected: \(cachePath)")
    }
}

/// Read from disk cache. Returns nil if cache doesn't exist or is expired.
private func readFromCache(_ cacheFile: String) -> String? {
    let cachePath = (CACHE_DIR as NSString).appendingPathComponent(cacheFile)
    do {
        try validateCachePath(cachePath)
    } catch {
        return nil
    }

    let fileManager = FileManager.default
    guard fileManager.fileExists(atPath: cachePath) else { return nil }

    guard let attrs = try? fileManager.attributesOfItem(atPath: cachePath),
          let modDate = attrs[.modificationDate] as? Date else {
        return nil
    }

    // Check TTL
    if Date().timeIntervalSince(modDate) > CACHE_TTL {
        return nil
    }

    return try? String(contentsOfFile: cachePath, encoding: .utf8)
}

/// Write data to cache file.
private func writeToCache(_ cacheFile: String, data: String) {
    let cachePath = (CACHE_DIR as NSString).appendingPathComponent(cacheFile)
    do {
        try validateCachePath(cachePath)
        try FileManager.default.createDirectory(atPath: CACHE_DIR, withIntermediateDirectories: true)
        try data.write(toFile: cachePath, atomically: true, encoding: .utf8)
    } catch {
        // Cache write failure is not fatal
    }
}

// MARK: - Network Fetch & Decompression

/// Fetch and decompress tokenizer data from a URL.
/// Uses disk cache to avoid repeated network requests.
private func fetchData(targetUrl: String, forceRefresh: Bool = false) async throws -> String {
    let cacheFile = try getCacheFilename(targetUrl)

    // Try cache first
    if !forceRefresh, let cached = readFromCache(cacheFile) {
        return cached
    }

    guard let url = URL(string: targetUrl) else {
        throw NovelAIError.tokenizer("Invalid tokenizer URL: \(targetUrl)")
    }

    var request = URLRequest(url: url)
    request.timeoutInterval = 30
    request.setValue("novelai-swift-api/1.0", forHTTPHeaderField: "User-Agent")

    let (data, response): (Data, URLResponse)
    do {
        (data, response) = try await URLSession.shared.data(for: request)
    } catch {
        throw NovelAIError.tokenizer("Network error while fetching tokenizer: \(error.localizedDescription)")
    }

    if let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode != 200 {
        throw NovelAIError.tokenizer("Failed to fetch tokenizer: HTTP \(httpResponse.statusCode)")
    }

    // Decompress: try raw deflate first, then zlib
    let decompressed: Data
    if let rawResult = try? decompressRawDeflate(data) {
        decompressed = rawResult
    } else if let zlibResult = try? decompressZlib(data) {
        decompressed = zlibResult
    } else {
        throw NovelAIError.tokenizer("Failed to decompress tokenizer data")
    }

    guard let dataStr = String(data: decompressed, encoding: .utf8) else {
        throw NovelAIError.tokenizer("Failed to convert decompressed data to UTF-8 string")
    }

    // Save to cache
    writeToCache(cacheFile, data: dataStr)

    return dataStr
}

// MARK: - Decompression

/// Decompress raw deflate data using Compression framework.
private func decompressRawDeflate(_ data: Data) throws -> Data {
    return try decompress(data, algorithm: COMPRESSION_ZLIB)
}

/// Decompress zlib-wrapped data.
/// Validates the 2-byte zlib header using the checksum rule: (CMF * 256 + FLG) % 31 == 0.
private func decompressZlib(_ data: Data) throws -> Data {
    if data.count > 2 {
        let cmf = UInt16(data[0])
        let flg = UInt16(data[1])
        let isZlibHeader = cmf == 0x78 && (cmf * 256 + flg) % 31 == 0
        if isZlibHeader {
            return try decompress(data.dropFirst(2), algorithm: COMPRESSION_ZLIB)
        }
    }
    return try decompress(data, algorithm: COMPRESSION_ZLIB)
}

private func decompress(_ data: Data, algorithm: compression_algorithm) throws -> Data {
    let bufferSize = MAX_RESPONSE_SIZE_TOKENIZER
    let destinationBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
    defer { destinationBuffer.deallocate() }

    let decompressedSize = data.withUnsafeBytes { (sourcePtr: UnsafeRawBufferPointer) -> Int in
        guard let sourceAddress = sourcePtr.baseAddress else { return 0 }
        return compression_decode_buffer(
            destinationBuffer,
            bufferSize,
            sourceAddress.assumingMemoryBound(to: UInt8.self),
            data.count,
            nil,
            algorithm
        )
    }

    guard decompressedSize > 0 else {
        throw NovelAIError.tokenizer("Decompression failed or produced empty output")
    }

    return Data(bytes: destinationBuffer, count: decompressedSize)
}

// MARK: - Convenience Functions

/// Get or create CLIP tokenizer (convenience wrapper).
public func getClipTokenizer(forceRefresh: Bool = false) async throws -> NovelAIClipTokenizer {
    return try await TokenizerCacheManager.shared.getClipTokenizer(forceRefresh: forceRefresh)
}

/// Get or create T5 tokenizer (convenience wrapper).
public func getT5Tokenizer(forceRefresh: Bool = false) async throws -> NovelAIT5Tokenizer {
    return try await TokenizerCacheManager.shared.getT5Tokenizer(forceRefresh: forceRefresh)
}

/// Validate token count (convenience wrapper).
public func validateTokenCount(_ text: String) async throws -> Int {
    return try await TokenizerCacheManager.shared.validateTokenCount(text)
}

/// Clear tokenizer cache (convenience wrapper).
public func clearTokenizerCache() async {
    await TokenizerCacheManager.shared.clearCache()
}
