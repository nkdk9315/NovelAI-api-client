import XCTest
import MessagePack

@testable import NovelAIAPI

// MARK: - Helpers

/// Fields of a streaming-endpoint event; nil fields are left out.
private struct StreamEvent: Encodable {
    var event_type: String
    var step_ix: Int? = nil
    var sigma: Double? = nil
    var image: Data? = nil
    var message: String? = nil
}

/// One `[u32 BE length][msgpack map]` frame of the streaming endpoint.
private func frame(_ event: StreamEvent) throws -> Data {
    let payload = try MessagePackEncoder().encode(event)
    var out = withUnsafeBytes(of: UInt32(payload.count).bigEndian) { Data($0) }
    out.append(payload)
    return out
}

private func intermediate(_ step: Int, _ jpeg: String) throws -> Data {
    try frame(StreamEvent(event_type: "intermediate", step_ix: step, sigma: 14.6 / Double(step + 1), image: Data(jpeg.utf8)))
}

private func final(_ image: Data) throws -> Data {
    try frame(StreamEvent(event_type: "final", image: image))
}

/// Collects previews from a `@Sendable` handler.
private final class PreviewBox: @unchecked Sendable {
    private let lock = NSLock()
    private var items: [GenerateProgress] = []
    func append(_ p: GenerateProgress) { lock.withLock { items.append(p) } }
    var all: [GenerateProgress] { lock.withLock { items } }
}

/// The given bytes as an async sequence, like `URLSession.AsyncBytes`.
private func byteStream(_ data: Data) -> AsyncStream<UInt8> {
    AsyncStream { continuation in
        for byte in data { continuation.yield(byte) }
        continuation.finish()
    }
}

// MARK: - Frame Scanning

final class StreamTests: XCTestCase {

    func testScannerWaitsForWholeFrames() throws {
        let first = try intermediate(0, "a")
        let second = try intermediate(1, "b")
        var body = first
        body.append(second)
        body.append(try final(Data("png".utf8)))

        var scanner = FrameScanner()
        // Only part of the first frame: nothing yet
        XCTAssertTrue(scanner.scan(body.prefix(first.count - 1)).isEmpty)
        // First frame complete, second cut short
        XCTAssertEqual(scanner.scan(body.prefix(first.count + 3)).map(\.step), [0])
        // The rest: the second preview once, and the final frame is not a preview
        let got = scanner.scan(body)
        XCTAssertEqual(got.count, 1)
        XCTAssertEqual(got.first?.step, 1)
        XCTAssertEqual(got.first?.image, Data("b".utf8))
        XCTAssertNotNil(got.first?.sigma)
        XCTAssertTrue(scanner.scan(body).isEmpty)
    }

    func testBytesNeededPointsAtTheNextFrameEnd() throws {
        let first = try intermediate(0, "a")
        var scanner = FrameScanner()
        XCTAssertEqual(scanner.bytesNeeded(Data()), 4)
        XCTAssertEqual(scanner.bytesNeeded(first.prefix(4)), first.count)
        _ = scanner.scan(first)
        XCTAssertEqual(scanner.bytesNeeded(first), first.count + 4)
    }

    func testScannerIgnoresUnframedBodies() throws {
        var scanner = FrameScanner()
        XCTAssertTrue(scanner.scan(makeMinimalPNG()).isEmpty)
        var zipScanner = FrameScanner()
        XCTAssertTrue(zipScanner.scan(try makeZipWithPNG(makeMinimalPNG())).isEmpty)
    }

    func testOnlyIntermediateFramesArePreviews() throws {
        let payload = { (f: Data) in Data(f.dropFirst(4)) }
        XCTAssertNotNil(intermediatePreview(payload(try intermediate(3, "x"))))
        XCTAssertNil(intermediatePreview(payload(try final(Data("x".utf8)))))
        XCTAssertNil(intermediatePreview(payload(try frame(StreamEvent(event_type: "error", message: "boom")))))
        XCTAssertNil(intermediatePreview(Data("not msgpack".utf8)))
    }

    func testFinalImageStillParsedFromStreamedBody() throws {
        let png = makeMinimalPNG()
        var body = try intermediate(0, "jpeg")
        body.append(try final(png))
        XCTAssertEqual(try parseStreamResponse(body), png)
    }

    // MARK: - Reading

    func testReadWithProgressReportsPreviewsAndReturnsWholeBody() async throws {
        var body = try intermediate(0, "a")
        body.append(try intermediate(1, "b"))
        body.append(try final(Data("png".utf8)))

        let box = PreviewBox()
        let out = try await readWithProgress(byteStream(body), onProgress: { box.append($0) })
        XCTAssertEqual(box.all.map(\.step), [0, 1])
        XCTAssertEqual(out, body)
    }

    func testReadWithProgressRejectsOversizedContentLength() async throws {
        do {
            _ = try await readWithProgress(byteStream(Data()), expectedLength: Int64(MAX_RESPONSE_SIZE) + 1, onProgress: { _ in })
            XCTFail("Expected a size error")
        } catch NovelAIError.parse(let message) {
            XCTAssertTrue(message.contains("Response too large"))
        }
    }

    // MARK: - generate(_:onProgress:)

    override func tearDown() {
        MockURLProtocol.requestHandler = nil
        MockURLProtocol.capturedRequests = []
        super.tearDown()
    }

    func testGeneratePassesEachPreviewAndReturnsFinalImage() async throws {
        let png = makeMinimalPNG()
        var body = try intermediate(0, "jpeg-0")
        body.append(try intermediate(1, "jpeg-1"))
        body.append(try final(png))

        MockURLProtocol.requestHandler = { request in
            // Balance lookups fail: generation goes ahead without the cost check
            if request.url?.path.contains("subscription") == true {
                return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 500), Data())
            }
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: 200), body)
        }

        let client = NovelAIClient(apiKey: "test-key", session: makeMockSession())
        let box = PreviewBox()
        let result = try await client.generate(GenerateParams(prompt: "1girl", seed: 7), onProgress: { box.append($0) })

        XCTAssertEqual(result.imageData, png)
        XCTAssertEqual(box.all.map(\.step), [0, 1])
        XCTAssertEqual(box.all.map(\.image), [Data("jpeg-0".utf8), Data("jpeg-1".utf8)])
    }

    func testGenerateErrorStatusSendsNoPreviews() async throws {
        let body = try intermediate(0, "jpeg-0")
        MockURLProtocol.requestHandler = { request in
            let status = request.url?.path.contains("subscription") == true ? 500 : 400
            return (makeHTTPResponse(url: request.url!.absoluteString, statusCode: status), body)
        }

        let client = NovelAIClient(apiKey: "test-key", session: makeMockSession())
        let box = PreviewBox()
        do {
            _ = try await client.generate(GenerateParams(prompt: "1girl", seed: 7), onProgress: { box.append($0) })
            XCTFail("Expected an API error")
        } catch NovelAIError.api(let statusCode, _) {
            XCTAssertEqual(statusCode, 400)
        }
        XCTAssertTrue(box.all.isEmpty)
    }
}
