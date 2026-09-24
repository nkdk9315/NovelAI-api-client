// V4.5 regression smoke test against the live API (small sizes to stay in the Opus free tier).
// Run with: swift run SmokeV45 [step...]
import Foundation
import NovelAIAPI

let out = "/tmp/novelai_smoke/swift"
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

func params(_ p: String, seed: UInt32, save: String, action: GenerateAction = .generate) -> GenerateParams {
    GenerateParams(prompt: p, action: action, savePath: "\(out)/\(save)", model: .naiDiffusion45Full, width: 512, height: 768, steps: 23, seed: seed)
}

await step("balance") { "\(try await client.getAnlasBalance())" }
await step("t2i") {
    let r = try await client.generate(params(prompt, seed: 1, save: "t2i.png"))
    return "seed=\(r.seed) \(anlas(r.anlasConsumed, r.anlasRemaining))"
}
await step("i2i") {
    var p = params(prompt, seed: 2, save: "i2i.png", action: .img2img)
    p.sourceImage = .filePath(src)
    p.img2imgStrength = 0.6
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("infill") {
    var p = params(prompt + ", cat ears", seed: 3, save: "infill.png", action: .infill)
    p.sourceImage = .filePath(src)
    p.mask = .bytes(try createRectangularMask(width: 512, height: 768, region: MaskRegion(x: 0.25, y: 0, w: 0.5, h: 1.0 / 3.0)))
    p.maskStrength = 0.7
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("vibe") {
    var p = params(prompt, seed: 4, save: "vibe.png")
    p.vibes = [.filePath("vibes/input1.naiv4vibe")]
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("charref") {
    var p = params(prompt, seed: 5, save: "charref.png")
    p.characterReference = CharacterReferenceConfig(image: .filePath("reference/input.jpeg"))
    let r = try await client.generate(p)
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("augment") {
    let r = try await client.augmentImage(AugmentParams(reqType: .sketch, image: .filePath(src), savePath: "\(out)/sketch.png"))
    return anlas(r.anlasConsumed, r.anlasRemaining)
}
await step("upscale") {
    let r = try await client.upscaleImage(UpscaleParams(image: .filePath(src), savePath: "\(out)/upscale.png"))
    return "\(r.outputWidth)x\(r.outputHeight) \(anlas(r.anlasConsumed, r.anlasRemaining))"
}
