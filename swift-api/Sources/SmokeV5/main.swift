// V5 smoke test against the live API (small sizes to stay in the Opus free tier).
// Run with: swift run SmokeV5 [step...]
import CoreGraphics
import Foundation
import ImageIO
import NovelAIAPI

let out = "/tmp/novelai_smoke/swift_v5"
try FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
let wanted = Array(CommandLine.arguments.dropFirst())
let client = try NovelAIClient()
let prompt = "1girl, solo, red apple in hand, simple background"
let src = "\(out)/t2i.png"

func step(_ name: String, _ body: () async throws -> String) async {
    if !wanted.isEmpty && !wanted.contains(name) { return }
    let t0 = Date()
    do {
        let info = try await body()
        print(String(format: "OK   %@ (%.1fs) %@", name, Date().timeIntervalSince(t0), info))
    } catch {
        print(String(format: "FAIL %@ (%.1fs) %@", name, Date().timeIntervalSince(t0), String(describing: error).prefix(300).description))
    }
}

func anlas(_ consumed: Int?, _ remaining: Int?) -> String {
    "anlas consumed=\(consumed.map(String.init) ?? "nil") remaining=\(remaining.map(String.init) ?? "nil")"
}

func alphaRatio(_ path: String) -> String {
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { return "(unreadable)" }
    let w = image.width, h = image.height
    var pixels = [UInt8](repeating: 0, count: w * h * 4)
    guard let ctx = CGContext(data: &pixels, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                              space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return "(no ctx)" }
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
    let transparent = stride(from: 3, to: pixels.count, by: 4).filter { pixels[$0] == 0 }.count
    return String(format: "%dx%d alpha0=%.1f%%", w, h, Double(transparent) * 100 / Double(w * h))
}

func params(_ p: String, model: Model = .naiDiffusion5Full, seed: UInt32, save: String, action: GenerateAction = .generate) -> GenerateParams {
    GenerateParams(prompt: p, action: action, savePath: "\(out)/\(save)", model: model, width: 512, height: 768, steps: 23, seed: seed)
}

let mask = try createRectangularMask(width: 512, height: 768, region: MaskRegion(x: 0.25, y: 0, w: 0.5, h: 1.0 / 3.0))

await step("balance") {
    let b = try await client.getAnlasBalance()
    return "total=\(b.fixedTrainingStepsLeft + b.purchasedTrainingSteps) tier=\(b.tier) usage=\(String(describing: b.usage)) summary=\(b.usage.map { String(describing: summarizeOpusUsage($0)) } ?? "-")"
}
await step("t2i_transparent") {
    var p = params(prompt, seed: 1, save: "t2i.png")
    p.transparentBackground = true
    let r = try await client.generate(p)
    return "\(alphaRatio(src)) \(anlas(r.anlasConsumed, r.anlasRemaining))"
}
await step("t2i_webp") {
    var p = params(prompt, seed: 8, save: "unused.png")
    p.savePath = nil
    p.saveDir = "\(out)/webp_dir"
    p.transparentBackground = true
    p.imageFormat = .webp
    let r = try await client.generate(p)
    let path = r.savedPath ?? ""
    return "format=\(r.imageFormat.rawValue) file=\((path as NSString).lastPathComponent) \(alphaRatio(path)) \(anlas(r.anlasConsumed, r.anlasRemaining))"
}
await step("i2i") {
    var p = params(prompt, seed: 2, save: "i2i.png", action: .img2img)
    p.sourceImage = .filePath(src)
    p.img2imgStrength = 0.6
    p.transparentBackground = true
    let r = try await client.generate(p)
    return "\(alphaRatio("\(out)/i2i.png")) \(anlas(r.anlasConsumed, r.anlasRemaining))"
}
await step("infill_full") {
    var p = params(prompt + ", cat ears", seed: 3, save: "infill_full.png", action: .infill)
    p.sourceImage = .filePath(src)
    p.mask = .bytes(mask)
    p.maskStrength = 0.7
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("t2i_curated") {
    let r = try await client.generate(params(prompt, model: .naiDiffusion5Curated, seed: 4, save: "curated.png"))
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("infill_curated") {
    var p = params(prompt, model: .naiDiffusion5Curated, seed: 5, save: "infill_curated.png", action: .infill)
    p.sourceImage = .filePath("\(out)/curated.png")
    p.mask = .bytes(mask)
    p.maskStrength = 0.7
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("characters") {
    var p = params("2girls, standing, simple background", seed: 6, save: "characters.png")
    p.characters = [
        CharacterConfig(prompt: "girl, red hair", centerX: 0.3, centerY: 0.5),
        CharacterConfig(prompt: "girl, blue hair", centerX: 0.7, centerY: 0.5),
    ]
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("vibe_rejected") {
    var p = params(prompt, seed: 7, save: "never.png")
    p.vibes = [.filePath("vibes/input1.naiv4vibe")]
    do {
        _ = try await client.generate(p)
        return "UNEXPECTED: accepted"
    } catch {
        return "rejected locally: \(error)"
    }
}
await step("declutter_keep_bubbles") {
    let r = try await client.augmentImage(AugmentParams(reqType: .declutterKeepBubbles, image: .filePath(src), savePath: "\(out)/declutter.png"))
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
