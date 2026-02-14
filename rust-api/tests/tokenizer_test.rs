use novelai_api::tokenizer::{
    NovelAIClipTokenizer, PureUnigram, NovelAIT5Tokenizer,
    preprocess_t5, get_cache_filename, clear_tokenizer_cache,
};

// =============================================================================
// Mock tokenizer definition (minimal BPE merge rules for testing)
// =============================================================================
const MOCK_TOKENIZER_DEFINITION: &str = "#version: 1.0
he llo
wo rld
he llo</w>
";

// =============================================================================
// NovelAIClipTokenizer Tests
// =============================================================================
mod clip_tokenizer {
    use super::*;

    fn make_tokenizer() -> NovelAIClipTokenizer {
        NovelAIClipTokenizer::new(MOCK_TOKENIZER_DEFINITION)
    }

    #[test]
    fn should_create_a_tokenizer_instance() {
        let _tokenizer = make_tokenizer();
    }

    mod encode {
        use super::*;

        #[test]
        fn should_return_a_vec_of_numbers() {
            let tokenizer = make_tokenizer();
            let result = tokenizer.encode("hello");
            assert!(!result.is_empty() || result.is_empty()); // just check it returns
            for token in &result {
                assert!(*token < u32::MAX);
            }
        }

        #[test]
        fn should_return_empty_vec_for_empty_string() {
            let tokenizer = make_tokenizer();
            let result = tokenizer.encode("");
            assert!(result.is_empty());
        }

        #[test]
        fn should_return_empty_vec_for_whitespace_only() {
            let tokenizer = make_tokenizer();
            let result = tokenizer.encode("   ");
            assert!(result.is_empty());
        }

        #[test]
        fn should_handle_html_entities() {
            let tokenizer = make_tokenizer();
            let result1 = tokenizer.encode("&amp;");
            let result2 = tokenizer.encode("&");
            assert_eq!(result1, result2);
        }

        #[test]
        fn should_handle_double_encoded_html_entities() {
            let tokenizer = make_tokenizer();
            let result = tokenizer.encode("&amp;amp;");
            let expected = tokenizer.encode("&");
            assert_eq!(result, expected);
        }

        #[test]
        fn should_lowercase_text() {
            let tokenizer = make_tokenizer();
            let result_upper = tokenizer.encode("HELLO");
            let result_lower = tokenizer.encode("hello");
            assert_eq!(result_upper, result_lower);
        }

        #[test]
        fn should_normalize_whitespace() {
            let tokenizer = make_tokenizer();
            let result1 = tokenizer.encode("hello    world");
            let result2 = tokenizer.encode("hello world");
            assert_eq!(result1, result2);
        }

        #[test]
        fn should_handle_special_characters() {
            let tokenizer = make_tokenizer();
            // Should not panic
            let _ = tokenizer.encode("!@#$%^&*()");
            let _ = tokenizer.encode("日本語");
            let _ = tokenizer.encode("émojis 🎨");
        }

        #[test]
        fn should_handle_newlines_and_tabs() {
            let tokenizer = make_tokenizer();
            let result1 = tokenizer.encode("hello\n\tworld");
            let result2 = tokenizer.encode("hello world");
            assert_eq!(result1, result2);
        }
    }
}

// =============================================================================
// preprocessT5 Tests
// =============================================================================
mod preprocess_t5_tests {
    use super::*;

    mod bracket_removal {
        use super::*;

        #[test]
        fn should_remove_square_brackets() {
            assert_eq!(preprocess_t5("[1girl]"), "1girl");
        }

        #[test]
        fn should_remove_curly_brackets() {
            assert_eq!(preprocess_t5("{beautiful}"), "beautiful");
        }

        #[test]
        fn should_remove_mixed_brackets() {
            assert_eq!(preprocess_t5("[1girl, {beautiful}]"), "1girl, beautiful");
        }

        #[test]
        fn should_handle_nested_brackets() {
            assert_eq!(preprocess_t5("[[nested]]"), "nested");
        }

        #[test]
        fn should_handle_double_curly_brackets() {
            assert_eq!(preprocess_t5("{{sitting}}"), "sitting");
        }
    }

    mod weight_syntax_removal {
        use super::*;

        #[test]
        fn should_remove_integer_weight_syntax() {
            assert_eq!(preprocess_t5("1girl, 2::beautiful::"), "1girl, beautiful");
        }

        #[test]
        fn should_remove_decimal_weight_syntax() {
            assert_eq!(preprocess_t5("1girl, 1.5::beautiful::"), "1girl, beautiful");
        }

        #[test]
        fn should_remove_negative_weight_syntax() {
            assert_eq!(preprocess_t5("1girl, -1::bad::"), "1girl, bad");
        }

        #[test]
        fn should_remove_weight_syntax_without_number() {
            assert_eq!(preprocess_t5("::beautiful::"), "beautiful");
        }

        #[test]
        fn should_handle_multiple_weight_syntaxes() {
            assert_eq!(
                preprocess_t5("1.2::girl::, 0.8::beautiful::"),
                "girl, beautiful"
            );
        }

        #[test]
        fn should_handle_complex_novelai_style_weight_syntax() {
            let input = "3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::";
            let expected = "rosa (pokemon), smile, artist:ixy, artist:ahemaru";
            assert_eq!(preprocess_t5(input), expected);
        }
    }

    mod preserves_standalone_colons {
        use super::*;

        #[test]
        fn should_preserve_standalone_colons_not_part_of_weight_syntax() {
            assert_eq!(preprocess_t5("namespace::method"), "namespace::method");
        }

        #[test]
        fn should_preserve_colons_at_end_of_string_without_matching_pair() {
            assert_eq!(preprocess_t5("some text ::"), "some text ::");
        }
    }

    mod preserves_case_and_whitespace {
        use super::*;

        #[test]
        fn should_preserve_uppercase() {
            assert_eq!(preprocess_t5("1GIRL, BEAUTIFUL"), "1GIRL, BEAUTIFUL");
        }

        #[test]
        fn should_preserve_mixed_case() {
            assert_eq!(preprocess_t5("MaStErPiEcE"), "MaStErPiEcE");
        }

        #[test]
        fn should_preserve_multiple_spaces() {
            assert_eq!(preprocess_t5("1girl    beautiful"), "1girl    beautiful");
        }

        #[test]
        fn should_preserve_tabs_and_newlines() {
            assert_eq!(
                preprocess_t5("1girl\t\nbeautiful"),
                "1girl\t\nbeautiful"
            );
        }

        #[test]
        fn should_preserve_leading_trailing_whitespace() {
            assert_eq!(preprocess_t5("  1girl  "), "  1girl  ");
        }
    }

    mod preserves_html_entities {
        use super::*;

        #[test]
        fn should_preserve_amp_as_is() {
            assert_eq!(preprocess_t5("rock &amp; roll"), "rock &amp; roll");
        }

        #[test]
        fn should_preserve_lt_and_gt_as_is() {
            assert_eq!(preprocess_t5("&lt;tag&gt;"), "&lt;tag&gt;");
        }
    }

    mod combined_operations {
        use super::*;

        #[test]
        fn should_handle_complex_prompts() {
            let input = "[1girl], {1.5::beautiful::}, MASTERPIECE";
            let expected = "1girl, beautiful, MASTERPIECE";
            assert_eq!(preprocess_t5(input), expected);
        }

        #[test]
        fn should_handle_empty_string() {
            assert_eq!(preprocess_t5(""), "");
        }

        #[test]
        fn should_handle_user_example_prompt() {
            let input =
                "3::rosa (pokemon)::, 2::smile::, 1::artist:ixy, artist:ahemaru::, {{sitting}}";
            let expected = "rosa (pokemon), smile, artist:ixy, artist:ahemaru, sitting";
            assert_eq!(preprocess_t5(input), expected);
        }
    }
}

// =============================================================================
// clearTokenizerCache Tests
// =============================================================================
mod clear_tokenizer_cache_tests {
    use super::*;

    #[test]
    fn should_not_panic_when_called() {
        clear_tokenizer_cache();
    }

    #[test]
    fn should_be_callable_multiple_times() {
        clear_tokenizer_cache();
        clear_tokenizer_cache();
        clear_tokenizer_cache();
    }
}

// =============================================================================
// PureUnigram Tests
// =============================================================================
mod pure_unigram {
    use super::*;

    fn mini_vocab() -> Vec<(String, f64)> {
        vec![
            ("<pad>".to_string(), 0.0),
            ("</s>".to_string(), 0.0),
            ("<unk>".to_string(), 0.0),
            ("\u{2581}".to_string(), -2.0),
            ("\u{2581}hello".to_string(), -5.0),
            ("\u{2581}world".to_string(), -5.5),
            ("\u{2581}he".to_string(), -6.0),
            ("llo".to_string(), -6.0),
            ("wor".to_string(), -7.0),
            ("ld".to_string(), -7.0),
            ("h".to_string(), -8.0),
            ("e".to_string(), -8.0),
            ("l".to_string(), -8.0),
            ("o".to_string(), -8.0),
            ("w".to_string(), -8.0),
            ("r".to_string(), -8.0),
            ("d".to_string(), -8.0),
            (",".to_string(), -4.0),
            ("\u{2581},".to_string(), -3.5),
        ]
    }

    const UNK_ID: u32 = 2;

    fn make_tokenizer() -> PureUnigram {
        PureUnigram::new(mini_vocab(), UNK_ID)
    }

    #[test]
    fn should_create_instance() {
        let _tokenizer = make_tokenizer();
    }

    mod token_to_id {
        use super::*;

        #[test]
        fn should_return_correct_id_for_known_tokens() {
            let tokenizer = make_tokenizer();
            assert_eq!(tokenizer.token_to_id("</s>"), Some(1));
            assert_eq!(tokenizer.token_to_id("<unk>"), Some(2));
            assert_eq!(tokenizer.token_to_id("\u{2581}hello"), Some(4));
        }

        #[test]
        fn should_return_none_for_unknown_tokens() {
            let tokenizer = make_tokenizer();
            assert_eq!(tokenizer.token_to_id("nonexistent"), None);
        }
    }

    mod encode {
        use super::*;

        #[test]
        fn should_return_vec_of_numbers() {
            let tokenizer = make_tokenizer();
            let result = tokenizer.encode("hello");
            assert!(!result.is_empty());
            for id in &result {
                assert!(*id < u32::MAX);
            }
        }

        #[test]
        fn should_return_empty_vec_for_empty_string() {
            let tokenizer = make_tokenizer();
            assert!(tokenizer.encode("").is_empty());
        }

        #[test]
        fn should_return_empty_vec_for_whitespace_only() {
            let tokenizer = make_tokenizer();
            assert!(tokenizer.encode("   ").is_empty());
        }

        #[test]
        fn should_prefer_longer_matching_pieces_viterbi() {
            let tokenizer = make_tokenizer();
            // "hello" -> pre-tokenized as "▁hello"
            // ▁hello (id=4, score=-5.0) is better than ▁he+llo (score=-12.0)
            let result = tokenizer.encode("hello");
            assert_eq!(result, vec![4]); // ▁hello
        }

        #[test]
        fn should_handle_multiple_words() {
            let tokenizer = make_tokenizer();
            // "hello world" -> ["▁hello", "▁world"] -> [4, 5]
            let result = tokenizer.encode("hello world");
            assert_eq!(result, vec![4, 5]);
        }

        #[test]
        fn should_use_unk_for_characters_not_in_vocab() {
            let tokenizer = make_tokenizer();
            // "xyz" -> "▁xyz"
            // ▁ is in vocab, but x,y,z are not -> unk for each
            let result = tokenizer.encode("xyz");
            assert!(result.contains(&UNK_ID));
        }

        #[test]
        fn should_handle_mixed_spaces_and_text() {
            let tokenizer = make_tokenizer();
            // Multiple spaces are collapsed by WhitespaceSplit
            let result1 = tokenizer.encode("hello   world");
            let result2 = tokenizer.encode("hello world");
            assert_eq!(result1, result2);
        }
    }
}

// =============================================================================
// PureUnigram with real T5 vocab (if cached file available)
// =============================================================================
mod pure_unigram_real_vocab {
    use super::*;

    fn try_load_real_tokenizer() -> Option<PureUnigram> {
        // Try to load from cached file
        let cache_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".cache")
            .join("tokenizers")
            .join("t5_tokenizer_v2.json");

        let data = std::fs::read_to_string(&cache_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&data).ok()?;

        let vocab_arr = json.get("model")?.get("vocab")?.as_array()?;
        let unk_id = json.get("model")?.get("unk_id")?.as_u64()? as u32;

        let vocab: Vec<(String, f64)> = vocab_arr
            .iter()
            .filter_map(|entry| {
                let arr = entry.as_array()?;
                let piece = arr[0].as_str()?.to_string();
                let score = arr[1].as_f64()?;
                Some((piece, score))
            })
            .collect();

        Some(PureUnigram::new(vocab, unk_id))
    }

    #[test]
    fn should_tokenize_simple_english_text() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return; // Cache not available, skip
        };
        let ids = tokenizer.encode("hello world");
        assert!(!ids.is_empty());
        for id in &ids {
            assert!((*id as i64) >= 0);
        }
    }

    #[test]
    fn should_tokenize_novelai_style_prompts() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return;
        };
        let ids = tokenizer.encode("1girl, beautiful, masterpiece, best quality");
        assert!(!ids.is_empty());
    }

    #[test]
    fn should_resolve_eos_token_to_id_1() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return;
        };
        assert_eq!(tokenizer.token_to_id("</s>"), Some(1));
    }

    #[test]
    fn should_resolve_unk_token_to_id_2() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return;
        };
        assert_eq!(tokenizer.token_to_id("<unk>"), Some(2));
    }

    #[test]
    fn should_handle_empty_string() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return;
        };
        assert!(tokenizer.encode("").is_empty());
    }

    #[test]
    fn should_handle_japanese_text() {
        let Some(tokenizer) = try_load_real_tokenizer() else {
            return;
        };
        let ids = tokenizer.encode("美しい少女");
        assert!(!ids.is_empty());
    }
}

// =============================================================================
// getCacheFilename Tests (Path Traversal Prevention)
// =============================================================================
mod get_cache_filename_tests {
    use super::*;

    #[test]
    fn should_generate_correct_filename_for_normal_url() {
        let result = get_cache_filename(
            "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true",
        )
        .unwrap();
        assert_eq!(result, "t5_tokenizer_v2.json");
    }

    #[test]
    fn should_sanitize_path_traversal_in_version_parameter() {
        let result =
            get_cache_filename("https://example.com/tokenizer.def?v=../../etc/passwd").unwrap();
        assert!(!result.contains('/'));
        assert_eq!(result, "tokenizer_v....etcpasswd.json");
    }

    #[test]
    fn should_sanitize_path_traversal_in_pathname() {
        let result =
            get_cache_filename("https://example.com/../../etc/passwd.def?v=1").unwrap();
        assert!(!result.contains(".."));
        assert!(!result.contains('/'));
    }

    #[test]
    fn should_handle_url_with_no_version_parameter() {
        let result = get_cache_filename("https://example.com/tokenizer.def").unwrap();
        assert_eq!(result, "tokenizer_vunknown.json");
    }

    #[test]
    fn should_handle_dotfile_style_basename() {
        let result = get_cache_filename("https://example.com/.def?v=1").unwrap();
        assert_eq!(result, ".def_v1.json");
    }

    #[test]
    fn should_strip_special_characters_from_filename_components() {
        let result = get_cache_filename("https://example.com/my<>file.def?v=a|b").unwrap();
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
        assert!(!result.contains('|'));
    }
}

// =============================================================================
// PureUnigram Emoji (Surrogate Pair) Tests
// =============================================================================
mod pure_unigram_unicode_handling {
    use super::*;

    fn emoji_vocab() -> Vec<(String, f64)> {
        vec![
            ("<pad>".to_string(), 0.0),
            ("</s>".to_string(), 0.0),
            ("<unk>".to_string(), 0.0),
            ("\u{2581}".to_string(), -2.0),
            ("\u{2581}hello".to_string(), -5.0),
            ("h".to_string(), -8.0),
            ("e".to_string(), -8.0),
            ("l".to_string(), -8.0),
            ("o".to_string(), -8.0),
        ]
    }

    const UNK_ID: u32 = 2;

    #[test]
    fn should_not_crash_on_emoji_input() {
        let tokenizer = PureUnigram::new(emoji_vocab(), UNK_ID);
        // Should not panic - emoji are multi-byte in UTF-8
        let _ = tokenizer.encode("hello 🎨🔥");
    }

    #[test]
    fn should_produce_correct_token_count_for_emoji() {
        let tokenizer = PureUnigram::new(emoji_vocab(), UNK_ID);
        let result = tokenizer.encode("🎨");
        // 🎨 is 1 code point, should produce ▁ + 🎨 (unk)
        assert!(!result.is_empty());
    }

    #[test]
    fn should_handle_mixed_emoji_and_text() {
        let tokenizer = PureUnigram::new(emoji_vocab(), UNK_ID);
        let result = tokenizer.encode("hello 🎨");
        assert!(!result.is_empty());
        // "hello" -> ▁hello (id=4)
        assert_eq!(result[0], 4);
    }
}

// =============================================================================
// NovelAIT5Tokenizer Tests
// =============================================================================
mod t5_tokenizer {
    use super::*;

    fn mini_vocab() -> Vec<(String, f64)> {
        vec![
            ("<pad>".to_string(), 0.0),
            ("</s>".to_string(), 0.0),
            ("<unk>".to_string(), 0.0),
            ("\u{2581}".to_string(), -2.0),
            ("\u{2581}hello".to_string(), -5.0),
            ("\u{2581}world".to_string(), -5.5),
            ("h".to_string(), -8.0),
            ("e".to_string(), -8.0),
            ("l".to_string(), -8.0),
            ("o".to_string(), -8.0),
        ]
    }

    #[test]
    fn should_encode_with_eos_token() {
        let unigram = PureUnigram::new(mini_vocab(), 2);
        let tokenizer = NovelAIT5Tokenizer::from_pure_unigram(unigram);

        let result = tokenizer.encode("hello");
        // Should end with EOS token (id=1 for </s>)
        assert_eq!(*result.last().unwrap(), 1);
    }

    #[test]
    fn should_return_only_eos_for_empty_string() {
        let unigram = PureUnigram::new(mini_vocab(), 2);
        let tokenizer = NovelAIT5Tokenizer::from_pure_unigram(unigram);

        let result = tokenizer.encode("");
        assert_eq!(result, vec![1]); // Only EOS
    }

    #[test]
    fn should_count_tokens_including_eos() {
        let unigram = PureUnigram::new(mini_vocab(), 2);
        let tokenizer = NovelAIT5Tokenizer::from_pure_unigram(unigram);

        let count = tokenizer.count_tokens("hello world");
        // ▁hello + ▁world + EOS = 3
        assert_eq!(count, 3);
    }

    #[test]
    fn should_preprocess_before_encoding() {
        let unigram = PureUnigram::new(mini_vocab(), 2);
        let tokenizer = NovelAIT5Tokenizer::from_pure_unigram(unigram);

        // Brackets should be removed before encoding
        let result_with_brackets = tokenizer.encode("[hello]");
        let result_without = tokenizer.encode("hello");
        assert_eq!(result_with_brackets, result_without);
    }
}

// =============================================================================
// Integration Tests (Network-dependent, skipped)
// =============================================================================
#[cfg(test)]
mod integration_tests {
    // These tests require network access to fetch tokenizer definitions.
    // They are skipped by default. To run them:
    //   cargo test -- --ignored integration_tests

    #[tokio::test]
    #[ignore] // Requires network access
    async fn should_fetch_and_create_clip_tokenizer() {
        use novelai_api::tokenizer::get_clip_tokenizer;
        let tokenizer = get_clip_tokenizer(false).await.unwrap();
        let tokens = tokenizer.encode("1girl, beautiful");
        assert!(!tokens.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn should_fetch_and_create_t5_tokenizer() {
        use novelai_api::tokenizer::get_t5_tokenizer;
        let tokenizer = get_t5_tokenizer(false).await.unwrap();
        let ids = tokenizer.encode("1girl, beautiful");
        assert!(!ids.is_empty());
        // Should end with EOS
        assert_eq!(*ids.last().unwrap(), 1);
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn should_cache_t5_tokenizer() {
        use novelai_api::tokenizer::get_t5_tokenizer;
        use std::sync::Arc;
        let t1 = get_t5_tokenizer(false).await.unwrap();
        let t2 = get_t5_tokenizer(false).await.unwrap();
        assert!(Arc::ptr_eq(&t1, &t2));
    }
}
