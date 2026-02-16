import Foundation

/// Preprocess text for T5 tokenizer.
///
/// Based on official NovelAI JavaScript, T5 preprocessing ONLY removes brackets and weight syntax.
/// Unlike CLIP, it does NOT decode HTML entities, normalize whitespace, or convert to lowercase.
public func preprocessT5(_ text: String) -> String {
    var result = text

    // 1. Remove brackets [] and {}
    result = result.replacingOccurrences(of: "[", with: "")
    result = result.replacingOccurrences(of: "]", with: "")
    result = result.replacingOccurrences(of: "{", with: "")
    result = result.replacingOccurrences(of: "}", with: "")

    // 2. Remove weighting syntax (e.g., "2::content::", "1.5::content::", "-1::content::")
    // Matches optional NUMBER::content:: pairs, replacing with just the content
    if let regex = try? NSRegularExpression(pattern: "(-?\\d+\\.?\\d*)?::((?:(?!::)[\\s\\S])+)(?:::)", options: []) {
        let range = NSRange(result.startIndex..<result.endIndex, in: result)
        result = regex.stringByReplacingMatches(in: result, options: [], range: range, withTemplate: "$2")
    }

    return result
}
