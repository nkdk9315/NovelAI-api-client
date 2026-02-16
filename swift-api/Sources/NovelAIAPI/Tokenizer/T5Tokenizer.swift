import Foundation

// MARK: - PureUnigram

/// Pure Swift Unigram tokenizer implementation.
/// Implements Viterbi algorithm for optimal Unigram segmentation.
public struct PureUnigram: Sendable {
    private let vocab: [String: Double]       // piece → log score
    private let pieceToId: [String: Int]      // piece → token ID
    private let unkId: Int
    private let unkScore: Double
    private let maxPieceLength: Int

    public init(vocabEntries: [(String, Double)], unkId: Int) {
        self.unkId = unkId

        var vocabMap = [String: Double]()
        var idMap = [String: Int]()
        var maxLen = 0
        var minScore: Double = 0

        for (i, entry) in vocabEntries.enumerated() {
            let (piece, score) = entry
            vocabMap[piece] = score
            idMap[piece] = i
            let pieceLen = Array(piece.unicodeScalars).count
            if pieceLen > maxLen {
                maxLen = pieceLen
            }
            if score != 0 && score < minScore {
                minScore = score
            }
        }

        self.vocab = vocabMap
        self.pieceToId = idMap
        self.maxPieceLength = maxLen
        // SentencePiece uses min_score - kUnkPenalty (10.0)
        self.unkScore = minScore - 10
    }

    /// Look up a token string to get its ID.
    public func tokenToId(_ token: String) -> Int? {
        return pieceToId[token]
    }

    /// Encode text into token IDs using Unigram model with Viterbi algorithm.
    /// Pre-tokenization: NFKC normalize → WhitespaceSplit → Metaspace (▁ prefix)
    public func encode(_ text: String) -> [Int] {
        // 1. NFKC normalization
        let normalized = text.precomposedStringWithCompatibilityMapping

        // 2. WhitespaceSplit: split on whitespace
        let pieces = normalized.components(separatedBy: .whitespaces).filter { !$0.isEmpty }
        if pieces.isEmpty { return [] }

        // 3. Metaspace: prepend ▁ to each piece
        let metaspaced = pieces.map { "\u{2581}\($0)" }

        // 4. Viterbi on each metaspaced piece
        var ids: [Int] = []
        for piece in metaspaced {
            ids.append(contentsOf: viterbi(piece))
        }

        return ids
    }

    /// Viterbi algorithm for optimal Unigram segmentation.
    /// Uses code point iteration to correctly handle BMP-external characters.
    private func viterbi(_ text: String) -> [Int] {
        let chars = Array(text.unicodeScalars).map { String($0) }
        let len = chars.count
        if len == 0 { return [] }

        // best[i] = (score, prev) for position i (0..i processed)
        var bestScore = [Double](repeating: -.infinity, count: len + 1)
        var bestPrev = [Int](repeating: 0, count: len + 1)
        bestScore[0] = 0

        for i in 1...len {
            for l in 1...min(maxPieceLength, i) {
                let substr = chars[(i - l)..<i].joined()
                if let score = vocab[substr] {
                    let candidate = bestScore[i - l] + score
                    if candidate > bestScore[i] {
                        bestScore[i] = candidate
                        bestPrev[i] = i - l
                    }
                }
            }

            // If no vocab match found, single char fallback to unk
            if bestScore[i] == -.infinity {
                bestScore[i] = bestScore[i - 1] + unkScore
                bestPrev[i] = i - 1
            }
        }

        // Backtrack to recover pieces
        var pieces: [String] = []
        var pos = len
        while pos > 0 {
            let prev = bestPrev[pos]
            pieces.append(chars[prev..<pos].joined())
            pos = prev
        }
        pieces.reverse()

        // Convert pieces to token IDs
        return pieces.map { pieceToId[$0] ?? unkId }
    }
}

// MARK: - NovelAIT5Tokenizer

/// T5 tokenizer for NovelAI with EOS token handling and preprocessing.
public struct NovelAIT5Tokenizer: Sendable {
    private let backend: PureUnigram
    private let eosTokenId: Int

    public init(backend: PureUnigram) {
        self.backend = backend
        self.eosTokenId = backend.tokenToId("</s>") ?? 1
    }

    /// Encode text using official NovelAI T5 logic.
    /// Returns the full token array INCLUDING EOS (for model input).
    public func encode(_ text: String) -> [Int] {
        if text.isEmpty {
            return [eosTokenId]
        }

        let processed = preprocessT5(text)
        var ids = backend.encode(processed)
        ids.append(eosTokenId)
        return ids
    }

    /// Count tokens matching official NovelAI UI display.
    /// Returns token count INCLUDING EOS token.
    public func countTokens(_ text: String) -> Int {
        return encode(text).count
    }
}
