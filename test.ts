import { getClipTokenizer, getT5Tokenizer, preprocessT5 } from './novelai_tokenizer';

async function runTest() {
    const args = process.argv.slice(2);
    const text = args.length > 0 ? args.join(" ") : "Hello World! This is a test.";

    console.log(`\nProcessing Text: ${text.length > 50 ? text.slice(0, 50) + "..." : text}`);

    try {
        console.log("Loading CLIP tokenizer...");
        const clipTokenizer = await getClipTokenizer();
        const clipTokens = clipTokenizer.encode(text);
        console.log(`[Raw Token Count] (CLIP, includes weights): ${clipTokens.length}`);
        console.log(`IDs: ${clipTokens}`);
    } catch (e) {
        console.error("Error loading CLIP tokenizer:", e);
    }

    try {
        console.log("\nLoading T5 tokenizer...");
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

        console.log(`[Effective Token Count] (T5, weights removed + quality tags): ${count}`);
        console.log(`IDs: ${ids}`);
    } catch (e) {
         console.error("Error loading T5 tokenizer:", e);
    }
}

runTest();
