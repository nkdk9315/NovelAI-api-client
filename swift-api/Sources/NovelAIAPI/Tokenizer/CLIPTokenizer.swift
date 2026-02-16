import Foundation

// MARK: - Byte-to-Unicode Mapping

/// GPT-2 style bytes_to_unicode mapping.
/// Maps byte values to Unicode characters, using printable ASCII where possible
/// and mapping non-printable bytes to higher Unicode codepoints.
func bytesToUnicode() -> [UInt8: Character] {
    var mapping = [UInt8: Character]()
    var n = 0

    // Printable ASCII ranges: 33-126, 161-172, 174-255
    let ranges: [ClosedRange<Int>] = [
        33...126,   // '!' to '~'
        161...172,  // '¡' to '¬'
        174...255,  // '®' to 'ÿ'
    ]

    for range in ranges {
        for b in range {
            mapping[UInt8(b)] = Character(Unicode.Scalar(b)!)
        }
    }

    // Map remaining bytes (0-32, 127-160, 173) to codepoints starting at 256
    n = 0
    for b in 0...255 {
        if mapping[UInt8(b)] == nil {
            mapping[UInt8(b)] = Character(Unicode.Scalar(256 + n)!)
            n += 1
        }
    }

    return mapping
}

// MARK: - HTML Entity Decoding

/// Lookup table for common HTML entities.
private let htmlEntities: [String: String] = [
    "amp": "&", "lt": "<", "gt": ">", "quot": "\"", "apos": "'",
    "nbsp": "\u{00A0}", "iexcl": "\u{00A1}", "cent": "\u{00A2}",
    "pound": "\u{00A3}", "curren": "\u{00A4}", "yen": "\u{00A5}",
    "brvbar": "\u{00A6}", "sect": "\u{00A7}", "uml": "\u{00A8}",
    "copy": "\u{00A9}", "ordf": "\u{00AA}", "laquo": "\u{00AB}",
    "not": "\u{00AC}", "shy": "\u{00AD}", "reg": "\u{00AE}",
    "macr": "\u{00AF}", "deg": "\u{00B0}", "plusmn": "\u{00B1}",
    "sup2": "\u{00B2}", "sup3": "\u{00B3}", "acute": "\u{00B4}",
    "micro": "\u{00B5}", "para": "\u{00B6}", "middot": "\u{00B7}",
    "cedil": "\u{00B8}", "sup1": "\u{00B9}", "ordm": "\u{00BA}",
    "raquo": "\u{00BB}", "frac14": "\u{00BC}", "frac12": "\u{00BD}",
    "frac34": "\u{00BE}", "iquest": "\u{00BF}", "times": "\u{00D7}",
    "divide": "\u{00F7}", "mdash": "\u{2014}", "ndash": "\u{2013}",
    "lsquo": "\u{2018}", "rsquo": "\u{2019}", "ldquo": "\u{201C}",
    "rdquo": "\u{201D}", "bull": "\u{2022}", "hellip": "\u{2026}",
    "trade": "\u{2122}", "larr": "\u{2190}", "rarr": "\u{2192}",
    "hearts": "\u{2665}", "clubs": "\u{2663}", "diams": "\u{2666}",
    "spades": "\u{2660}",
]

/// Decode HTML entities in a string (similar to Python's html.unescape).
func decodeHTMLEntities(_ text: String) -> String {
    guard text.contains("&") else { return text }

    var result = ""
    var i = text.startIndex

    while i < text.endIndex {
        if text[i] == "&" {
            let rest = text[i...]
            // Try numeric entities: &#123; or &#x1F60E;
            if let match = rest.range(of: "&#[xX]?[0-9a-fA-F]+;", options: .regularExpression) {
                let entity = String(text[match])
                let inner = String(entity.dropFirst(2).dropLast()) // Remove &# and ;
                var codePoint: UInt32?
                if inner.hasPrefix("x") || inner.hasPrefix("X") {
                    codePoint = UInt32(inner.dropFirst(), radix: 16)
                } else {
                    codePoint = UInt32(inner)
                }
                if let cp = codePoint, let scalar = Unicode.Scalar(cp) {
                    result.append(Character(scalar))
                } else {
                    result.append(contentsOf: text[match])
                }
                i = match.upperBound
                continue
            }

            // Try named entities: &amp;
            if let match = rest.range(of: "&[a-zA-Z]+;", options: .regularExpression) {
                let entity = String(text[match])
                let name = String(entity.dropFirst().dropLast()) // Remove & and ;
                if let decoded = htmlEntities[name] {
                    result.append(decoded)
                } else {
                    result.append(contentsOf: text[match])
                }
                i = match.upperBound
                continue
            }

            result.append(text[i])
            i = text.index(after: i)
        } else {
            result.append(text[i])
            i = text.index(after: i)
        }
    }

    return result
}

// MARK: - BPE Cache Size

private let BPE_CACHE_MAX_SIZE = 10_000

// MARK: - CLIP Initial Vocabulary

/// Build CLIP initial vocabulary (256 byte-level tokens).
private func buildInitialVocab() -> [String] {
    let byteToUnicode = bytesToUnicode()
    return (0...255).map { String(byteToUnicode[UInt8($0)]!) }
}

private let INITIAL_VOCAB = buildInitialVocab()

// MARK: - NovelAIClipTokenizer

/// CLIP BPE tokenizer for NovelAI.
/// Marked `@unchecked Sendable` because mutable cache state is protected by `cacheLock`.
public final class NovelAIClipTokenizer: @unchecked Sendable {
    private let encoder: [String: Int]
    private let byteEncoder: [UInt8: Character]
    private let bpeRanks: [String: Int]
    private let pat: NSRegularExpression

    // LRU cache: Dictionary + order tracking, protected by cacheLock for thread safety
    private var cache: [String: String]
    private var cacheOrder: [String]
    private let cacheLock = NSLock()
    private let separator = "\u{B7}\u{1F60E}\u{B7}" // ·😎·

    public init(definitionText: String) {
        self.byteEncoder = bytesToUnicode()

        let lines = definitionText.components(separatedBy: "\n")
        // Python: lines[1:48895]
        let mergesRaw = Array(lines.dropFirst().prefix(48894))
        let merges = mergesRaw.map { $0.components(separatedBy: " ") }

        var vocabList = [String]()
        vocabList.append(contentsOf: INITIAL_VOCAB)
        for token in INITIAL_VOCAB {
            vocabList.append(token + "</w>")
        }
        for mergePair in merges {
            vocabList.append(mergePair.joined())
        }
        vocabList.append("<|startoftext|>")
        vocabList.append("<|endoftext|>")

        var enc = [String: Int]()
        for (i, token) in vocabList.enumerated() {
            enc[token] = i
        }
        self.encoder = enc

        let sep = "\u{B7}\u{1F60E}\u{B7}"
        var ranks = [String: Int]()
        for (i, pair) in merges.enumerated() {
            ranks[pair.joined(separator: sep)] = i
        }
        self.bpeRanks = ranks

        self.cache = [
            "<|startoftext|>": "<|startoftext|>",
            "<|endoftext|>": "<|endoftext|>",
        ]
        self.cacheOrder = ["<|startoftext|>", "<|endoftext|>"]

        // CLIP regex pattern — pattern is a compile-time known literal; try! is safe
        // swiftlint:disable:next force_try
        self.pat = try! NSRegularExpression(
            pattern: "<\\|startoftext\\|>|<\\|endoftext\\|>|'s|'t|'re|'ve|'m|'ll|'d|[\\p{L}]+|[\\p{N}]|[^\\s\\p{L}\\p{N}]+",
            options: []
        )
    }

    /// Encode text into token IDs.
    public func encode(_ text: String) -> [Int] {
        // Double HTML entity decode, then strip + lowercase + normalize whitespace
        var decoded = decodeHTMLEntities(decodeHTMLEntities(text))
        decoded = decoded.trimmingCharacters(in: .whitespacesAndNewlines)
        // Replace all whitespace sequences with single space
        decoded = decoded.replacingOccurrences(
            of: "\\s+",
            with: " ",
            options: .regularExpression
        ).trimmingCharacters(in: .whitespacesAndNewlines).lowercased()

        if decoded.isEmpty { return [] }

        let nsString = decoded as NSString
        let range = NSRange(location: 0, length: nsString.length)
        let matches = pat.matches(in: decoded, options: [], range: range)

        var bpeTokens: [Int] = []

        for match in matches {
            let token = nsString.substring(with: match.range)
            let tokenBytes = Array(token.utf8)

            var tokenTranslated = ""
            for b in tokenBytes {
                tokenTranslated.append(byteEncoder[b]!)
            }

            let bpeResult = bpe(tokenTranslated)
            let splitTokens = bpeResult.components(separatedBy: " ")

            for bpeToken in splitTokens {
                if let id = encoder[bpeToken] {
                    bpeTokens.append(id)
                }
            }
        }

        return bpeTokens
    }

    // MARK: - BPE Algorithm

    private func bpe(_ token: String) -> String {
        // Check cache (LRU: move to end) — lock protects concurrent access
        cacheLock.lock()
        if let cached = cache[token] {
            if let idx = cacheOrder.firstIndex(of: token) {
                cacheOrder.remove(at: idx)
                cacheOrder.append(token)
            }
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        let chars = Array(token)
        guard chars.count > 1 else {
            return token + "</w>"
        }

        // Split into individual chars, last char gets </w> suffix
        var word: [String] = chars.dropLast().map { String($0) }
        word.append(String(chars.last!) + "</w>")

        var pairs = getPairs(word)
        if pairs.isEmpty {
            return token + "</w>"
        }

        while true {
            // Find the pair with the lowest rank
            var bestPair: [String]?
            var minRank = Int.max

            for pair in pairs {
                let key = pair.joined(separator: separator)
                let rank = bpeRanks[key] ?? Int.max
                if rank < minRank {
                    minRank = rank
                    bestPair = pair
                }
            }

            guard let bigram = bestPair, bpeRanks[bigram.joined(separator: separator)] != nil else {
                break
            }

            let first = bigram[0]
            let second = bigram[1]
            var newWord: [String] = []
            var i = 0

            while i < word.count {
                // Find next occurrence of 'first'
                var j = -1
                for k in i..<word.count {
                    if word[k] == first {
                        j = k
                        break
                    }
                }

                if j == -1 {
                    newWord.append(contentsOf: word[i...])
                    break
                }

                newWord.append(contentsOf: word[i..<j])
                i = j

                if word[i] == first && i < word.count - 1 && word[i + 1] == second {
                    newWord.append(first + second)
                    i += 2
                } else {
                    newWord.append(word[i])
                    i += 1
                }
            }

            word = newWord
            if word.count == 1 {
                break
            }
            pairs = getPairs(word)
        }

        let result = word.joined(separator: " ")

        // LRU eviction — lock protects concurrent access
        cacheLock.lock()
        if cache.count >= BPE_CACHE_MAX_SIZE {
            let oldest = cacheOrder.removeFirst()
            cache.removeValue(forKey: oldest)
        }
        cache[token] = result
        cacheOrder.append(token)
        cacheLock.unlock()

        return result
    }

    private func getPairs(_ word: [String]) -> [[String]] {
        var seen = Set<String>()
        var pairs: [[String]] = []
        var prev = word[0]

        for i in 1..<word.count {
            let current = word[i]
            let pair = [prev, current]
            let key = "\(prev)\0\(current)"

            if !seen.contains(key) {
                seen.insert(key)
                pairs.append(pair)
            }
            prev = current
        }

        return pairs
    }
}
