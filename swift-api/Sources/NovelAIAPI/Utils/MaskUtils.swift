import Foundation
#if canImport(CoreGraphics)
import CoreGraphics
import ImageIO
#endif

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

/// Size of one mask cell in pixels (the model works on an 8x-downscaled latent).
public let MASK_CELL = 8

/// Normalize a mask for the API: full target size, binary, snapped to 8px cells.
///
/// The mask is first reduced to one value per 8x8 cell (area average) and
/// thresholded at 50%, then scaled back up with nearest-neighbour to exactly
/// `targetWidth` x `targetHeight`. Any input size works (a 1/8 cell grid or a
/// full-size brush mask).
///
/// Why: the official site sends the full-size mask. A 1/8-size mask, or a
/// full-size mask whose edges fall between cells (soft/antialiased or
/// unaligned), makes V5 inpainting draw a grey frame along the mask border.
public func resizeMaskImage(_ maskData: Data, targetWidth: Int, targetHeight: Int) throws -> Data {
    #if canImport(CoreGraphics)
    guard targetWidth > 0 && targetHeight > 0 else {
        throw NovelAIError.validation("Invalid dimensions: width (\(targetWidth)) and height (\(targetHeight)) must be positive")
    }
    let cols = max(1, targetWidth / MASK_CELL)
    let rows = max(1, targetHeight / MASK_CELL)
    let gridWidth = cols * MASK_CELL
    let gridHeight = rows * MASK_CELL

    guard let imageSource = CGImageSourceCreateWithData(maskData as CFData, nil),
          let sourceImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        throw NovelAIError.image("Failed to load mask image")
    }

    // Draw the mask as grayscale on a grid of whole 8px cells
    var grid = [UInt8](repeating: 0, count: gridWidth * gridHeight)
    try grid.withUnsafeMutableBytes { rawBuffer in
        guard let context = CGContext(
            data: rawBuffer.baseAddress,
            width: gridWidth,
            height: gridHeight,
            bitsPerComponent: 8,
            bytesPerRow: gridWidth,
            space: CGColorSpaceCreateDeviceGray(),
            bitmapInfo: CGImageAlphaInfo.none.rawValue
        ) else {
            throw NovelAIError.image("Failed to create graphics context for mask resize")
        }
        context.interpolationQuality = .high
        context.draw(sourceImage, in: CGRect(x: 0, y: 0, width: gridWidth, height: gridHeight))
    }

    // Area-average each cell, then threshold at 50%
    var cells = [UInt8](repeating: 0, count: cols * rows)
    let cellArea = MASK_CELL * MASK_CELL
    for cy in 0..<rows {
        for cx in 0..<cols {
            var sum = 0
            for y in (cy * MASK_CELL)..<((cy + 1) * MASK_CELL) {
                let rowOffset = y * gridWidth
                for x in (cx * MASK_CELL)..<((cx + 1) * MASK_CELL) {
                    sum += Int(grid[rowOffset + x])
                }
            }
            cells[cy * cols + cx] = sum / cellArea >= 128 ? 255 : 0
        }
    }

    // Nearest-neighbour upscale back to the full target size
    var pixels = [UInt8](repeating: 0, count: targetWidth * targetHeight)
    for y in 0..<targetHeight {
        let cy = min(rows - 1, y * rows / targetHeight)
        for x in 0..<targetWidth {
            let cx = min(cols - 1, x * cols / targetWidth)
            pixels[y * targetWidth + x] = cells[cy * cols + cx]
        }
    }

    return try encodeGrayscalePixelsAsPNG(pixels, width: targetWidth, height: targetHeight)
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
