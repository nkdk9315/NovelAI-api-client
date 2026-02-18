import Foundation
#if canImport(CoreGraphics)
import CoreGraphics
import ImageIO
#endif

// MARK: - Internal Helpers

// Static regex constants — patterns are compile-time known literals; try! is safe.
// swiftlint:disable force_try
private let dataURLPrefixRegex = try! NSRegularExpression(pattern: "^data:image/[\\w+.-]+;base64,")
private let base64OnlyRegex = try! NSRegularExpression(pattern: "^[A-Za-z0-9+/\\-_]+=*$")
private let imageExtRegex = try! NSRegularExpression(pattern: "\\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$", options: .caseInsensitive)
// swiftlint:enable force_try

private func decodeBase64Image(_ base64Str: String) throws -> Data {
    let range = NSRange(base64Str.startIndex..<base64Str.endIndex, in: base64Str)
    let stripped = dataURLPrefixRegex.stringByReplacingMatches(
        in: base64Str, options: [], range: range, withTemplate: ""
    )

    let strippedRange = NSRange(stripped.startIndex..<stripped.endIndex, in: stripped)
    if stripped.isEmpty || base64OnlyRegex.firstMatch(in: stripped, options: [], range: strippedRange) == nil {
        throw NovelAIError.image("Invalid Base64 string: contains characters outside the Base64 alphabet or is empty")
    }

    guard let data = Data(base64Encoded: stripped) else {
        throw NovelAIError.image("Failed to decode Base64 data")
    }
    return data
}

// MARK: - Public Functions

/// Validate image data size against MAX_REF_IMAGE_SIZE_MB.
public func validateImageDataSize(_ data: Data, source: String? = nil) throws {
    let sizeMB = Double(data.count) / (1024.0 * 1024.0)
    if sizeMB > Double(MAX_REF_IMAGE_SIZE_MB) {
        let suffix = source.map { ": \($0)" } ?? ""
        throw NovelAIError.imageFileSize(
            "Image file size (\(String(format: "%.2f", sizeMB)) MB) exceeds maximum allowed size (\(MAX_REF_IMAGE_SIZE_MB) MB)\(suffix)"
        )
    }
}

/// Convert an ImageInput to raw bytes.
public func getImageBuffer(_ input: ImageInput) throws -> Data {
    switch input {
    case .bytes(let data):
        return data
    case .filePath(let path):
        do {
            try validateSafePath(path)
        } catch {
            throw NovelAIError.image("Invalid file path (path traversal detected): \(path)")
        }
        let normalized = (path as NSString).resolvingSymlinksInPath
        guard let data = FileManager.default.contents(atPath: normalized) else {
            throw NovelAIError.image("Image file not found or not readable: \(path)")
        }
        return data
    case .base64(let b64):
        return try decodeBase64Image(b64)
    case .dataURL(let url):
        return try decodeBase64Image(url)
    }
}

/// Get image dimensions from image data.
/// Returns (width, height, buffer).
public func getImageDimensions(_ input: ImageInput) throws -> (width: Int, height: Int, buffer: Data) {
    let buffer = try getImageBuffer(input)

    // Validate size
    let source: String?
    if case .filePath(let path) = input {
        source = path
    } else {
        source = nil
    }
    try validateImageDataSize(buffer, source: source)

    #if canImport(CoreGraphics)
    guard let imageSource = CGImageSourceCreateWithData(buffer as CFData, nil),
          let properties = CGImageSourceCopyPropertiesAtIndex(imageSource, 0, nil) as? [CFString: Any],
          let width = properties[kCGImagePropertyPixelWidth] as? Int,
          let height = properties[kCGImagePropertyPixelHeight] as? Int else {
        throw NovelAIError.image("Could not determine image dimensions. The file may be corrupted or not a valid image.")
    }
    return (width: width, height: height, buffer: buffer)
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

/// Heuristically determine if a string looks like a file path.
public func looksLikeFilePath(_ str: String) -> Bool {
    // If it starts with data URL prefix, it's definitely not a path
    if str.hasPrefix("data:") {
        return false
    }

    // Short-circuit: long Base64-only strings are not paths
    let range = NSRange(str.startIndex..<str.endIndex, in: str)
    if base64OnlyRegex.firstMatch(in: str, options: [], range: range) != nil && str.count > 64 {
        return false
    }

    let hasImageExt = imageExtRegex.firstMatch(in: str, options: [], range: range) != nil

    // Absolute paths (Unix)
    if str.hasPrefix("/") {
        if hasImageExt { return true }
        // Has at least two path segments (e.g., /dir/file)
        let rest = String(str.dropFirst())
        if rest.contains("/") { return true }
        return false
    }

    // Windows absolute paths (e.g., C:\...)
    if str.count >= 3 {
        let bytes = Array(str.utf8)
        if bytes.count >= 3 && bytes[0].isASCIILetter && bytes[1] == UInt8(ascii: ":") &&
           (bytes[2] == UInt8(ascii: "\\") || bytes[2] == UInt8(ascii: "/")) {
            return true
        }
    }

    // Relative paths with directory separators and file extension
    if (str.contains("/") || str.contains("\\")) && hasImageExt {
        return true
    }

    // If it has a file extension, assume path
    if hasImageExt {
        return true
    }

    // Default: if it contains directory separator, try as path
    return str.contains("/") || str.contains("\\")
}

/// Convert an ImageInput to a base64 string.
public func getImageBase64(_ input: ImageInput) throws -> String {
    let buffer = try getImageBuffer(input)
    return buffer.base64EncodedString()
}

/// Resize an image to the specified dimensions and return as a base64 string.
///
/// Used for img2img to resize the source image to match the output dimensions
/// before sending to the API. Large images (exceeding ~3M pixels) cause server
/// errors, so resizing to the target width/height avoids this issue.
///
/// The image is resized using `fill` fit (stretches to exact dimensions without
/// preserving aspect ratio), matching the TypeScript implementation's `sharp`
/// `fit: 'fill'` behavior.
///
/// - Parameters:
///   - input: The source image input.
///   - targetWidth: The desired output width in pixels.
///   - targetHeight: The desired output height in pixels.
/// - Returns: Base64-encoded PNG string of the resized image.
/// - Throws: `NovelAIError.image` if the image cannot be loaded or resized.
public func resizeImageForImg2Img(_ input: ImageInput, targetWidth: Int, targetHeight: Int) throws -> String {
    #if canImport(CoreGraphics)
    let buffer = try getImageBuffer(input)

    guard let imageSource = CGImageSourceCreateWithData(buffer as CFData, nil),
          let sourceImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        throw NovelAIError.image("Failed to load source image for img2img resize")
    }

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let context = CGContext(
        data: nil,
        width: targetWidth,
        height: targetHeight,
        bitsPerComponent: 8,
        bytesPerRow: targetWidth * 4,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        throw NovelAIError.image("Failed to create graphics context for img2img resize")
    }

    context.interpolationQuality = .high
    context.draw(sourceImage, in: CGRect(x: 0, y: 0, width: targetWidth, height: targetHeight))

    guard let resizedImage = context.makeImage() else {
        throw NovelAIError.image("Failed to create resized image for img2img")
    }

    // Encode to PNG
    let mutableData = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(mutableData as CFMutableData, "public.png" as CFString, 1, nil) else {
        throw NovelAIError.image("Failed to create PNG encoder for img2img resize")
    }
    CGImageDestinationAddImage(destination, resizedImage, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw NovelAIError.image("Failed to encode resized image as PNG")
    }

    return (mutableData as Data).base64EncodedString()
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

/// Resize an image buffer to the specified dimensions and return as Data.
///
/// Used for augment dimension clamping — resizes images that exceed MAX_PIXELS
/// before sending to the API.
///
/// - Parameters:
///   - buffer: The source image data.
///   - targetWidth: The desired output width in pixels.
///   - targetHeight: The desired output height in pixels.
/// - Returns: PNG-encoded Data of the resized image.
/// - Throws: `NovelAIError.image` if the image cannot be loaded or resized.
public func resizeImageBuffer(_ buffer: Data, targetWidth: Int, targetHeight: Int) throws -> Data {
    #if canImport(CoreGraphics)
    guard let imageSource = CGImageSourceCreateWithData(buffer as CFData, nil),
          let sourceImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        throw NovelAIError.image("Failed to load source image for augment resize")
    }

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let context = CGContext(
        data: nil,
        width: targetWidth,
        height: targetHeight,
        bitsPerComponent: 8,
        bytesPerRow: targetWidth * 4,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        throw NovelAIError.image("Failed to create graphics context for augment resize")
    }

    context.interpolationQuality = .high
    context.draw(sourceImage, in: CGRect(x: 0, y: 0, width: targetWidth, height: targetHeight))

    guard let resizedImage = context.makeImage() else {
        throw NovelAIError.image("Failed to create resized image for augment")
    }

    let mutableData = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(mutableData as CFMutableData, "public.png" as CFString, 1, nil) else {
        throw NovelAIError.image("Failed to create PNG encoder for augment resize")
    }
    CGImageDestinationAddImage(destination, resizedImage, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw NovelAIError.image("Failed to encode resized image as PNG")
    }

    return mutableData as Data
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

// MARK: - Private Extensions

private extension UInt8 {
    var isASCIILetter: Bool {
        (self >= 65 && self <= 90) || (self >= 97 && self <= 122) // A-Z or a-z
    }
}
