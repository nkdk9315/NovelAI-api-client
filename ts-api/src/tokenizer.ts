import axios from 'axios';
import * as zlib from 'zlib';
import * as fs from 'fs/promises';
import * as path from 'path';
import he from 'he';
import { MAX_TOKENS } from './constants';

// Cache directory for tokenizer data (relative to project root)
const CACHE_DIR = path.resolve(__dirname, '..', '.cache', 'tokenizers');

// Cache TTL: 7 days
const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000;

// Maximum response size: 50MB
const MAX_RESPONSE_SIZE = 50 * 1024 * 1024;

// BPE cache size limit
const BPE_CACHE_MAX_SIZE = 10_000;

// Re-export MAX_TOKENS from constants for backward compatibility
export { MAX_TOKENS };

// Custom error class for tokenizer-related errors
export class TokenizerError extends Error {
    constructor(message: string, public readonly cause?: unknown) {
        super(message);
        this.name = 'TokenizerError';
    }
}

// Custom error class for token validation errors
export class TokenValidationError extends Error {
    constructor(message: string, public readonly tokenCount: number, public readonly maxTokens: number) {
        super(message);
        this.name = 'TokenValidationError';
    }
}

// Singleton cache for tokenizers (caching Promises prevents duplicate concurrent requests)
let cachedClipTokenizerPromise: Promise<NovelAIClipTokenizer> | null = null;
let cachedT5TokenizerPromise: Promise<NovelAIT5Tokenizer> | null = null;

// Dynamic import for native tokenizer (tokenizers package)
let nativeTokenizerModule: any = null;
let nativeTokenizerUnavailable = false;

async function tryLoadNativeTokenizer(): Promise<any> {
    if (nativeTokenizerUnavailable) return null;
    if (nativeTokenizerModule) return nativeTokenizerModule;
    try {
        nativeTokenizerModule = await import('tokenizers');
        return nativeTokenizerModule;
    } catch {
        nativeTokenizerUnavailable = true;
        return null;
    }
}

// The initial vocabulary list from 9423.2de67be589ffa59d.js
const INITIAL_VOCAB: string[] = [
    "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<", "=", ">", "?", "@", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "^", "_", "`", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "{", "|", "}", "~", "\xa1", "\xa2", "\xa3", "\xa4", "\xa5", "\xa6", "\xa7", "\xa8", "\xa9", "\xaa", "\xab", "\xac", "\xae", "\xaf", "\xb0", "\xb1", "\xb2", "\xb3", "\xb4", "\xb5", "\xb6", "\xb7", "\xb8", "\xb9", "\xba", "\xbb", "\xbc", "\xbd", "\xbe", "\xbf", "\xc0", "\xc1", "\xc2", "\xc3", "\xc4", "\xc5", "\xc6", "\xc7", "\xc8", "\xc9", "\xca", "\xcb", "\xcc", "\xcd", "\xce", "\xcf", "\xd0", "\xd1", "\xd2", "\xd3", "\xd4", "\xd5", "\xd6", "\xd7", "\xd8", "\xd9", "\xda", "\xdb", "\xdc", "\xdd", "\xde", "\xdf", "\xe0", "\xe1", "\xe2", "\xe3", "\xe4", "\xe5", "\xe6", "\xe7", "\xe8", "\xe9", "\xea", "\xeb", "\xec", "\xed", "\xee", "\xef", "\xf0", "\xf1", "\xf2", "\xf3", "\xf4", "\xf5", "\xf6", "\xf7", "\xf8", "\xf9", "\xfa", "\xfb", "\xfc", "\xfd", "\xfe", "\xff", "Ā", "ā", "Ă", "ă", "Ą", "ą", "Ć", "ć", "Ĉ", "ĉ", "Ċ", "ċ", "Č", "č", "Ď", "ď", "Đ", "đ", "Ē", "ē", "Ĕ", "ĕ", "Ė", "ė", "Ę", "ę", "Ě", "ě", "Ĝ", "ĝ", "Ğ", "ğ", "Ġ", "ġ", "Ģ", "ģ", "Ĥ", "ĥ", "Ħ", "ħ", "Ĩ", "ĩ", "Ī", "ī", "Ĭ", "ĭ", "Į", "į", "İ", "ı", "Ĳ", "ĳ", "Ĵ", "ĵ", "Ķ", "ķ", "ĸ", "Ĺ", "ĺ", "Ļ", "ļ", "Ľ", "ľ", "Ŀ", "ŀ", "Ł", "ł", "Ń"
];

function bytesToUnicode(): { [key: number]: string } {
    const bs: number[] = [
        ...Array.from({ length: '~'.charCodeAt(0) - '!'.charCodeAt(0) + 1 }, (_, i) => i + '!'.charCodeAt(0)),
        ...Array.from({ length: '¬'.charCodeAt(0) - '¡'.charCodeAt(0) + 1 }, (_, i) => i + '¡'.charCodeAt(0)),
        ...Array.from({ length: 'ÿ'.charCodeAt(0) - '®'.charCodeAt(0) + 1 }, (_, i) => i + '®'.charCodeAt(0))
    ];

    const cs: number[] = [...bs];
    let n = 0;

    for (let b = 0; b < 256; b++) {
        if (!bs.includes(b)) {
            bs.push(b);
            cs.push(256 + n);
            n++;
        }
    }

    const result: { [key: number]: string } = {};
    for (let i = 0; i < bs.length; i++) {
        result[bs[i]] = String.fromCharCode(cs[i]);
    }
    return result;
}

export class NovelAIClipTokenizer {
    private byteEncoder: { [key: number]: string };
    private encoder: { [key: string]: number };
    private decoder: { [key: number]: string };
    private bpeRanks: { [key: string]: number };
    private cache: Map<string, string>;
    private pat: RegExp;
    private textEncoder: TextEncoder;

    constructor(definitionText: string) {
        this.textEncoder = new TextEncoder();
        this.byteEncoder = bytesToUnicode();

        const lines = definitionText.split('\n');
        // JS slice is [start, end), Python is same. Python 1:48895.
        // lines[0] is version or something? In Python it skips line 0.
        const mergesRaw = lines.slice(1, 48895);
        const merges = mergesRaw.map(line => line.split(" "));

        const vocabList = [...INITIAL_VOCAB];
        INITIAL_VOCAB.forEach(token => vocabList.push(token + "</w>"));

        for (const mergePair of merges) {
            vocabList.push(mergePair.join(""));
        }

        vocabList.push("<|startoftext|>");
        vocabList.push("<|endoftext|>");

        this.encoder = {};
        vocabList.forEach((token, i) => {
            this.encoder[token] = i;
        });

        this.decoder = {};
        Object.entries(this.encoder).forEach(([token, i]) => {
            this.decoder[i] = token;
        });

        // "\xb7" is Middle Dot. "\U0001F60E" is Sunglasses Emoji.
        const separator = "\xb7\u{1F60E}\xb7";
        this.bpeRanks = {};
        merges.forEach((pair, i) => {
            this.bpeRanks[pair.join(separator)] = i;
        });

        this.cache = new Map<string, string>([
            ["<|startoftext|>", "<|startoftext|>"],
            ["<|endoftext|>", "<|endoftext|>"],
        ]);

        // Regex pattern from Python:
        // r"""<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+"""
        // JS equivalent with 'u' flag:
        this.pat = /<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+/gu;
    }

    private bpe(token: string): string {
        const cached = this.cache.get(token);
        if (cached !== undefined) {
            // LRU: move to end
            this.cache.delete(token);
            this.cache.set(token, cached);
            return cached;
        }

        let word = [...token.slice(0, -1)]; // split into chars
        word.push(token.slice(-1) + "</w>"); // last char + </w>

        let pairs = this.getPairs(word);
        if (pairs.length === 0) {
            return token + "</w>";
        }

        while (true) {
            const separator = "\xb7\u{1F60E}\xb7";

            let bigram: string[] | null = null;
            let minRank = Infinity;

            for (const pair of pairs) {
                const key = pair.join(separator);
                const rank = this.bpeRanks[key] ?? Infinity;
                if (rank < minRank) {
                    minRank = rank;
                    bigram = pair;
                }
            }

            if (!bigram || !(bigram.join(separator) in this.bpeRanks)) {
                break;
            }

            const [first, second] = bigram;
            const newWord: string[] = [];
            let i = 0;
            while (i < word.length) {
                let j = -1;
                // Find occurrence of 'first' starting at i
                for(let k = i; k < word.length; k++) {
                     if (word[k] === first) {
                         j = k;
                         break;
                     }
                }

                if (j === -1) {
                    newWord.push(...word.slice(i));
                    break;
                }

                newWord.push(...word.slice(i, j));
                i = j;

                if (word[i] === first && i < word.length - 1 && word[i + 1] === second) {
                    newWord.push(first + second);
                    i += 2;
                } else {
                    newWord.push(word[i]);
                    i += 1;
                }
            }

            word = newWord;
            if (word.length === 1) {
                break;
            }
            pairs = this.getPairs(word);
        }

        const result = word.join(" ");
        // LRU eviction: remove oldest entry when at capacity
        if (this.cache.size >= BPE_CACHE_MAX_SIZE) {
            const firstKey = this.cache.keys().next().value;
            if (firstKey !== undefined) {
                this.cache.delete(firstKey);
            }
        }
        this.cache.set(token, result);
        return result;
    }

    private getPairs(word: string[]): string[][] {
        const seen = new Set<string>();
        const pairs: string[][] = [];
        let prevChar = word[0];
        for (let i = 1; i < word.length; i++) {
            const char = word[i];
            const pair = [prevChar, char];
            const key = pair.join('\0');

            if (!seen.has(key)) {
                seen.add(key);
                pairs.push(pair);
            }
            prevChar = char;
        }
        return pairs;
    }

    public encode(text: string): number[] {
        // html.unescape twice in python?
        // text = html.unescape(html.unescape(text)).strip()
        // he.decode matches html.unescape
        let decoded = he.decode(he.decode(text)).trim();

        // re.sub(r'\s+', ' ', text).strip().lower()
        decoded = decoded.replace(/\s+/g, ' ').trim().toLowerCase();

        const bpeTokens: number[] = [];

        // this.pat.findall(text)
        // matchAll in JS
        const matches = decoded.match(this.pat);
        if (!matches) return [];

        for (const token of matches) {
             // token_bytes = token.encode("utf-8")
             const tokenBytes = this.textEncoder.encode(token);

             // token_translated = "".join([self.byte_encoder[b] for b in token_bytes])
             let tokenTranslated = "";
             for (const b of tokenBytes) {
                 tokenTranslated += this.byteEncoder[b];
             }

             const bpeRes = this.bpe(tokenTranslated);
             const splitTokens = bpeRes.split(" ");

             for (const bpeToken of splitTokens) {
                 if (this.encoder[bpeToken] !== undefined) {
                     bpeTokens.push(this.encoder[bpeToken]);
                 }
             }
        }

        return bpeTokens;
    }
}

/**
 * Sanitize a string to only allow safe filename characters.
 * Allows alphanumeric, dot, hyphen, underscore.
 */
function sanitizeFilenameComponent(s: string): string {
    return s.replace(/[^a-zA-Z0-9._-]/g, '');
}

/**
 * Generate a cache filename from a URL.
 * Extracts the tokenizer name and version from the URL.
 */
export function getCacheFilename(url: string): string {
    const urlObj = new URL(url);
    const pathname = urlObj.pathname; // e.g., /tokenizer/compressed/t5_tokenizer.def
    const rawBasename = path.basename(pathname, '.def'); // e.g., t5_tokenizer
    const rawVersion = urlObj.searchParams.get('v') || 'unknown';
    const basename = sanitizeFilenameComponent(rawBasename);
    const version = sanitizeFilenameComponent(rawVersion);
    if (!basename) {
        throw new TokenizerError('Invalid tokenizer URL: empty basename after sanitization');
    }
    return `${basename}_v${version}.json`;
}

/**
 * Validate that a resolved cache path is within CACHE_DIR.
 * Prevents path traversal attacks.
 */
function validateCachePath(cachePath: string): void {
    const resolved = path.resolve(cachePath);
    if (!resolved.startsWith(CACHE_DIR + path.sep) && resolved !== CACHE_DIR) {
        throw new TokenizerError(`Cache path traversal detected: ${cachePath}`);
    }
}

/**
 * Check if a cached file exists and read it.
 * Returns null if cache doesn't exist or is expired.
 */
async function readFromCache(cacheFile: string): Promise<string | null> {
    const cachePath = path.join(CACHE_DIR, cacheFile);
    validateCachePath(cachePath);
    try {
        const stat = await fs.stat(cachePath);
        if (Date.now() - stat.mtimeMs > CACHE_TTL_MS) {
            return null;
        }
        const data = await fs.readFile(cachePath, 'utf-8');
        return data;
    } catch {
        // Cache file doesn't exist or can't be read
        return null;
    }
}

/**
 * Write data to cache file.
 */
async function writeToCache(cacheFile: string, data: string): Promise<void> {
    const cachePath = path.join(CACHE_DIR, cacheFile);
    validateCachePath(cachePath);
    try {
        // Ensure cache directory exists
        await fs.mkdir(CACHE_DIR, { recursive: true });
        await fs.writeFile(cachePath, data, 'utf-8');
    } catch {
        // Cache write failure is not fatal, silently continue
    }
}

/**
 * Fetch and decompress tokenizer data from a URL.
 * Uses disk cache to avoid repeated network requests.
 */
async function fetchData(targetUrl: string, forceRefresh = false): Promise<string> {
    const cacheFile = getCacheFilename(targetUrl);
    
    // Try to load from cache first (unless forceRefresh is true)
    if (!forceRefresh) {
        const cachedData = await readFromCache(cacheFile);
        if (cachedData) {
            return cachedData;
        }
    }
    
    let response;
    try {
        response = await axios.get(targetUrl, {
            responseType: 'arraybuffer',
            headers: { "User-Agent": "novelai-ts-api/1.0" },
            timeout: 30000, // 30 second timeout
            maxContentLength: MAX_RESPONSE_SIZE,
            maxBodyLength: MAX_RESPONSE_SIZE,
        });
    } catch (error) {
        if (axios.isAxiosError(error)) {
            if (error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND') {
                throw new TokenizerError(`Failed to connect to tokenizer server: ${targetUrl}`, error);
            }
            if (error.code === 'ETIMEDOUT' || error.code === 'ECONNABORTED') {
                throw new TokenizerError(`Request timed out while fetching tokenizer data`, error);
            }
            throw new TokenizerError(`Network error while fetching tokenizer: ${error.message}`, error);
        }
        throw new TokenizerError(`Unexpected error while fetching tokenizer`, error);
    }

    const buffer = Buffer.from(response.data);
    let data: Buffer;

    try {
        // Try raw deflate first (most common for this API)
        data = zlib.inflateRawSync(buffer);
    } catch (rawError) {
        try {
            data = zlib.inflateSync(buffer);
        } catch (standardError) {
            throw new TokenizerError(
                `Failed to decompress tokenizer data. Raw inflate error: ${rawError}. Standard inflate error: ${standardError}`,
                { rawError, standardError }
            );
        }
    }

    const dataStr = data.toString("utf-8");
    
    // Save to cache for future use
    await writeToCache(cacheFile, dataStr);
    
    return dataStr;
}

export async function getClipTokenizer(forceRefresh = false): Promise<NovelAIClipTokenizer> {
    if (cachedClipTokenizerPromise && !forceRefresh) {
        return cachedClipTokenizerPromise;
    }

    const promise = (async () => {
        const tokenUrl = "https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true";
        const dataStr = await fetchData(tokenUrl, forceRefresh);

        let jsonData;
        try {
            jsonData = JSON.parse(dataStr);
        } catch (error) {
            throw new TokenizerError('Failed to parse CLIP tokenizer data as JSON', error);
        }

        if (!jsonData['text']) {
            throw new TokenizerError('CLIP tokenizer data missing "text" field');
        }

        return new NovelAIClipTokenizer(jsonData['text']);
    })();

    cachedClipTokenizerPromise = promise;
    promise.catch(() => {
        // On failure, clear the cached promise so retries can succeed
        if (cachedClipTokenizerPromise === promise) {
            cachedClipTokenizerPromise = null;
        }
    });

    return promise;
}

// Abstraction for tokenizer backends (native vs pure JS)
interface TokenizerBackend {
    encode(text: string): Promise<number[]> | number[];
    tokenToId(token: string): number | null;
}

/**
 * Pure JavaScript Unigram tokenizer implementation.
 * Used as a fallback when the native `tokenizers` package is unavailable
 * (e.g., macOS Apple Silicon where tokenizers-darwin-arm64 is not published).
 *
 * Implements:
 * - NFKC normalization (approximation of Precompiled normalizer)
 * - WhitespaceSplit + Metaspace pre-tokenization
 * - Viterbi algorithm for optimal Unigram segmentation
 */
export class PureJSUnigram implements TokenizerBackend {
    private vocab: Map<string, number>;      // piece → log score
    private pieceToId: Map<string, number>;  // piece → token ID
    private unkId: number;
    private unkScore: number;
    private maxPieceLength: number;

    constructor(vocabEntries: [string, number][], unkId: number) {
        this.unkId = unkId;
        this.vocab = new Map();
        this.pieceToId = new Map();
        this.maxPieceLength = 0;

        let minScore = 0;
        for (let i = 0; i < vocabEntries.length; i++) {
            const [piece, score] = vocabEntries[i];
            this.vocab.set(piece, score);
            this.pieceToId.set(piece, i);
            const pieceCodePointLength = Array.from(piece).length;
            if (pieceCodePointLength > this.maxPieceLength) {
                this.maxPieceLength = pieceCodePointLength;
            }
            if (score !== 0 && score < minScore) {
                minScore = score;
            }
        }

        // SentencePiece uses min_score - kUnkPenalty (10.0) for unknown characters
        this.unkScore = minScore - 10;
    }

    /**
     * Encode text into token IDs using Unigram model with Viterbi algorithm.
     * Pre-tokenization: NFKC normalize → WhitespaceSplit → Metaspace (▁ prefix)
     */
    encode(text: string): number[] {
        // 1. NFKC normalization (approximation of Precompiled normalizer)
        const normalized = text.normalize('NFKC');

        // 2. WhitespaceSplit: split on whitespace
        const pieces = normalized.split(/\s+/).filter(p => p.length > 0);
        if (pieces.length === 0) return [];

        // 3. Metaspace: prepend ▁ to each piece (add_prefix_space: true)
        const metaspaced = pieces.map(p => '\u2581' + p);

        // 4. Viterbi on each metaspaced piece
        const ids: number[] = [];
        for (const piece of metaspaced) {
            const pieceIds = this.viterbi(piece);
            ids.push(...pieceIds);
        }

        return ids;
    }

    /**
     * Viterbi algorithm for optimal Unigram segmentation.
     * Finds the highest-scoring segmentation of the input text into vocab pieces.
     * Uses code point iteration to correctly handle BMP-external characters (e.g., emoji).
     */
    private viterbi(text: string): number[] {
        // Split into code points to handle surrogate pairs correctly
        const chars = Array.from(text);
        const len = chars.length;
        if (len === 0) return [];

        // best[i] = { score, prev } for position i (code points 0..i processed)
        const best: Array<{ score: number; prev: number }> = new Array(len + 1);
        best[0] = { score: 0, prev: -1 };

        for (let i = 1; i <= len; i++) {
            best[i] = { score: -Infinity, prev: 0 };

            for (let l = 1; l <= Math.min(this.maxPieceLength, i); l++) {
                const substr = chars.slice(i - l, i).join('');
                const score = this.vocab.get(substr);

                if (score !== undefined) {
                    const candidate = best[i - l].score + score;
                    if (candidate > best[i].score) {
                        best[i] = { score: candidate, prev: i - l };
                    }
                }
            }

            // If no vocab match found, single char fallback to unk
            if (best[i].score === -Infinity) {
                best[i] = { score: best[i - 1].score + this.unkScore, prev: i - 1 };
            }
        }

        // Backtrack to recover pieces
        const pieces: string[] = [];
        let pos = len;
        while (pos > 0) {
            const prev = best[pos].prev;
            pieces.push(chars.slice(prev, pos).join(''));
            pos = prev;
        }
        pieces.reverse();

        // Convert pieces to token IDs
        return pieces.map(p => this.pieceToId.get(p) ?? this.unkId);
    }

    tokenToId(token: string): number | null {
        return this.pieceToId.get(token) ?? null;
    }
}

export class NovelAIT5Tokenizer {
    private eosTokenId: number;
    private backend: TokenizerBackend;

    private constructor(backend: TokenizerBackend, eosTokenId: number) {
        this.backend = backend;
        this.eosTokenId = eosTokenId;
    }

    /**
     * Create from native `tokenizers` package.
     */
    static createFromNative(tokenizer: any): NovelAIT5Tokenizer {
        const eosId = tokenizer.tokenToId("</s>") ?? 1;
        const backend: TokenizerBackend = {
            encode: async (text: string) => {
                const encoding = await tokenizer.encode(text);
                return Array.from(encoding.getIds());
            },
            tokenToId: (token: string) => tokenizer.tokenToId(token) ?? null,
        };
        return new NovelAIT5Tokenizer(backend, eosId);
    }

    /**
     * Create from pure JS Unigram fallback.
     */
    static createFromPureJS(pureJS: PureJSUnigram): NovelAIT5Tokenizer {
        const eosId = pureJS.tokenToId("</s>") ?? 1;
        return new NovelAIT5Tokenizer(pureJS, eosId);
    }

    /**
     * Encode text using official NovelAI T5 logic.
     * Returns the full token array INCLUDING EOS (for model input).
     *
     * For display purposes (matching official site), use countTokens() instead.
     */
    public async encode(text: string): Promise<number[]> {
        // 1. Empty check
        if (!text || text.length === 0) {
            return [this.eosTokenId];
        }

        // 2. Preprocess
        const processed = preprocessT5(text);

        // 3. Encode via backend
        const ids = Array.from(await this.backend.encode(processed));

        // 4. Append EOS
        ids.push(this.eosTokenId);

        return ids;
    }

    /**
     * Count tokens matching official NovelAI UI display.
     * Returns token count INCLUDING EOS token.
     *
     * This is what the NovelAI website shows in the token counter.
     */
    public async countTokens(text: string): Promise<number> {
        const ids = await this.encode(text);
        return ids.length;
    }
}

export async function getT5Tokenizer(forceRefresh = false): Promise<NovelAIT5Tokenizer> {
    if (cachedT5TokenizerPromise && !forceRefresh) {
        return cachedT5TokenizerPromise;
    }

    const promise = (async () => {
        const tokenUrl = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true";
        const dataStr = await fetchData(tokenUrl, forceRefresh);

        // Try native tokenizer first
        const nativeMod = await tryLoadNativeTokenizer();
        if (nativeMod) {
            try {
                const tokenizer = await nativeMod.Tokenizer.fromString(dataStr);
                return NovelAIT5Tokenizer.createFromNative(tokenizer);
            } catch {
                // Fall back to pure JS implementation
            }
        }

        // Fallback: Pure JS Unigram
        let json: any;
        try {
            json = JSON.parse(dataStr);
        } catch (error) {
            throw new TokenizerError('Failed to parse T5 tokenizer data as JSON', error);
        }

        // Validate JSON structure
        if (!json?.model?.vocab || !Array.isArray(json.model.vocab)) {
            throw new TokenizerError('T5 tokenizer data missing or invalid "model.vocab" array');
        }
        if (typeof json.model.unk_id !== 'number') {
            throw new TokenizerError('T5 tokenizer data missing or invalid "model.unk_id" number');
        }

        const vocab: [string, number][] = json.model.vocab;
        const unkId: number = json.model.unk_id;
        const pureJS = new PureJSUnigram(vocab, unkId);
        return NovelAIT5Tokenizer.createFromPureJS(pureJS);
    })();

    cachedT5TokenizerPromise = promise;
    promise.catch(() => {
        // On failure, clear the cached promise so retries can succeed
        if (cachedT5TokenizerPromise === promise) {
            cachedT5TokenizerPromise = null;
        }
    });

    return promise;
}

/**
 * Preprocess text for T5 tokenizer.
 * 
 * IMPORTANT: Based on official NovelAI JavaScript (9423.2de67be589ffa59d.js),
 * T5 preprocessing ONLY removes brackets and weight syntax.
 * Unlike CLIP, it does NOT:
 * - Decode HTML entities
 * - Normalize whitespace
 * - Convert to lowercase
 */
export function preprocessT5(text: string): string {
    // 1. Remove brackets [] and {}
    text = text.replace(/[[\]{}]/g, "");

    // 2. Remove weighting syntax (e.g., "2::content::", "1.5::content::", "-1::content::")
    // Matches NUMBER::content:: pairs, replacing with just the content
    text = text.replace(/(-?\d+\.?\d*)?::((?:(?!::)[\s\S])+)(?:::)/g, '$2');

    return text;
}

// Main logic for direct execution test?
// The user asked to transplant, so exporting functions is good.
// But we can add a main block if run directly.

/**
 * Validates that the token count for the given text does not exceed MAX_TOKENS (512).
 * @param text - The text to validate
 * @throws {TokenValidationError} If token count exceeds MAX_TOKENS
 * @returns {Promise<number>} The token count if valid
 */
export async function validateTokenCount(text: string): Promise<number> {
    const tokenizer = await getT5Tokenizer();
    const tokenCount = await tokenizer.countTokens(text);
    
    if (tokenCount > MAX_TOKENS) {
        throw new TokenValidationError(
            `Token count (${tokenCount}) exceeds maximum allowed (${MAX_TOKENS})`,
            tokenCount,
            MAX_TOKENS
        );
    }
    
    return tokenCount;
}

// Helper function for clearing cached tokenizers (useful for testing)
export function clearTokenizerCache(): void {
    cachedClipTokenizerPromise = null;
    cachedT5TokenizerPromise = null;
    nativeTokenizerModule = null;
    nativeTokenizerUnavailable = false;
}

// Main module check (CommonJS)
const isMainModule = require.main === module;

if (isMainModule) {
    (async () => {
        const args = process.argv.slice(2);
        const text = args.length > 0 ? args.join(" ") : "Hello World";

        console.log(`\nProcessing Text: ${text.length > 50 ? text.slice(0, 50) + "..." : text}`);

        try {
            const clipTokenizer = await getClipTokenizer();
            const clipTokens = clipTokenizer.encode(text);
            console.log(`\n[Raw Token Count] (CLIP, includes weights): ${clipTokens.length}`);
        } catch (e) {
            console.error("Error loading CLIP tokenizer:", e);
        }

        try {
            const t5Tokenizer = await getT5Tokenizer();
            const textWithTags = "masterpiece, best quality, " + text;

            // New API returns number[] directly
            const ids = await t5Tokenizer.encode(textWithTags);

            console.log(`\n[Effective Token Count] (T5, with quality tags + EOS): ${ids.length}`);
            console.log(`Token IDs: [${ids.slice(0, 10).join(', ')}${ids.length > 10 ? ', ...' : ''}]`);
        } catch (e) {
             console.error("Error loading T5 tokenizer:", e);
        }
    })();
}