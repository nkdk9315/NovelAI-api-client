import Foundation
import NovelAIAPI

let inputImage = "./reference/input.jpeg"
let outputDir = "./output/test/"

// 出力ディレクトリの作成
if !FileManager.default.fileExists(atPath: outputDir) {
    try FileManager.default.createDirectory(atPath: outputDir, withIntermediateDirectories: true)
}

let client = try NovelAIClient()
var results: [(name: String, success: Bool)] = []

// Test 1: Img2Img
print("\n=== Test: Img2Img ===")
do {
    let result = try await client.generate(GenerateParams(
        prompt: "1girl, beautiful, masterpiece",
        action: .img2img,
        sourceImage: .filePath(inputImage),
        img2imgStrength: 0.6,
        img2imgNoise: 0.1,
        saveDir: outputDir,
        width: 832,
        height: 1216
    ))
    print("✅ Img2Img success!")
    print("   Saved to: \(result.savedPath ?? "N/A")")
    print("   Anlas consumed: \(result.anlasConsumed ?? 0)")
    results.append((name: "Img2Img", success: true))
} catch {
    print("❌ Img2Img failed: \(error)")
    results.append((name: "Img2Img", success: false))
}

// マスクの作成 (832x1216 キャンバス上の x=116, y=208, w=600, h=800 の白い矩形)
// Sharp ライブラリの代わりに createRectangularMask を使用し、相対座標で指定
let maskData = try createRectangularMask(
    width: 832,
    height: 1216,
    region: MaskRegion(
        x: 116.0 / 832.0,
        y: 208.0 / 1216.0,
        w: 600.0 / 832.0,
        h: 800.0 / 1216.0
    )
)

// Test 2: Infill + Img2Img (Hybrid Mode) - ハイブリッドモード
print("\n=== Test: Infill + Img2Img (Hybrid Mode) ===")
do {
    let result = try await client.generate(GenerateParams(
        prompt: "1girl, beautiful dress, elegant",
        action: .infill,
        sourceImage: .filePath(inputImage),
        mask: .bytes(maskData),
        maskStrength: 0.68,
        hybridImg2imgStrength: 0.45,
        hybridImg2imgNoise: 0,
        saveDir: outputDir,
        width: 832,
        height: 1216
    ))
    print("✅ Infill + Img2Img (Hybrid) success!")
    print("   Saved to: \(result.savedPath ?? "N/A")")
    print("   Anlas consumed: \(result.anlasConsumed ?? 0)")
    results.append((name: "Infill + Img2Img (Hybrid)", success: true))
} catch {
    print("❌ Infill + Img2Img (Hybrid) failed: \(error)")
    results.append((name: "Infill + Img2Img (Hybrid)", success: false))
}

// テスト結果のサマリーを表示
print("\n=== Test Summary ===")
for result in results {
    let status = result.success ? "✅" : "❌"
    print("  \(status) \(result.name)")
}

let passed = results.filter { $0.success }.count
let total = results.count
print("\n\(passed)/\(total) tests passed")

// 失敗があれば exit(1)
if results.contains(where: { !$0.success }) {
    exit(1)
}
