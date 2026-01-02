import axios from 'axios';
import * as zlib from 'zlib';
import he from 'he';
import { Tokenizer } from 'tokenizers';

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
    private cache: { [key: string]: string };
    private pat: RegExp;

    constructor(definitionText: string) {
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

        this.cache = {
            "<|startoftext|>": "<|startoftext|>",
            "<|endoftext|>": "<|endoftext|>"
        };

        // Regex pattern from Python:
        // r"""<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+"""
        // JS equivalent with 'u' flag:
        this.pat = /<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+/gu;
    }

    private bpe(token: string): string {
        if (token in this.cache) {
            return this.cache[token];
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
        this.cache[token] = result;
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
             // In JS, we can get bytes using TextEncoder
             const encoder = new TextEncoder();
             const tokenBytes = encoder.encode(token);

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

async function fetchData(url: string): Promise<string> {
    console.log(`Fetching ${url}...`);
    const response = await axios.get(url, {
        responseType: 'arraybuffer',
        headers: { "User-Agent": "Mozilla/5.0" }
    });

    const buffer = Buffer.from(response.data);
    let data: Buffer;

    console.log("Decompressing data...");
    try {
        // -15 for raw deflate
        data = zlib.inflateRawSync(buffer);
    } catch (e) {
        // try standard inflate
        data = zlib.inflateSync(buffer);
    }

    return data.toString("utf-8");
}

export async function getClipTokenizer(): Promise<NovelAIClipTokenizer> {
    const url = "https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true";
    const dataStr = await fetchData(url);
    const jsonData = JSON.parse(dataStr);
    return new NovelAIClipTokenizer(jsonData["text"]);
}

export async function getT5Tokenizer(): Promise<Tokenizer> {
    const url = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true";
    // Using axios/fetch to get the JSON string for tokenizers
    // Tokenizer.fromData / fromString

    const dataStr = await fetchData(url);
    // dataStr is the JSON content for the tokenizer

    return Tokenizer.fromString(dataStr);
}

export function preprocessT5(text: string): string {
    // 1. Remove brackets [] and {}
    text = text.replace(/[[\]{}]/g, "");

    // 2. Remove weighting syntax
    // Python: re.sub(r"-?\d*\.?\d*::", "", text)
    // JS RegExp needs to be careful.
    text = text.replace(/-?\d*\.?\d*::/g, "");

    // 3. HTML unescape
    text = he.decode(he.decode(text)).trim();

    // 4. Whitespace and lowercase
    text = text.replace(/\s+/g, ' ').trim().toLowerCase();

    return text;
}

// Main logic for direct execution test?
// The user asked to transplant, so exporting functions is good.
// But we can add a main block if run directly.

if (require.main === module) {
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
            const processedText = preprocessT5(text);
            const textWithTags = "masterpiece, best quality, " + processedText;

            const encoded = t5Tokenizer.encode(textWithTags);
            let count = encoded.getIds().length;

            // Check EOS (id 1)
            const ids = encoded.getIds();
            if (ids[ids.length - 1] === 1) {
                count -= 1;
            }

            console.log(`\n[Effective Token Count] (T5, weights removed + quality tags): ${count}`);
        } catch (e) {
             console.error("Error loading T5 tokenizer:", e);
        }
    })();
}
