import Foundation
#if canImport(CoreGraphics)
import CoreGraphics
import ImageIO
#endif

// MARK: - Director Reference Description

/// A single director reference description entry for the API payload.
///
/// Corresponds to each element of `director_reference_descriptions` in the API request.
public struct DirectorReferenceDescription: Sendable {
    public var baseCaption: String
    public var legacyUc: Bool

    public init(baseCaption: String, legacyUc: Bool = false) {
        self.baseCaption = baseCaption
        self.legacyUc = legacyUc
    }

    /// Convert to the dictionary format expected by the API payload.
    func toDictionary() -> [String: Any] {
        [
            "caption": [
                "base_caption": baseCaption,
                "char_captions": [] as [Any],
            ] as [String: Any],
            "legacy_uc": legacyUc,
        ]
    }
}

// MARK: - Processed Character References

/// Result of processing character references for API payload construction.
public struct ProcessedCharacterReferences: Sendable {
    public var images: [String]
    public var descriptions: [DirectorReferenceDescription]
    public var infoExtracted: [Double]
    public var strengthValues: [Double]
    public var secondaryStrengthValues: [Double]

    public init(
        images: [String] = [],
        descriptions: [DirectorReferenceDescription] = [],
        infoExtracted: [Double] = [],
        strengthValues: [Double] = [],
        secondaryStrengthValues: [Double] = []
    ) {
        self.images = images
        self.descriptions = descriptions
        self.infoExtracted = infoExtracted
        self.strengthValues = strengthValues
        self.secondaryStrengthValues = secondaryStrengthValues
    }
}

// MARK: - Public Functions

/// Prepare a character reference image by resizing and padding to the appropriate size.
///
/// Selects target size based on aspect ratio:
/// - Portrait (< 0.8): 1024x1536
/// - Landscape (> 1.25): 1536x1024
/// - Square (0.8-1.25): 1472x1472
///
/// The image is resized to fit within the target dimensions while maintaining
/// aspect ratio, then centered on a black canvas.
public func prepareCharacterReferenceImage(_ imageBuffer: Data) throws -> Data {
    #if canImport(CoreGraphics)
    guard let imageSource = CGImageSourceCreateWithData(imageBuffer as CFData, nil),
          let sourceImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil),
          let properties = CGImageSourceCopyPropertiesAtIndex(imageSource, 0, nil) as? [CFString: Any],
          let origWidth = properties[kCGImagePropertyPixelWidth] as? Int,
          let origHeight = properties[kCGImagePropertyPixelHeight] as? Int else {
        throw NovelAIError.image("Could not get image dimensions")
    }

    guard origWidth > 0 && origHeight > 0 else {
        throw NovelAIError.image("Could not get image dimensions")
    }

    let aspectRatio = Double(origWidth) / Double(origHeight)

    let targetWidth: Int
    let targetHeight: Int

    // Select target canvas based on aspect ratio:
    //   < 0.8  → portrait (tall)
    //   > 1.25 → landscape (wide)
    //   0.8–1.25 → near-square
    // Thresholds match the NovelAI web UI character reference behavior.
    if aspectRatio < CHARREF_PORTRAIT_THRESHOLD {
        targetWidth = CHARREF_PORTRAIT_SIZE.width
        targetHeight = CHARREF_PORTRAIT_SIZE.height
    } else if aspectRatio > CHARREF_LANDSCAPE_THRESHOLD {
        targetWidth = CHARREF_LANDSCAPE_SIZE.width
        targetHeight = CHARREF_LANDSCAPE_SIZE.height
    } else {
        targetWidth = CHARREF_SQUARE_SIZE.width
        targetHeight = CHARREF_SQUARE_SIZE.height
    }

    // Calculate contain-fit dimensions
    let scaleX = Double(targetWidth) / Double(origWidth)
    let scaleY = Double(targetHeight) / Double(origHeight)
    let scale = min(scaleX, scaleY)

    let resizedWidth = Int(Double(origWidth) * scale)
    let resizedHeight = Int(Double(origHeight) * scale)

    // Create RGBA black canvas
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
        throw NovelAIError.image("Failed to create graphics context for character reference")
    }

    // Fill with black
    context.setFillColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: targetWidth, height: targetHeight))

    // Center the resized image
    let offsetX = (targetWidth - resizedWidth) / 2
    let offsetY = (targetHeight - resizedHeight) / 2

    context.interpolationQuality = .high
    context.draw(sourceImage, in: CGRect(x: offsetX, y: offsetY, width: resizedWidth, height: resizedHeight))

    guard let resultImage = context.makeImage() else {
        throw NovelAIError.image("Failed to create character reference image")
    }

    // Encode to PNG
    let mutableData = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(mutableData as CFMutableData, "public.png" as CFString, 1, nil) else {
        throw NovelAIError.image("Failed to create PNG encoder")
    }
    CGImageDestinationAddImage(destination, resultImage, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw NovelAIError.image("Failed to encode image as PNG")
    }

    return mutableData as Data
    #else
    throw NovelAIError.image("CoreGraphics is not available on this platform")
    #endif
}

/// Process an array of character reference configs into payload-ready data.
public func processCharacterReferences(_ refs: [CharacterReferenceConfig]) throws -> ProcessedCharacterReferences {
    var images: [String] = []
    var descriptions: [DirectorReferenceDescription] = []
    var infoExtracted: [Double] = []
    var strengthValues: [Double] = []
    var secondaryStrengthValues: [Double] = []

    for ref in refs {
        let imageBuffer = try getImageBuffer(ref.image)
        let processedBuffer = try prepareCharacterReferenceImage(imageBuffer)
        let b64Image = processedBuffer.base64EncodedString()

        images.append(b64Image)

        descriptions.append(DirectorReferenceDescription(baseCaption: ref.mode.rawValue))

        infoExtracted.append(1.0)
        strengthValues.append(ref.strength)
        secondaryStrengthValues.append(1.0 - ref.fidelity)
    }

    return ProcessedCharacterReferences(
        images: images,
        descriptions: descriptions,
        infoExtracted: infoExtracted,
        strengthValues: strengthValues,
        secondaryStrengthValues: secondaryStrengthValues
    )
}
