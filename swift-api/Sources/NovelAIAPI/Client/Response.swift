import Foundation
import ZIPFoundation
import MessagePack

// MARK: - Constants

/// Maximum number of entries allowed in a ZIP response (ZIP bomb protection).
private let maxZipEntries = MAX_ZIP_ENTRIES

/// Maximum decompressed size for a single ZIP entry (50 MB).
private let maxDecompressedSize = MAX_DECOMPRESSED_IMAGE_SIZE

/// Maximum allowed compression ratio (ZIP bomb protection).
private let maxCompressionRatio = MAX_COMPRESSION_RATIO

// MARK: - PNG Signatures

/// Full 8-byte PNG file signature.
private let pngSignature: [UInt8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]

/// IEND chunk type identifier (marks the end of a PNG file).
private let iendMarker: [UInt8] = [0x49, 0x45, 0x4E, 0x44]

/// Image file extensions recognized when scanning ZIP entries.
private let imageExtensions: Set<String> = ["png", "webp", "jpg", "jpeg"]

// MARK: - ZIP Response Parsing

/// Extracts the first image file from a ZIP archive contained in `data`.
///
/// Security checks applied:
/// - Entry count must not exceed `MAX_ZIP_ENTRIES`
/// - Decompressed size must not exceed `MAX_DECOMPRESSED_IMAGE_SIZE`
/// - Compression ratio must not exceed `MAX_COMPRESSION_RATIO`
///
/// - Parameter data: Raw ZIP archive bytes.
/// - Returns: The decompressed image data.
/// - Throws: `NovelAIError.parse` if the archive is invalid, empty, or fails security checks.
func parseZipResponse(_ data: Data) throws -> Data {
    let archive: Archive
    do {
        archive = try Archive(data: data, accessMode: .read)
    } catch {
        throw NovelAIError.parse("Failed to open ZIP response: \(error.localizedDescription)")
    }

    // Collect entries and enforce entry count limit
    let entries = Array(archive)
    if entries.count > maxZipEntries {
        throw NovelAIError.parse(
            "Too many ZIP entries: \(entries.count) (max \(maxZipEntries))"
        )
    }

    for entry in entries {
        let pathLower = entry.path.lowercased()
        let ext = (pathLower as NSString).pathExtension
        guard imageExtensions.contains(ext) else {
            continue
        }

        // Check decompressed size
        let uncompressedSize = entry.uncompressedSize
        if uncompressedSize > UInt64(maxDecompressedSize) {
            throw NovelAIError.parse(
                "Decompressed image too large (\(uncompressedSize) bytes, max \(maxDecompressedSize))"
            )
        }

        // Check compression ratio (also reject when compressedSize == 0 to prevent bypass)
        let compressedSize = entry.compressedSize
        if compressedSize == 0 && uncompressedSize > 0 {
            throw NovelAIError.parse("Suspicious compression ratio detected (zero compressed size)")
        }
        if compressedSize > 0 && uncompressedSize / compressedSize > UInt64(maxCompressionRatio) {
            throw NovelAIError.parse("Suspicious compression ratio detected")
        }

        // Extract entry data
        var entryData = Data()
        do {
            _ = try archive.extract(entry, skipCRC32: false) { chunk in
                entryData.append(chunk)
            }
        } catch {
            throw NovelAIError.parse(
                "Failed to decompress ZIP entry '\(entry.path)': \(error.localizedDescription)"
            )
        }

        return entryData
    }

    throw NovelAIError.parse("No image found in response ZIP")
}

// MARK: - Stream Response Parsing

/// Parses a stream response using a fallback chain of format detectors.
///
/// Detection order:
/// 1. ZIP signature (`PK` header) -- delegates to ``parseZipResponse(_:)``
/// 2. PNG signature at start of data -- returns as-is
/// 3. Embedded PNG search (last occurrence for full-resolution image) -- extracts PNG slice to IEND
/// 4. MessagePack decoding -- extracts `data` or `image` field
///
/// The PNG byte search is prioritized over msgpack because streaming responses
/// may contain msgpack preview messages followed by a raw full-resolution PNG
/// at the end. The msgpack `data`/`image` fields hold low-resolution previews,
/// so the trailing PNG must be extracted first.
///
/// - Parameters:
///   - data: Raw response bytes.
///   - logger: Optional logger for diagnostic warnings.
/// - Returns: The extracted image data.
/// - Throws: `NovelAIError.parse` if no supported format is detected.
func parseStreamResponse(_ data: Data, logger: Logger? = nil) throws -> Data {
    guard data.count > 1 else {
        throw NovelAIError.parse("Cannot parse stream response (length: \(data.count))")
    }

    // 1. Check for ZIP signature (PK: 0x50 0x4B)
    if data[data.startIndex] == 0x50 && data[data.startIndex + 1] == 0x4B {
        return try parseZipResponse(data)
    }

    // 2. Check for PNG signature at start
    if data.count >= 8 && data.starts(with: pngSignature) {
        return data
    }

    // 3. Try parsing as length-prefixed binary frames (4-byte big-endian length + msgpack data)
    //    This handles error events from the server before searching for embedded data.
    //    We accumulate image data across frames and return the last one (highest resolution).
    do {
        let frames = parseBinaryFrames(data)
        if !frames.isEmpty {
            let decoder = MessagePackDecoder()
            var lastImageData: Data? = nil
            for frame in frames {
                // Check for error events
                if let errorEvent = try? decoder.decode(MsgpackErrorEvent.self, from: frame),
                   errorEvent.event_type == "error" {
                    let code = errorEvent.code.flatMap { Int($0) } ?? 500
                    let message = errorEvent.message ?? "Unknown server error"
                    throw NovelAIError.api(statusCode: code, message: message)
                }
                // Try to extract image data from frame (accumulate, last frame = full resolution)
                if let imageData = tryParseMsgpack(frame) {
                    lastImageData = imageData
                }
            }
            if let imageData = lastImageData {
                return imageData
            }
        }
    } catch let error as NovelAIError {
        throw error
    } catch {
        logger?.warn("[NovelAI] Binary frame parse failed: \(error.localizedDescription)")
    }

    // 4. Search for embedded PNG (last occurrence = full-resolution image)
    if let pngData = extractLastPNG(from: data) {
        logger?.warn("[NovelAI] Found embedded PNG in stream response via byte search")
        return pngData
    }

    // 5. Fallback: msgpack stream parsing (without frame handling, for backwards compatibility)
    if let msgpackData = tryParseMsgpack(data) {
        logger?.warn("[NovelAI] Extracted image from msgpack stream response")
        return msgpackData
    }

    logger?.warn("[NovelAI] msgpack parse failed, no supported format detected")
    throw NovelAIError.parse("Cannot parse stream response (length: \(data.count))")
}

// MARK: - Internal Helpers

/// Searches for the last PNG embedded in the data, extracting from the PNG signature
/// through the IEND chunk (plus its 4-byte CRC).
///
/// - Parameter data: The byte buffer to scan.
/// - Returns: The extracted PNG data, or `nil` if no PNG signature is found.
private func extractLastPNG(from data: Data) -> Data? {
    guard let pngStart = rfindSubsequence(in: data, pattern: pngSignature) else {
        return nil
    }

    let searchSlice = data[pngStart...]
    if let iendOffset = findSubsequence(in: searchSlice, pattern: iendMarker) {
        // IEND chunk: 4 bytes "IEND" + 4 bytes CRC
        let absoluteIendOffset = searchSlice.startIndex + iendOffset
        let endIndex = min(absoluteIendOffset + 8, data.endIndex)
        return Data(data[pngStart..<endIndex])
    }

    // No IEND found; return from PNG start to end of data
    return Data(data[pngStart...])
}

/// Attempts to decode the data as one or more msgpack messages, looking for
/// a map entry with key `"data"` or `"image"`.
///
/// The value may be binary data (returned as-is) or a base64-encoded string
/// (decoded before returning).
///
/// - Parameter data: Raw bytes that may contain msgpack-encoded messages.
/// - Returns: The extracted image data, or `nil` if parsing fails or no matching field is found.
private func tryParseMsgpack(_ data: Data) -> Data? {
    // DMMessagePack's MessagePackDecoder works with Codable types.
    // We define a lightweight struct to extract the fields we need.
    // We try two approaches: first decode as a single message, then as
    // a wrapper that may contain the fields at top level.

    let decoder = MessagePackDecoder()

    // Try decoding as a map with "data" field (binary)
    if let msg = try? decoder.decode(MsgpackDataBinary.self, from: data),
       !msg.data.isEmpty {
        return msg.data
    }

    // Try decoding as a map with "data" field (string, base64-encoded)
    if let msg = try? decoder.decode(MsgpackDataString.self, from: data),
       let decoded = Data(base64Encoded: msg.data) {
        return decoded
    }

    // Try decoding as a map with "image" field (binary)
    if let msg = try? decoder.decode(MsgpackImageBinary.self, from: data),
       !msg.image.isEmpty {
        return msg.image
    }

    // Try decoding as a map with "image" field (string, base64-encoded)
    if let msg = try? decoder.decode(MsgpackImageString.self, from: data),
       let decoded = Data(base64Encoded: msg.image) {
        return decoded
    }

    return nil
}

// MARK: - Binary Frame Parsing

/// Parses binary-framed stream data: each frame is a 4-byte big-endian length prefix
/// followed by the frame payload.
///
/// - Parameter data: Raw bytes that may contain length-prefixed frames.
/// - Returns: An array of frame payloads extracted from the data.
private func parseBinaryFrames(_ data: Data) -> [Data] {
    var frames: [Data] = []
    var offset = data.startIndex

    while offset + 4 <= data.endIndex {
        let frameLen = Int(data[offset]) << 24
            | Int(data[offset + 1]) << 16
            | Int(data[offset + 2]) << 8
            | Int(data[offset + 3])
        offset += 4

        guard frameLen > 0, offset + frameLen <= data.endIndex else {
            break
        }

        frames.append(Data(data[offset..<(offset + frameLen)]))
        offset += frameLen
    }

    return frames
}

// MARK: - Msgpack Helper Types

/// Msgpack error event with `event_type`, optional `message`, and optional `code` fields.
private struct MsgpackErrorEvent: Decodable {
    let event_type: String
    let message: String?
    let code: String?
}

/// Msgpack message with a binary `data` field.
private struct MsgpackDataBinary: Decodable {
    let data: Data
}

/// Msgpack message with a string `data` field (base64-encoded).
private struct MsgpackDataString: Decodable {
    let data: String
}

/// Msgpack message with a binary `image` field.
private struct MsgpackImageBinary: Decodable {
    let image: Data
}

/// Msgpack message with a string `image` field (base64-encoded).
private struct MsgpackImageString: Decodable {
    let image: String
}

// MARK: - Byte Pattern Search

/// Finds the first occurrence of `pattern` in `data`.
///
/// - Parameters:
///   - data: The data (or data slice) to search.
///   - pattern: The byte pattern to find.
/// - Returns: The offset from `data.startIndex` where the pattern begins, or `nil`.
private func findSubsequence(in data: Data, pattern: [UInt8]) -> Int? {
    let patternCount = pattern.count
    guard data.count >= patternCount else { return nil }

    let searchEnd = data.count - patternCount
    for offset in 0...searchEnd {
        let startIdx = data.startIndex + offset
        let endIdx = startIdx + patternCount
        if data[startIdx..<endIdx].elementsEqual(pattern) {
            return offset
        }
    }
    return nil
}

/// Finds the last occurrence of `pattern` in `data`.
///
/// - Parameters:
///   - data: The data to search.
///   - pattern: The byte pattern to find.
/// - Returns: The absolute `Data.Index` where the pattern begins, or `nil`.
private func rfindSubsequence(in data: Data, pattern: [UInt8]) -> Data.Index? {
    let patternCount = pattern.count
    guard data.count >= patternCount else { return nil }

    var offset = data.count - patternCount
    while offset >= 0 {
        let startIdx = data.startIndex + offset
        let endIdx = startIdx + patternCount
        if data[startIdx..<endIdx].elementsEqual(pattern) {
            return startIdx
        }
        offset -= 1
    }
    return nil
}
