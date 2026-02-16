import Foundation
import NovelAIAPI

let outputDir = "output/augment"

let client = try NovelAIClient()

let balance = try await client.getAnlasBalance()
let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
print("\n📊 現在のアンラス残高: \(total)")
print("   (固定: \(balance.fixedTrainingStepsLeft), 購入済み: \(balance.purchasedTrainingSteps))\n")

// 1. colorize
print("🎨 カラー化テスト...")
do {
    let colorizeResult = try await client.augmentImage(AugmentParams(
        reqType: .colorize,
        image: .filePath("reference/input.jpeg"),
        prompt: "vibrant colors, detailed shading",
        defry: 3,
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(colorizeResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(colorizeResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 2. emotion
print("\n😊 表情変換テスト...")
do {
    let emotionResult = try await client.augmentImage(AugmentParams(
        reqType: .emotion,
        image: .filePath("reference/input.jpeg"),
        prompt: "happy",
        defry: 0,
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(emotionResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(emotionResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 3. sketch
print("\n✏️ スケッチ化テスト...")
do {
    let sketchResult = try await client.augmentImage(AugmentParams(
        reqType: .sketch,
        image: .filePath("reference/input.jpeg"),
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(sketchResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(sketchResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 4. lineart
print("\n📝 線画抽出テスト...")
do {
    let lineartResult = try await client.augmentImage(AugmentParams(
        reqType: .lineart,
        image: .filePath("reference/input.jpeg"),
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(lineartResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(lineartResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 5. declutter
print("\n🧹 デクラッターテスト...")
do {
    let declutterResult = try await client.augmentImage(AugmentParams(
        reqType: .declutter,
        image: .filePath("reference/input.jpeg"),
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(declutterResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(declutterResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 6. bg-removal
print("\n🖼️ 背景除去テスト（⚠️ 常にアンラス消費）...")
do {
    let bgRemovalResult = try await client.augmentImage(AugmentParams(
        reqType: .bgRemoval,
        image: .filePath("reference/input.jpeg"),
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(bgRemovalResult.savedPath ?? "N/A")")
    print("   💰 消費アンラス: \(bgRemovalResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

// 7. upscale
print("\n🔍 アップスケールテスト（⚠️ 常にアンラス消費）...")
do {
    let upscaleResult = try await client.upscaleImage(UpscaleParams(
        image: .filePath("reference/input.jpeg"),
        scale: 4,
        saveDir: outputDir
    ))
    print("   ✅ 保存先: \(upscaleResult.savedPath ?? "N/A")")
    print("   📐 出力サイズ: \(upscaleResult.outputWidth)x\(upscaleResult.outputHeight)")
    print("   💰 消費アンラス: \(upscaleResult.anlasConsumed.map(String.init) ?? "N/A")")
} catch {
    print("   ❌ エラー: \(error)")
}

let finalBalance = try await client.getAnlasBalance()
let finalTotal = finalBalance.fixedTrainingStepsLeft + finalBalance.purchasedTrainingSteps
print("\n📊 最終アンラス残高: \(finalTotal)")
print("   総消費: \(total - finalTotal)\n")
