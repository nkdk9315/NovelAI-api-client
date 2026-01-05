import { getT5Tokenizer } from "./src/tokenizer";

(async () => {
    const tokenizer = await getT5Tokenizer();
    
    // countTokens() を使用（公式UIと一致）
    const count1 = await tokenizer.countTokens("3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}");
    console.log(`Token count: ${count1}`); // → 25

    const count2 = await tokenizer.countTokens("2::girls::, 2::smile, standing, ::, {{ scared }}, 0.8::artist:ixy, artist:ahemaru::, -2::multiple views::, target#hands in skirt");
    console.log(`Token count: ${count2}`);
})();