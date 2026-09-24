import Foundation

// MARK: - Qwen Tokenizer (V5)

/// Qwen byte-level BPE tokenizer used by V5 models.
///
/// Mirrors the official site's generic BPE encoder:
/// NFC normalize → split out special tokens → split regex → byte-to-unicode → BPE merges.
/// No EOS is appended and prompt weighting syntax is NOT stripped (the site counts raw text).
public final class NovelAIQwenTokenizer: @unchecked Sendable {
    private let vocab: [String: Int]
    private let bpeRanks: [String: Int]
    private let specials: [String]
    private let splitRegex: NSRegularExpression
    private let nfc: Bool
    private let byteEncoder: [UInt8: Character]
    private var cache: [String: [Int]] = [:]
    private let cacheLock = NSLock()
    private let cacheMaxSize = 10_000

    /// Build from the decompressed `qwen35_tokenizer.def` JSON.
    public init(jsonData: Data) throws {
        guard let json = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
              let config = json["config"] as? [String: Any],
              let pattern = config["splitRegex"] as? String,
              let vocab = json["vocab"] as? [String: Int],
              let merges = json["merges"] as? [[String]] else {
            throw NovelAIError.tokenizer("Qwen tokenizer data missing vocab, merges or config.splitRegex")
        }
        self.vocab = vocab

        var ranks: [String: Int] = [:]
        ranks.reserveCapacity(merges.count)
        for (rank, pair) in merges.enumerated() where pair.count == 2 {
            ranks[pair[0] + "\u{0}" + pair[1]] = rank
        }
        self.bpeRanks = ranks

        // Longest first so that overlapping special tokens match greedily
        let specialTokens = (json["specialTokens"] as? [String]) ?? []
        self.specials = specialTokens.filter { vocab[$0] != nil }.sorted { $0.count > $1.count }

        do {
            self.splitRegex = try NSRegularExpression(pattern: pattern)
        } catch {
            throw NovelAIError.tokenizer("Invalid Qwen split regex: \(error.localizedDescription)")
        }
        self.nfc = (config["normalization"] as? String) == "NFC"
        self.byteEncoder = bytesToUnicode()
    }

    /// Split text into (segment, isSpecial) parts.
    private func splitSpecials(_ text: String) -> [(String, Bool)] {
        guard !specials.isEmpty else { return [(text, false)] }
        var parts: [(String, Bool)] = []
        var buffer = ""
        var index = text.startIndex
        outer: while index < text.endIndex {
            for special in specials where text[index...].hasPrefix(special) {
                if !buffer.isEmpty { parts.append((buffer, false)) }
                parts.append((special, true))
                buffer = ""
                index = text.index(index, offsetBy: special.count)
                continue outer
            }
            buffer.append(text[index])
            index = text.index(after: index)
        }
        if !buffer.isEmpty { parts.append((buffer, false)) }
        return parts
    }

    private func bpe(_ word: String) -> [Int] {
        cacheLock.lock()
        if let cached = cache[word] {
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        // Work on unicode scalars so grapheme clustering never merges symbols
        var symbols = word.unicodeScalars.map { String($0) }
        while symbols.count > 1 {
            var bestRank = Int.max
            var bestIndex = -1
            for i in 0..<(symbols.count - 1) {
                if let rank = bpeRanks[symbols[i] + "\u{0}" + symbols[i + 1]], rank < bestRank {
                    bestRank = rank
                    bestIndex = i
                }
            }
            if bestIndex < 0 { break }

            // Merge every occurrence of the best pair, left to right
            let left = symbols[bestIndex]
            let right = symbols[bestIndex + 1]
            var merged: [String] = []
            merged.reserveCapacity(symbols.count)
            var i = 0
            while i < symbols.count {
                if i + 1 < symbols.count && symbols[i] == left && symbols[i + 1] == right {
                    merged.append(left + right)
                    i += 2
                } else {
                    merged.append(symbols[i])
                    i += 1
                }
            }
            symbols = merged
        }

        let ids = symbols.compactMap { vocab[$0] }
        cacheLock.lock()
        if cache.count >= cacheMaxSize { cache.removeAll(keepingCapacity: true) }
        cache[word] = ids
        cacheLock.unlock()
        return ids
    }

    /// Encode text into token IDs (no EOS).
    public func encode(_ text: String) -> [Int] {
        let normalized = nfc ? text.precomposedStringWithCanonicalMapping : text
        var ids: [Int] = []
        for (part, isSpecial) in splitSpecials(normalized) {
            if isSpecial {
                if let id = vocab[part] { ids.append(id) }
                continue
            }
            let nsPart = part as NSString
            for match in splitRegex.matches(in: part, range: NSRange(location: 0, length: nsPart.length)) {
                let piece = nsPart.substring(with: match.range)
                let unicode = String(piece.utf8.compactMap { byteEncoder[$0] })
                ids.append(contentsOf: bpe(unicode))
            }
        }
        return ids
    }

    /// Token count as shown by the official site (no EOS).
    public func countTokens(_ text: String) -> Int {
        return encode(text).count
    }
}
