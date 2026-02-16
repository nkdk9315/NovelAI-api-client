import Foundation

// MARK: - Logger Protocol

/// Protocol for logging warnings and errors during API operations.
public protocol Logger: Sendable {
    func warn(_ message: String)
    func error(_ message: String)
}

/// Default logger that prints messages to stderr.
public struct DefaultLogger: Logger {
    public init() {}

    public func warn(_ message: String) {
        FileHandle.standardError.write(Data("[WARN] \(message)\n".utf8))
    }

    public func error(_ message: String) {
        FileHandle.standardError.write(Data("[ERROR] \(message)\n".utf8))
    }
}

// MARK: - Retry Configuration

/// Maximum number of retry attempts for retryable errors.
private let maxRetries = 3

/// Base delay in milliseconds before exponential backoff is applied.
private let baseRetryDelayMs: UInt64 = 1000

// MARK: - Retry Logic

/// Set of `URLError` codes that are considered retryable (transient network issues).
private let retryableURLErrorCodes: Set<URLError.Code> = [
    .timedOut,
    .notConnectedToInternet,
    .cannotFindHost,
    .cannotConnectToHost,
]

/// Calculates the retry delay with exponential backoff and jitter.
///
/// Formula: `baseRetryDelayMs * 2^attempt * (1 + random * 0.3)`
///
/// - Parameter attempt: The zero-based attempt index (0 for the first retry).
/// - Returns: The delay in nanoseconds.
private func retryDelay(attempt: Int) -> UInt64 {
    let base = Double(baseRetryDelayMs) * pow(2.0, Double(attempt))
    let jitter = 1.0 + Double.random(in: 0..<1) * 0.3
    let delayMs = base * jitter
    return UInt64(delayMs) * 1_000_000 // convert ms to ns
}

/// Performs an HTTP request with retry logic, exponential backoff, and an overall timeout.
///
/// This function mirrors the TypeScript `fetchWithRetry` behavior:
/// - Retries on HTTP 429 (rate limit) responses
/// - Retries on transient network errors (timeout, DNS, connection)
/// - Throws `NovelAIError.api` for non-retryable HTTP error responses
/// - Enforces a 60-second overall timeout via `DEFAULT_REQUEST_TIMEOUT_MS`
///
/// - Parameters:
///   - request: The `URLRequest` to execute.
///   - session: The `URLSession` to use (injected for testability).
///   - operationName: A descriptive name for the operation, used in log messages.
///   - logger: A `Logger` instance for recording warnings and errors.
/// - Returns: A tuple of the response `Data` and `HTTPURLResponse`.
/// - Throws: `NovelAIError.api` for HTTP errors, `CancellationError` on timeout,
///           or the underlying `URLError` if retries are exhausted.
public func fetchWithRetry(
    request: URLRequest,
    session: URLSession,
    operationName: String = "Request",
    logger: Logger = DefaultLogger()
) async throws -> (Data, HTTPURLResponse) {
    // Overall timeout using a task group
    return try await withThrowingTaskGroup(of: (Data, HTTPURLResponse).self) { group in
        // Timeout task — throws a descriptive NovelAIError instead of CancellationError
        group.addTask {
            try await Task.sleep(nanoseconds: UInt64(DEFAULT_REQUEST_TIMEOUT_MS) * 1_000_000)
            throw NovelAIError.api(
                statusCode: 0,
                message: "\(operationName) timed out after \(DEFAULT_REQUEST_TIMEOUT_MS)ms"
            )
        }

        // Retry task
        group.addTask {
            try await performWithRetry(
                request: request,
                session: session,
                operationName: operationName,
                logger: logger
            )
        }

        // Return whichever finishes first; if the timeout fires, it throws
        do {
            guard let result = try await group.next() else {
                throw NovelAIError.other("\(operationName) failed: no result")
            }
            group.cancelAll()
            return result
        } catch is CancellationError {
            throw NovelAIError.api(
                statusCode: 0,
                message: "\(operationName) timed out after \(DEFAULT_REQUEST_TIMEOUT_MS)ms"
            )
        }
    }
}

/// Internal retry loop implementation.
private func performWithRetry(
    request: URLRequest,
    session: URLSession,
    operationName: String,
    logger: Logger
) async throws -> (Data, HTTPURLResponse) {
    for attempt in 0...maxRetries {
        // Check for cancellation (e.g. from the overall timeout)
        try Task.checkCancellation()

        // --- Attempt the request ---
        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: request)
        } catch let urlError as URLError where retryableURLErrorCodes.contains(urlError.code) {
            if attempt < maxRetries {
                let delay = retryDelay(attempt: attempt)
                let delayMs = delay / 1_000_000
                logger.warn(
                    "[NovelAI] \(operationName): Network error (\(urlError.localizedDescription)). "
                    + "Retrying in \(delayMs)ms... (attempt \(attempt + 1)/\(maxRetries))"
                )
                try await Task.sleep(nanoseconds: delay)
                continue
            }
            throw urlError
        }

        guard let httpResponse = response as? HTTPURLResponse else {
            throw NovelAIError.other("\(operationName) failed: non-HTTP response")
        }

        let statusCode = httpResponse.statusCode

        // --- Success ---
        if (200...299).contains(statusCode) {
            return (data, httpResponse)
        }

        // --- Rate limited (429) - retry ---
        if statusCode == 429 {
            if attempt < maxRetries {
                let delay = retryDelay(attempt: attempt)
                let delayMs = delay / 1_000_000
                logger.warn(
                    "[NovelAI] \(operationName): Rate limited (429). "
                    + "Retrying in \(delayMs)ms... (attempt \(attempt + 1)/\(maxRetries))"
                )
                try await Task.sleep(nanoseconds: delay)
                continue
            }
            // Max retries exhausted on 429
            let body = sanitizedBody(data)
            logger.error(
                "[NovelAI] \(operationName) error after \(maxRetries) retries (\(statusCode)): \(body)"
            )
            throw NovelAIError.api(
                statusCode: statusCode,
                message: "\(operationName) failed after \(maxRetries) retries: \(statusCode)"
            )
        }

        // --- Non-retryable HTTP error ---
        let body = sanitizedBody(data)
        logger.error("[NovelAI] \(operationName) error (\(statusCode)): \(body)")
        throw NovelAIError.api(
            statusCode: statusCode,
            message: "\(operationName) failed: \(statusCode) - \(body)"
        )
    }

    // Should be unreachable, but provides a safety net
    throw NovelAIError.other("\(operationName) failed: unknown error after \(maxRetries) retries")
}

// MARK: - Helpers

/// Converts response body data to a string, truncating if it exceeds 200 characters.
private func sanitizedBody(_ data: Data) -> String {
    let text = String(data: data, encoding: .utf8) ?? "<non-UTF8 body, \(data.count) bytes>"
    if text.count > 200 {
        return String(text.prefix(200)) + "...[truncated]"
    }
    return text
}
