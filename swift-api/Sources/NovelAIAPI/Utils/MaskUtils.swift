import Foundation
import CryptoKit
#if canImport(CoreGraphics)
import CoreGraphics
import ImageIO
#endif

// MARK: - Cache Secret Key

/// Calculate SHA256 hash of image data for cache_secret_key.
public func calculateCacheSecretKey(_ imageData: Data) -> String {
    let digest = SHA256.hash(data: imageData)
    return digest.map { String(format: "%02x", $0) }.joined()
}

// MARK: - Mask Region / Center

/// Region for rectangular mask (0.0-1.0 relative coordinates).
public struct MaskRegion: Sendable {
    public var x: Double
    public var y: Double
    public var w: Double
    public var h: Double

    public init(x: Double, y: Double, w: Double, h: Double) {
        self.x = x
        self.y = y
        self.w = w
        self.h = h
    }
}

/// Center point for circular mask (0.0-1.0 relative coordinates).
public struct MaskCenter: Sendable {
    public var x: Double
    public var y: Double

    public init(x: Double, y: Double) {
        self.x = x
        self.y = y
    }
}

// MARK: - Mask Resize

/// Resize a mask image to 1/8 of target dimensions (API specification).
public func resizeMaskImage(_ maskData: Data, targetWidth: Int, targetHeight: Int) throws -> Data {
    #if canImport(CoreGraphics)
    let maskWidth = targetWidth / 8
    let maskHeight = targetHeight / 8

    guard let imageSource = CGImageSourceCreateWithData(maskData as CFData, nil),
          let sourceImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        throw NovelAIError.image("Failed to load mask image")
    }

    let colorSpace = CGColorSpaceCreateDeviceGray()
    guard let context = CGContext(
        data: nil,
        width: maskWidth,
        height: maskHeight,
        bitsPerComponent: 8,
        bytesPerRow: maskWidth,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.none.rawValue
    ) else {
        throw NovelAIError.image("Failed to create graphics context for mask resize")
    }

    context.interpolationQuality = .high
    context.draw(sourceImage, in: CGRect(x: 0, y: 0, width: maskWidth, height: maskHeight))

    guard let resizedImage = context.makeImage() else {
        throw NovelAIError.image("Failed to create resized mask image")
    }

    return try encodeCGImageAsPNG(resizedImage)
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

// MARK: - Rectangular Mask

/// Create a rectangular mask image programmatically.
/// - Parameters:
///   - width: Original image width
///   - height: Original image height
///   - region: Mask region (0.0-1.0 relative coordinates)
/// - Returns: PNG mask data (white = change area, black = keep area)
public func createRectangularMask(width: Int, height: Int, region: MaskRegion) throws -> Data {
    guard width > 0 && height > 0 else {
        throw NovelAIError.validation("Invalid dimensions: width (\(width)) and height (\(height)) must be positive")
    }

    try validateRegionValue("x", region.x)
    try validateRegionValue("y", region.y)
    try validateRegionValue("w", region.w)
    try validateRegionValue("h", region.h)

    #if canImport(CoreGraphics)
    let maskWidth = width / 8
    let maskHeight = height / 8

    let rectX = Int(region.x * Double(maskWidth))
    let rectY = Int(region.y * Double(maskHeight))
    let rectW = Int(region.w * Double(maskWidth))
    let rectH = Int(region.h * Double(maskHeight))

    // Create grayscale canvas (all black)
    var pixels = [UInt8](repeating: 0, count: maskWidth * maskHeight)

    // Fill the specified region with white (255)
    for y in rectY..<min(rectY + rectH, maskHeight) {
        for x in rectX..<min(rectX + rectW, maskWidth) {
            pixels[y * maskWidth + x] = 255
        }
    }

    return try encodeGrayscalePixelsAsPNG(pixels, width: maskWidth, height: maskHeight)
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

// MARK: - Circular Mask

/// Create a circular mask image programmatically.
/// - Parameters:
///   - width: Original image width
///   - height: Original image height
///   - center: Center point (0.0-1.0 relative coordinates)
///   - radius: Radius (0.0-1.0, relative to width)
/// - Returns: PNG mask data
public func createCircularMask(width: Int, height: Int, center: MaskCenter, radius: Double) throws -> Data {
    guard width > 0 && height > 0 else {
        throw NovelAIError.validation("Invalid dimensions: width (\(width)) and height (\(height)) must be positive")
    }
    guard center.x >= 0.0 && center.x <= 1.0 && center.y >= 0.0 && center.y <= 1.0 else {
        throw NovelAIError.validation("Invalid center: (\(center.x), \(center.y)) (values must be between 0.0 and 1.0)")
    }
    guard radius >= 0.0 && radius <= 1.0 else {
        throw NovelAIError.validation("Invalid radius: \(radius) (must be between 0.0 and 1.0)")
    }

    #if canImport(CoreGraphics)
    let maskWidth = width / 8
    let maskHeight = height / 8

    let centerX = center.x * Double(maskWidth)
    let centerY = center.y * Double(maskHeight)
    let radiusPx = radius * Double(maskWidth)
    let radiusPxSq = radiusPx * radiusPx

    var pixels = [UInt8](repeating: 0, count: maskWidth * maskHeight)

    for y in 0..<maskHeight {
        for x in 0..<maskWidth {
            let dx = Double(x) - centerX
            let dy = Double(y) - centerY
            if dx * dx + dy * dy <= radiusPxSq {
                pixels[y * maskWidth + x] = 255
            }
        }
    }

    return try encodeGrayscalePixelsAsPNG(pixels, width: maskWidth, height: maskHeight)
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

// MARK: - Internal Helpers

private func validateRegionValue(_ name: String, _ value: Double) throws {
    guard value >= 0.0 && value <= 1.0 else {
        throw NovelAIError.validation("Invalid region.\(name): \(value) (must be between 0.0 and 1.0)")
    }
}

#if canImport(CoreGraphics)
private func encodeGrayscalePixelsAsPNG(_ pixels: [UInt8], width: Int, height: Int) throws -> Data {
    let colorSpace = CGColorSpaceCreateDeviceGray()

    var mutablePixels = pixels
    let cgImage: CGImage = try mutablePixels.withUnsafeMutableBytes { rawBuffer in
        guard let baseAddress = rawBuffer.baseAddress else {
            throw NovelAIError.image("Failed to access pixel buffer")
        }
        guard let context = CGContext(
            data: baseAddress,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: width,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.none.rawValue
        ) else {
            throw NovelAIError.image("Failed to create graphics context for mask")
        }

        guard let image = context.makeImage() else {
            throw NovelAIError.image("Failed to create mask image")
        }
        return image
    }

    return try encodeCGImageAsPNG(cgImage)
}

private func encodeCGImageAsPNG(_ image: CGImage) throws -> Data {
    let mutableData = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(
        mutableData as CFMutableData,
        "public.png" as CFString,
        1,
        nil
    ) else {
        throw NovelAIError.image("Failed to create PNG encoder")
    }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw NovelAIError.image("Failed to encode mask as PNG")
    }
    return mutableData as Data
}
#endif
