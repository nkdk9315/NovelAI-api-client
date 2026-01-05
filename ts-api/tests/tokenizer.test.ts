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
