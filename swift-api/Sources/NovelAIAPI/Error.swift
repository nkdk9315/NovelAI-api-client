import Foundation

/// NovelAI API error types
public enum NovelAIError: Error, LocalizedError {
    /// Schema/parameter validation error
    case validation(String)
    /// Numeric range error
    case range(String)
    /// Image processing error
    case image(String)
    /// Image file size exceeded
    case imageFileSize(String)
    /// Tokenizer initialization/loading error
    case tokenizer(String)
    /// Token count validation error
    case tokenValidation(String)
    /// API request/response error
    case api(statusCode: Int, message: String)
    /// Response parsing error
    case parse(String)
    /// File I/O error
    case io(String)
    /// Other/unexpected error
    case other(String)

    public var errorDescription: String? {
        switch self {
        case .validation(let msg): return "Validation error: \(msg)"
        case .range(let msg): return "Range error: \(msg)"
        case .image(let msg): return "Image error: \(msg)"
        case .imageFileSize(let msg): return "Image file size error: \(msg)"
        case .tokenizer(let msg): return "Tokenizer error: \(msg)"
        case .tokenValidation(let msg): return "Token validation error: \(msg)"
        case .api(let code, let msg): return "API error (\(code)): \(msg)"
        case .parse(let msg): return "Parse error: \(msg)"
        case .io(let msg): return "I/O error: \(msg)"
        case .other(let msg): return "Error: \(msg)"
        }
    }
}
