//! Qwen tokenizer (V5) must match the official site's encoder output.
//! The expected IDs were produced by running the site's own BPE class on the same definition file.

use novelai_api::tokenizer::get_qwen_tokenizer;

#[derive(serde::Deserialize)]
struct Case {
    text: String,
    ids: Vec<u32>,
}

#[tokio::test]
async fn qwen_matches_official_encoder() {
    let tokenizer = match get_qwen_tokenizer(false).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("skipping: Qwen tokenizer unavailable ({})", e);
            return;
        }
    };
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/qwen_expected.json")).unwrap();
    for case in cases {
        assert_eq!(tokenizer.encode(&case.text), case.ids, "text: {:?}", case.text);
    }
}
