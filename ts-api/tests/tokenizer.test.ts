/**
 * NovelAI Tokenizer Tests
 * tokenizer.ts のユニットテスト
 */

import { describe, it, expect, beforeAll, afterEach, vi } from 'vitest';

// Mock the tokenizers module to avoid native binary dependency
vi.mock('tokenizers', () => ({
    Tokenizer: {
        fromString: vi.fn().mockResolvedValue({
            encode: vi.fn().mockResolvedValue({
                getIds: vi.fn().mockReturnValue([1, 2, 3]),
            }),
        }),
    },
}));

import {
    NovelAIClipTokenizer,
    PureJSUnigram,
    preprocessT5,
    clearTokenizerCache,
    TokenizerError,
} from '../src/tokenizer';

// =============================================================================
// Mock定義用のテストデータ
// =============================================================================

// 最小限のトークナイザー定義（テスト用）
// 実際のトークナイザーはネットワークフェッチが必要なため、
// CLIPトークナイザーの直接初期化テスト用に簡略化した定義を使用
const MOCK_TOKENIZER_DEFINITION = `#version: 1.0
he llo
wo rld
he llo</w>
`;

// =============================================================================
// NovelAIClipTokenizer Tests
// =============================================================================
describe('NovelAIClipTokenizer', () => {
    let tokenizer: NovelAIClipTokenizer;

    beforeAll(() => {
        tokenizer = new NovelAIClipTokenizer(MOCK_TOKENIZER_DEFINITION);
    });

    describe('constructor', () => {
        it('should create a tokenizer instance', () => {
            expect(tokenizer).toBeInstanceOf(NovelAIClipTokenizer);
        });
    });

    describe('encode()', () => {
        it('should return an array of numbers', () => {
            const result = tokenizer.encode('hello');
            expect(Array.isArray(result)).toBe(true);
            result.forEach(token => {
                expect(typeof token).toBe('number');
            });
        });

        it('should return empty array for empty string', () => {
            const result = tokenizer.encode('');
            expect(result).toEqual([]);
        });

        it('should return empty array for whitespace only', () => {
            const result = tokenizer.encode('   ');
            expect(result).toEqual([]);
        });

        it('should handle HTML entities', () => {
            // he.decode should convert &amp; to &
            const result1 = tokenizer.encode('&amp;');
            const result2 = tokenizer.encode('&');
            // 両方とも同じ結果になるはず
            expect(result1).toEqual(result2);
        });

        it('should handle double-encoded HTML entities', () => {
            // &amp;amp; -> &amp; -> &
            const result = tokenizer.encode('&amp;amp;');
            const expected = tokenizer.encode('&');
            expect(result).toEqual(expected);
        });

        it('should lowercase text', () => {
            const resultUpper = tokenizer.encode('HELLO');
            const resultLower = tokenizer.encode('hello');
            expect(resultUpper).toEqual(resultLower);
        });

        it('should normalize whitespace', () => {
            const result1 = tokenizer.encode('hello    world');
            const result2 = tokenizer.encode('hello world');
            expect(result1).toEqual(result2);
        });

        it('should handle special characters', () => {
            // Should not throw
            expect(() => tokenizer.encode('!@#$%^&*()')).not.toThrow();
            expect(() => tokenizer.encode('日本語')).not.toThrow();
            expect(() => tokenizer.encode('émojis 🎨')).not.toThrow();
        });

        it('should handle newlines and tabs', () => {
            const result1 = tokenizer.encode('hello\n\tworld');
            const result2 = tokenizer.encode('hello world');
            expect(result1).toEqual(result2);
        });
    });
});

// =============================================================================
// preprocessT5 Tests
// =============================================================================
describe('preprocessT5', () => {
    /**
     * NOTE: Based on official NovelAI JavaScript (9423.2de67be589ffa59d.js),
     * T5 preprocessing ONLY removes brackets and weight syntax.
     * Unlike CLIP, it does NOT:
     * - Decode HTML entities
     * - Normalize whitespace  
     * - Convert to lowercase
     */
    
    describe('bracket removal', () => {
        it('should remove square brackets', () => {
            expect(preprocessT5('[1girl]')).toBe('1girl');
        });

        it('should remove curly brackets', () => {
            expect(preprocessT5('{beautiful}')).toBe('beautiful');
        });

        it('should remove mixed brackets', () => {
            expect(preprocessT5('[1girl, {beautiful}]')).toBe('1girl, beautiful');
        });

        it('should handle nested brackets', () => {
            expect(preprocessT5('[[nested]]')).toBe('nested');
        });

        it('should handle double curly brackets', () => {
            expect(preprocessT5('{{sitting}}')).toBe('sitting');
        });
    });

    describe('weight syntax removal', () => {
        it('should remove integer weight syntax', () => {
            expect(preprocessT5('1girl, 2::beautiful::')).toBe('1girl, beautiful');
        });

        it('should remove decimal weight syntax', () => {
            expect(preprocessT5('1girl, 1.5::beautiful::')).toBe('1girl, beautiful');
        });

        it('should remove negative weight syntax', () => {
            expect(preprocessT5('1girl, -1::bad::')).toBe('1girl, bad');
        });

        it('should remove weight syntax without number', () => {
            expect(preprocessT5('1girl, ::beautiful::')).toBe('1girl, beautiful');
        });

        it('should handle multiple weight syntaxes', () => {
            expect(preprocessT5('1.2::girl::, 0.8::beautiful::')).toBe('girl, beautiful');
        });

        it('should handle complex NovelAI-style weight syntax', () => {
            // Example from user: 3::rosa (pokemon)::, 2::smile::
            const input = '3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::';
            const expected = 'rosa (pokemon), smile, artist:ixy, artist:ahemaru';
            expect(preprocessT5(input)).toBe(expected);
        });
    });

    describe('preserves case and whitespace (unlike CLIP)', () => {
        it('should preserve uppercase (unlike CLIP)', () => {
            expect(preprocessT5('1GIRL, BEAUTIFUL')).toBe('1GIRL, BEAUTIFUL');
        });

        it('should preserve mixed case', () => {
            expect(preprocessT5('MaStErPiEcE')).toBe('MaStErPiEcE');
        });

        it('should preserve multiple spaces', () => {
            expect(preprocessT5('1girl    beautiful')).toBe('1girl    beautiful');
        });

        it('should preserve tabs and newlines', () => {
            expect(preprocessT5('1girl\t\nbeautiful')).toBe('1girl\t\nbeautiful');
        });

        it('should preserve leading/trailing whitespace', () => {
            expect(preprocessT5('  1girl  ')).toBe('  1girl  ');
        });
    });

    describe('preserves HTML entities (unlike CLIP)', () => {
        it('should preserve &amp; as-is', () => {
            expect(preprocessT5('rock &amp; roll')).toBe('rock &amp; roll');
        });

        it('should preserve &lt; and &gt; as-is', () => {
            expect(preprocessT5('&lt;tag&gt;')).toBe('&lt;tag&gt;');
        });
    });

    describe('combined operations', () => {
        it('should handle complex prompts (brackets and weights only)', () => {
            const input = '[1girl], {1.5::beautiful::}, MASTERPIECE';
            const expected = '1girl, beautiful, MASTERPIECE';
            expect(preprocessT5(input)).toBe(expected);
        });

        it('should handle empty string', () => {
            expect(preprocessT5('')).toBe('');
        });

        it('should handle user example prompt', () => {
            const input = '3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}';
            const expected = 'rosa (pokemon), smile, artist:ixy, artist:ahemaru, sitting';
            expect(preprocessT5(input)).toBe(expected);
        });
    });
});

// =============================================================================
// TokenizerError Tests
// =============================================================================
describe('TokenizerError', () => {
    it('should be an instance of Error', () => {
        const error = new TokenizerError('test error');
        expect(error).toBeInstanceOf(Error);
        expect(error).toBeInstanceOf(TokenizerError);
    });

    it('should have correct name', () => {
        const error = new TokenizerError('test error');
        expect(error.name).toBe('TokenizerError');
    });

    it('should store message', () => {
        const error = new TokenizerError('test message');
        expect(error.message).toBe('test message');
    });

    it('should store cause', () => {
        const cause = new Error('original error');
        const error = new TokenizerError('wrapped error', cause);
        expect(error.cause).toBe(cause);
    });

    it('should work without cause', () => {
        const error = new TokenizerError('no cause');
        expect(error.cause).toBeUndefined();
    });
});

// =============================================================================
// clearTokenizerCache Tests
// =============================================================================
describe('clearTokenizerCache', () => {
    afterEach(() => {
        clearTokenizerCache();
    });

    it('should not throw when called', () => {
        expect(() => clearTokenizerCache()).not.toThrow();
    });

    it('should be callable multiple times', () => {
        clearTokenizerCache();
        clearTokenizerCache();
        clearTokenizerCache();
        // Should not throw
    });
});

// =============================================================================
// PureJSUnigram Tests
// =============================================================================
describe('PureJSUnigram', () => {
    // Minimal test vocab:
    //  0: <pad>    (score 0, special)
    //  1: </s>     (score 0, special / EOS)
    //  2: <unk>    (score 0, special)
    //  3: ▁        (score -2.0)
    //  4: ▁hello   (score -5.0)
    //  5: ▁world   (score -5.5)
    //  6: ▁he      (score -6.0)
    //  7: llo      (score -6.0)
    //  8: wor      (score -7.0)
    //  9: ld       (score -7.0)
    // 10: h        (score -8.0)
    // 11: e        (score -8.0)
    // 12: l        (score -8.0)
    // 13: o        (score -8.0)
    // 14: w        (score -8.0)
    // 15: r        (score -8.0)
    // 16: d        (score -8.0)
    // 17: ,        (score -4.0)
    // 18: ▁,       (score -3.5)
    const MINI_VOCAB: [string, number][] = [
        ['<pad>', 0],
        ['</s>', 0],
        ['<unk>', 0],
        ['\u2581', -2.0],
        ['\u2581hello', -5.0],
        ['\u2581world', -5.5],
        ['\u2581he', -6.0],
        ['llo', -6.0],
        ['wor', -7.0],
        ['ld', -7.0],
        ['h', -8.0],
        ['e', -8.0],
        ['l', -8.0],
        ['o', -8.0],
        ['w', -8.0],
        ['r', -8.0],
        ['d', -8.0],
        [',', -4.0],
        ['\u2581,', -3.5],
    ];
    const UNK_ID = 2;

    let tokenizer: PureJSUnigram;

    beforeAll(() => {
        tokenizer = new PureJSUnigram(MINI_VOCAB, UNK_ID);
    });

    describe('constructor', () => {
        it('should create a PureJSUnigram instance', () => {
            expect(tokenizer).toBeInstanceOf(PureJSUnigram);
        });
    });

    describe('tokenToId()', () => {
        it('should return correct ID for known tokens', () => {
            expect(tokenizer.tokenToId('</s>')).toBe(1);
            expect(tokenizer.tokenToId('<unk>')).toBe(2);
            expect(tokenizer.tokenToId('\u2581hello')).toBe(4);
        });

        it('should return null for unknown tokens', () => {
            expect(tokenizer.tokenToId('nonexistent')).toBeNull();
        });
    });

    describe('encode()', () => {
        it('should return an array of numbers', () => {
            const result = tokenizer.encode('hello');
            expect(Array.isArray(result)).toBe(true);
            result.forEach(id => {
                expect(typeof id).toBe('number');
            });
        });

        it('should return empty array for empty string', () => {
            expect(tokenizer.encode('')).toEqual([]);
        });

        it('should return empty array for whitespace only', () => {
            expect(tokenizer.encode('   ')).toEqual([]);
        });

        it('should prefer longer matching pieces (Viterbi)', () => {
            // "hello" → pre-tokenized as "▁hello"
            // ▁hello (id=4, score=-5.0) is better than ▁he+llo (score=-12.0) or ▁+h+e+l+l+o
            const result = tokenizer.encode('hello');
            expect(result).toEqual([4]); // ▁hello
        });

        it('should handle multiple words', () => {
            // "hello world" → WhitespaceSplit → ["hello", "world"]
            // → Metaspace → ["▁hello", "▁world"]
            // → Viterbi: ▁hello(4), ▁world(5)
            const result = tokenizer.encode('hello world');
            expect(result).toEqual([4, 5]);
        });

        it('should use unk for characters not in vocab', () => {
            // "xyz" → "▁xyz"
            // ▁ is in vocab, but x,y,z are not → unk for each unknown char
            const result = tokenizer.encode('xyz');
            // ▁ (id=3) then x→unk(2), y→unk(2), z→unk(2)
            expect(result).toContain(UNK_ID);
        });

        it('should handle mixed spaces and text', () => {
            // Multiple spaces are collapsed by WhitespaceSplit
            const result1 = tokenizer.encode('hello   world');
            const result2 = tokenizer.encode('hello world');
            expect(result1).toEqual(result2);
        });
    });
});

// =============================================================================
// PureJSUnigram with real T5 vocab (if cached file available)
// =============================================================================
describe('PureJSUnigram with real T5 vocab', () => {
    let tokenizer: PureJSUnigram | null = null;

    beforeAll(async () => {
        const fs = await import('fs/promises');
        const path = await import('path');
        const cachePath = path.join(__dirname, '..', '.cache', 'tokenizers', 't5_tokenizer_v2.json');
        try {
            const data = await fs.readFile(cachePath, 'utf-8');
            const json = JSON.parse(data);
            tokenizer = new PureJSUnigram(json.model.vocab, json.model.unk_id);
        } catch {
            // Cache file not available, skip tests
        }
    });

    it('should tokenize simple English text', () => {
        if (!tokenizer) return;
        const ids = tokenizer.encode('hello world');
        expect(ids.length).toBeGreaterThan(0);
        // All IDs should be non-negative integers
        ids.forEach(id => {
            expect(id).toBeGreaterThanOrEqual(0);
            expect(Number.isInteger(id)).toBe(true);
        });
    });

    it('should tokenize NovelAI-style prompts', () => {
        if (!tokenizer) return;
        const ids = tokenizer.encode('1girl, beautiful, masterpiece, best quality');
        expect(ids.length).toBeGreaterThan(0);
    });

    it('should resolve </s> EOS token to ID 1', () => {
        if (!tokenizer) return;
        expect(tokenizer.tokenToId('</s>')).toBe(1);
    });

    it('should resolve <unk> token to ID 2', () => {
        if (!tokenizer) return;
        expect(tokenizer.tokenToId('<unk>')).toBe(2);
    });

    it('should handle empty string', () => {
        if (!tokenizer) return;
        expect(tokenizer.encode('')).toEqual([]);
    });

    it('should handle Japanese text (CJK characters)', () => {
        if (!tokenizer) return;
        const ids = tokenizer.encode('美しい少女');
        expect(ids.length).toBeGreaterThan(0);
    });
});

// =============================================================================
// Integration Tests (Network-dependent, skip in CI)
// =============================================================================
describe.skip('Integration Tests (requires network)', () => {
    // これらのテストはネットワーク接続が必要なため、
    // 通常のテスト実行ではスキップされます。
    // 手動で実行する場合は describe.skip を describe に変更してください。

    it('should fetch and create CLIP tokenizer', async () => {
        const { getClipTokenizer } = await import('../src/tokenizer');
        const tokenizer = await getClipTokenizer();
        expect(tokenizer).toBeInstanceOf(NovelAIClipTokenizer);

        const tokens = tokenizer.encode('1girl, beautiful');
        expect(tokens.length).toBeGreaterThan(0);
    });

    it('should cache CLIP tokenizer', async () => {
        const { getClipTokenizer } = await import('../src/tokenizer');
        const tokenizer1 = await getClipTokenizer();
        const tokenizer2 = await getClipTokenizer();
        expect(tokenizer1).toBe(tokenizer2);
    });

    it('should force refresh CLIP tokenizer', async () => {
        const { getClipTokenizer } = await import('../src/tokenizer');
        const tokenizer1 = await getClipTokenizer();
        const tokenizer2 = await getClipTokenizer(true);
        // Note: これは新しいインスタンスを作成するので同一ではないが、
        // 同じ機能を持つはず
        expect(tokenizer2).toBeInstanceOf(NovelAIClipTokenizer);
    });

    it('should fetch and create T5 tokenizer', async () => {
        const { getT5Tokenizer } = await import('../src/tokenizer');
        const tokenizer = await getT5Tokenizer();
        expect(tokenizer).toBeDefined();

        // New API returns number[] directly
        const ids = await tokenizer.encode('1girl, beautiful');
        expect(Array.isArray(ids)).toBe(true);
        expect(ids.length).toBeGreaterThan(0);
        
        // Should end with EOS token (assumed to be 1 or whatever mock returns)
        // In real execution it's 1. In mock we need to check what tokenizer.encode returns.
        // But getT5Tokenizer uses real Tokenizer.fromString in implementation, 
        // while the test mocks 'tokenizers' module.
    });
});
