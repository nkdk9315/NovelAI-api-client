import Foundation
import MessagePack

// Incremental reading of the streaming endpoint (`stream: "msgpack"`), so a
// caller can show the denoising previews while the image is generated.
//
// The body is `[u32 BE length][msgpack map]` frames: one `intermediate`
// frame per sampling step (JPEG preview), then `final` (or `error`). The
// whole body is still collected and handed to `parseStreamResponse`.

// MARK: - Public Types

/// One `intermediate` frame of the generation stream.
public struct GenerateProgress: Sendable, Equatable {
    /// `step_ix`: the sampling step this preview comes from
    public let step: Int
    /// `sigma`: noise level left at this step (falls toward 0)
    public let sigma: Double?
    /// Preview image (JPEG)
    public let image: Data

    public init(step: Int, sigma: Double?, image: Data) {
        self.step = step
        self.sigma = sigma
        self.image = image
    }
}

/// Receives each preview while `generate(_:onProgress:)` runs.
public typealias ProgressHandler = @Sendable (GenerateProgress) -> Void

// MARK: - Frame Scanning

/// Finds the frames of a growing body as they complete.
public struct FrameScanner: Sendable {
    /// Offset of the first frame not yet looked at
    private var next = 0

    public init() {}

    /// Length `buffer` must reach before `scan` can find another frame.
    /// A body that is not framed (ZIP, bare PNG) reads as a length far
    /// beyond what ever arrives, so it is never scanned.
    public func bytesNeeded(_ buffer: Data) -> Int {
        guard let length = frameLength(buffer, at: next) else { return next + 4 }
        return length > 0 ? next + 4 + length : Int.max
    }

    /// Previews in the frames of `buffer` that completed since the last call.
    public mutating func scan(_ buffer: Data) -> [GenerateProgress] {
        var out: [GenerateProgress] = []
        while let length = frameLength(buffer, at: next), length > 0, next + 4 + length <= buffer.count {
            let start = buffer.startIndex + next + 4
            if let preview = intermediatePreview(Data(buffer[start..<(start + length)])) {
                out.append(preview)
            }
            next += 4 + length
        }
        return out
    }
}

/// The big-endian length prefix of the frame at `offset`, once its 4 bytes have arrived.
private func frameLength(_ buffer: Data, at offset: Int) -> Int? {
    guard offset + 4 <= buffer.count else { return nil }
    let i = buffer.startIndex + offset
    return Int(buffer[i]) << 24 | Int(buffer[i + 1]) << 16 | Int(buffer[i + 2]) << 8 | Int(buffer[i + 3])
}

/// The preview in a frame, if it is an `intermediate` event with an image.
public func intermediatePreview(_ frame: Data) -> GenerateProgress? {
    guard let event = try? MessagePackDecoder().decode(MsgpackIntermediate.self, from: frame),
          event.eventType == "intermediate" else {
        return nil
    }
    return GenerateProgress(step: event.step ?? 0, sigma: event.sigma, image: event.image)
}

/// Msgpack `intermediate` event; other fields (`samp_ix`, `gen_id`) are ignored.
private struct MsgpackIntermediate: Decodable {
    let eventType: String
    let step: Int?
    let sigma: Double?
    let image: Data

    enum CodingKeys: String, CodingKey {
        case eventType = "event_type"
        case event
        case step = "step_ix"
        case sigma
        case image
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        eventType = try c.decodeIfPresent(String.self, forKey: .eventType) ?? c.decode(String.self, forKey: .event)
        step = try? c.decode(Int.self, forKey: .step)
        // The server may send float32 or float64
        sigma = (try? c.decode(Double.self, forKey: .sigma)) ?? (try? c.decode(Float.self, forKey: .sigma)).map(Double.init)
        image = try c.decode(Data.self, forKey: .image)
    }
}

// MARK: - Reading

/// Collect the whole body from `bytes`, calling `onProgress` for each preview
/// as soon as its frame has arrived.
///
/// - Parameters:
///   - bytes: The body, e.g. `URLSession.AsyncBytes`.
///   - expectedLength: `Content-Length` if known (negative if not), checked up front.
///   - onProgress: Receives the previews in order.
/// - Throws: `NovelAIError.parse` if the body exceeds `MAX_RESPONSE_SIZE`.
func readWithProgress<Bytes: AsyncSequence>(
    _ bytes: Bytes,
    expectedLength: Int64 = -1,
    onProgress: ProgressHandler
) async throws -> Data where Bytes.Element == UInt8 {
    if expectedLength > Int64(MAX_RESPONSE_SIZE) {
        throw NovelAIError.parse("Response too large: \(expectedLength) bytes (max \(MAX_RESPONSE_SIZE))")
    }
    var buffer = Data()
    if expectedLength > 0 { buffer.reserveCapacity(Int(expectedLength)) }
    var scanner = FrameScanner()
    var needed = scanner.bytesNeeded(buffer)
    for try await byte in bytes {
        buffer.append(byte)
        if buffer.count > MAX_RESPONSE_SIZE {
            throw NovelAIError.parse("Response too large: \(buffer.count) bytes (max \(MAX_RESPONSE_SIZE))")
        }
        // Only look again once the frame being waited for can be complete
        guard buffer.count >= needed else { continue }
        for preview in scanner.scan(buffer) {
            onProgress(preview)
        }
        needed = scanner.bytesNeeded(buffer)
    }
    return buffer
}
