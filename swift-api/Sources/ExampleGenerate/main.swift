import Foundation
import NovelAIAPI

// 出力ディレクトリの作成
let outputDirs = ["output", "output/multi_character", "output/charref", "vibes"]
for dir in outputDirs {
    if !FileManager.default.fileExists(atPath: dir) {
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true, attributes: nil)
    }
}

func exampleSimpleGenerate() async {
    print("\n=== シンプル生成 ===")
    do {
        let client = try NovelAIClient()
        let result = try await client.generate(GenerateParams(
            prompt: "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
            saveDir: "output/"
        ))
        print("✓ Generated: \(result.savedPath ?? "unknown")")
        print("  Seed: \(result.seed)")
    } catch let error as NovelAIError {
        switch error {
        case .validation(let msg):
            print("❌ バリデーションエラー:")
            print("   - \(msg)")
        default:
            print("Error: \(error)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func exampleWithVibes() async {
    print("\n=== Vibe Transfer使用 ===")
    do {
        let client = try NovelAIClient()
        let vibeFiles = ["vibes/input1.naiv4vibe"]
        let validVibes = vibeFiles.filter { FileManager.default.fileExists(atPath: $0) }
        if validVibes.isEmpty {
            print("Vibeファイルが見つかりません")
            return
        }
        let characters: [CharacterConfig] = [
            CharacterConfig(prompt: "1girl, school uniform", centerX: 0.2, centerY: 0.5, negativePrompt: ""),
            CharacterConfig(prompt: "1boy, standing", centerX: 0.8, centerY: 0.5, negativePrompt: ""),
        ]
        let vibeItems: [VibeItem] = validVibes.map { .filePath($0) }
        let vibeStrengths: [Double] = Array([0.4, 0.3, 0.5, 0.2].prefix(validVibes.count))
        let result = try await client.generate(GenerateParams(
            prompt: "school classroom, sunny day, wide shot, detailed background",
            characters: characters,
            vibes: !validVibes.isEmpty ? vibeItems : nil,
            vibeStrengths: !validVibes.isEmpty ? vibeStrengths : nil,
            saveDir: "output/multi_character/",
            width: 1024,
            height: 1024
        ))
        print("✓ Generated: \(result.savedPath ?? "unknown")")
        if let remaining = result.anlasRemaining {
            print("残りアンラス: \(remaining)")
        }
        if let consumed = result.anlasConsumed {
            print("今回消費: \(consumed)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func exampleImg2img() async {
    print("\n=== Image2Image ===")
    do {
        let client = try NovelAIClient()
        let inputImage = "reference/input.jpeg"
        if !FileManager.default.fileExists(atPath: inputImage) {
            print("入力画像が見つかりません: \(inputImage)")
            return
        }
        let characters: [CharacterConfig] = [
            CharacterConfig(prompt: "1girl, school uniform", centerX: 0.2, centerY: 0.5, negativePrompt: ""),
            CharacterConfig(prompt: "1boy, standing", centerX: 0.8, centerY: 0.5, negativePrompt: ""),
        ]
        let result = try await client.generate(GenerateParams(
            prompt: "backstreet, night, neon lights, detailed background",
            action: .img2img,
            sourceImage: .filePath(inputImage),
            img2imgStrength: 0.8,
            characters: characters,
            saveDir: "output/"
        ))
        print("✓ Generated: \(result.savedPath ?? "unknown")")
        if let remaining = result.anlasRemaining {
            print("残りアンラス: \(remaining)")
        }
        if let consumed = result.anlasConsumed {
            print("今回消費: \(consumed)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func exampleImg2imgWithVibes() async {
    print("\n=== Image2Image + Vibe Transfer ===")
    do {
        let client = try NovelAIClient()
        let inputImage = "reference/input.jpeg"
        let vibeFile = "vibes/input1.naiv4vibe"
        if !FileManager.default.fileExists(atPath: inputImage) {
            print("入力画像が見つかりません: \(inputImage)")
            return
        }
        let vibes: [VibeItem]? = FileManager.default.fileExists(atPath: vibeFile) ? [.filePath(vibeFile)] : nil
        let result = try await client.generate(GenerateParams(
            prompt: "",
            action: .img2img,
            sourceImage: .filePath(inputImage),
            img2imgStrength: 0.5,
            img2imgNoise: 0,
            vibes: vibes,
            vibeStrengths: vibes != nil ? [0.7] : nil,
            saveDir: "output/",
            width: 1024,
            height: 1024
        ))
        print("✓ Generated: \(result.savedPath ?? "unknown")")
    } catch {
        print("Error: \(error)")
    }
}

func exampleMultiCharacter() async {
    // Similar to exampleWithVibes but with 2 vibes
}

func exampleEncodeVibe() async {
    print("\n=== Vibeエンコード ===")
    do {
        let client = try NovelAIClient()
        let imagePath = "reference/input.jpeg"
        if !FileManager.default.fileExists(atPath: imagePath) {
            print("参照画像が見つかりません: \(imagePath)")
            return
        }
        let resultSaved = try await client.encodeVibe(EncodeVibeParams(
            image: .filePath(imagePath),
            saveDir: "./vibes",
            saveFilename: "input1"
        ))
        print("✓ Saved: \(resultSaved.savedPath ?? "unknown")")
        if let remaining = resultSaved.anlasRemaining {
            print("残りアンラス: \(remaining)")
        }
        if let consumed = resultSaved.anlasConsumed {
            print("今回消費: \(consumed)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func exampleCharacterReference() async {
    print("\n=== キャラクター参照 ===")
    do {
        let client = try NovelAIClient()
        let referenceImage = "reference/input.jpeg"
        if !FileManager.default.fileExists(atPath: referenceImage) {
            print("参照画像が見つかりません: \(referenceImage)")
            return
        }
        let result = try await client.generate(GenerateParams(
            prompt: "school classroom, sunny day, detailed background",
            characters: [
                CharacterConfig(prompt: "1girl, standing", centerX: 0.5, centerY: 0.5, negativePrompt: "")
            ],
            characterReference: CharacterReferenceConfig(
                image: .filePath(referenceImage),
                strength: 0.9,
                fidelity: 0.9,
                mode: .characterAndStyle
            ),
            saveDir: "output/charref/"
        ))
        print("✓ Generated: \(result.savedPath ?? "unknown")")
        print("  Seed: \(result.seed)")
        if let remaining = result.anlasRemaining {
            print("  残りアンラス: \(remaining)")
        }
    } catch {
        print("Error: \(error)")
    }
}

func exampleCharacterReferenceStyles() async {
    print("\n=== キャラクター参照モード比較 ===")
    do {
        let client = try NovelAIClient()
        let referenceImage = "reference/input.jpeg"
        if !FileManager.default.fileExists(atPath: referenceImage) {
            print("参照画像が見つかりません: \(referenceImage)")
            return
        }
        let modes: [CharRefMode] = [.character, .characterAndStyle, .style]
        for mode in modes {
            print("\n--- mode: \"\(mode)\" ---")
            do {
                let characters: [CharacterConfig]? = mode == .style ? nil : [
                    CharacterConfig(prompt: "1girl, standing", centerX: 0.5, centerY: 0.5, negativePrompt: "")
                ]
                let result = try await client.generate(GenerateParams(
                    prompt: "school classroom, sunny day, detailed background",
                    characters: characters,
                    characterReference: CharacterReferenceConfig(
                        image: .filePath(referenceImage),
                        strength: 0.6,
                        fidelity: 0.8,
                        mode: mode
                    ),
                    saveDir: "output/charref/"
                ))
                print("✓ Generated: \(result.savedPath ?? "unknown")")
                print("  Seed: \(result.seed)")
                if let remaining = result.anlasRemaining {
                    print("  残りアンラス: \(remaining)")
                }
            } catch {
                print("Error: \(error)")
            }
        }
    } catch {
        print("Error: \(error)")
    }
}

// MARK: - Main

print(String(repeating: "=", count: 50))
print("NovelAI Unified Client 使用例 (Swift)")
print(String(repeating: "=", count: 50))

// await exampleSimpleGenerate()
// await exampleWithVibes()
// await exampleImg2img()
await exampleImg2imgWithVibes()
// await exampleMultiCharacter()
// await exampleEncodeVibe()
await exampleCharacterReference()
// await exampleCharacterReferenceStyles()

print("\n使用したい例のコード内のコメントを外して実行してください。")
