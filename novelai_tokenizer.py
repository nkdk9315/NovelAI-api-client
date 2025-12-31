import sys
import json
import zlib
import html
import requests
import regex as re
import argparse
import os

try:
    from tokenizers import Tokenizer
except ImportError:
    Tokenizer = None

# The initial vocabulary list from 9423.2de67be589ffa59d.js
INITIAL_VOCAB = [
    "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<", "=", ">", "?", "@", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "^", "_", "`", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "{", "|", "}", "~", "\xa1", "\xa2", "\xa3", "\xa4", "\xa5", "\xa6", "\xa7", "\xa8", "\xa9", "\xaa", "\xab", "\xac", "\xae", "\xaf", "\xb0", "\xb1", "\xb2", "\xb3", "\xb4", "\xb5", "\xb6", "\xb7", "\xb8", "\xb9", "\xba", "\xbb", "\xbc", "\xbd", "\xbe", "\xbf", "\xc0", "\xc1", "\xc2", "\xc3", "\xc4", "\xc5", "\xc6", "\xc7", "\xc8", "\xc9", "\xca", "\xcb", "\xcc", "\xcd", "\xce", "\xcf", "\xd0", "\xd1", "\xd2", "\xd3", "\xd4", "\xd5", "\xd6", "\xd7", "\xd8", "\xd9", "\xda", "\xdb", "\xdc", "\xdd", "\xde", "\xdf", "\xe0", "\xe1", "\xe2", "\xe3", "\xe4", "\xe5", "\xe6", "\xe7", "\xe8", "\xe9", "\xea", "\xeb", "\xec", "\xed", "\xee", "\xef", "\xf0", "\xf1", "\xf2", "\xf3", "\xf4", "\xf5", "\xf6", "\xf7", "\xf8", "\xf9", "\xfa", "\xfb", "\xfc", "\xfd", "\xfe", "\xff", "Ā", "ā", "Ă", "ă", "Ą", "ą", "Ć", "ć", "Ĉ", "ĉ", "Ċ", "ċ", "Č", "č", "Ď", "ď", "Đ", "đ", "Ē", "ē", "Ĕ", "ĕ", "Ė", "ė", "Ę", "ę", "Ě", "ě", "Ĝ", "ĝ", "Ğ", "ğ", "Ġ", "ġ", "Ģ", "ģ", "Ĥ", "ĥ", "Ħ", "ħ", "Ĩ", "ĩ", "Ī", "ī", "Ĭ", "ĭ", "Į", "į", "İ", "ı", "Ĳ", "ĳ", "Ĵ", "ĵ", "Ķ", "ķ", "ĸ", "Ĺ", "ĺ", "Ļ", "ļ", "Ľ", "ľ", "Ŀ", "ŀ", "Ł", "ł", "Ń"
]

def bytes_to_unicode():
    bs = list(range(ord("!"), ord("~")+1)) + list(range(ord("¡"), ord("¬")+1)) + list(range(ord("®"), ord("ÿ")+1))
    cs = bs[:]
    n = 0
    for b in range(2**8):
        if b not in bs:
            bs.append(b)
            cs.append(2**8 + n)
            n += 1
    cs = [chr(n) for n in cs]
    return dict(zip(bs, cs))

class NovelAIClipTokenizer:
    def __init__(self, definition_text):
        self.byte_encoder = bytes_to_unicode()

        lines = definition_text.split('\n')
        merges_raw = lines[1:48895]
        merges = [line.split(" ") for line in merges_raw]

        vocab_list = list(INITIAL_VOCAB)
        vocab_list.extend([token + "</w>" for token in INITIAL_VOCAB])

        for merge_pair in merges:
            vocab_list.append("".join(merge_pair))

        vocab_list.append("<|startoftext|>")
        vocab_list.append("<|endoftext|>")

        self.encoder = {token: i for i, token in enumerate(vocab_list)}
        self.decoder = {i: token for token, i in self.encoder.items()}

        separator = "\xb7\U0001F60E\xb7"
        self.bpe_ranks = {separator.join(pair): i for i, pair in enumerate(merges)}

        self.cache = {
            "<|startoftext|>": "<|startoftext|>",
            "<|endoftext|>": "<|endoftext|>"
        }

        self.pat = re.compile(r"""<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+""", re.IGNORECASE | re.UNICODE)

    def bpe(self, token):
        if token in self.cache:
            return self.cache[token]

        word = list(token[:-1]) + [token[-1] + "</w>"]

        pairs = self.get_pairs(word)
        if not pairs:
            return token + "</w>"

        while True:
            bigram = min(pairs, key=lambda pair: self.bpe_ranks.get("\xb7\U0001F60E\xb7".join(pair), float("inf")))

            separator = "\xb7\U0001F60E\xb7"
            if separator.join(bigram) not in self.bpe_ranks:
                break

            first, second = bigram
            new_word = []
            i = 0
            while i < len(word):
                try:
                    j = word.index(first, i)
                    new_word.extend(word[i:j])
                    i = j
                except ValueError:
                    new_word.extend(word[i:])
                    break

                if word[i] == first and i < len(word) - 1 and word[i+1] == second:
                    new_word.append(first + second)
                    i += 2
                else:
                    new_word.append(word[i])
                    i += 1

            word = new_word
            if len(word) == 1:
                break
            pairs = self.get_pairs(word)

        result = " ".join(word)
        self.cache[token] = result
        return result

    def get_pairs(self, word):
        pairs = set()
        prev_char = word[0]
        for char in word[1:]:
            pairs.add((prev_char, char))
            prev_char = char
        return pairs

    def encode(self, text):
        # JS: (t=(r=this.htmlEntities).decode(r.decode(t))).trim()
        text = html.unescape(html.unescape(text)).strip()
        # JS: .replace(/\s+/g," ").trim().toLowerCase()
        text = re.sub(r'\s+', ' ', text).strip().lower()

        bpe_tokens = []

        for match in self.pat.findall(text):
            token = match
            token_bytes = token.encode("utf-8")
            token_translated = "".join([self.byte_encoder[b] for b in token_bytes])

            bpe_res = self.bpe(token_translated)
            for bpe_token in bpe_res.split(" "):
                if bpe_token in self.encoder:
                    bpe_tokens.append(self.encoder[bpe_token])

        return bpe_tokens

def fetch_data(url):
    print(f"Fetching {url}...")
    headers = {"User-Agent": "Mozilla/5.0"}
    resp = requests.get(url, headers=headers)
    resp.raise_for_status()
    print("Decompressing data...")
    try:
        data = zlib.decompress(resp.content, -15)
    except zlib.error:
        data = zlib.decompress(resp.content)
    return data.decode("utf-8")

def get_clip_tokenizer():
    url = "https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true"
    data_str = fetch_data(url)
    json_data = json.loads(data_str)
    return NovelAIClipTokenizer(json_data["text"])

def get_t5_tokenizer():
    if Tokenizer is None:
        raise ImportError("The 'tokenizers' library is required for T5 mode. Please install it with `pip install tokenizers`.")

    url = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true"
    # Check for local cache to avoid redownloading
    cache_path = "t5_tokenizer.json"
    if os.path.exists(cache_path):
        return Tokenizer.from_file(cache_path)

    data_str = fetch_data(url)
    # Validate JSON
    json.loads(data_str)

    with open(cache_path, "w") as f:
        f.write(data_str)

    return Tokenizer.from_str(data_str)

def preprocess_t5(text):
    """
    NovelAI Frontend T5 Preprocessing Logic:
    1. Remove brackets [] and {} (used for old weighting or other syntax)
    2. Remove new weighting syntax (e.g., 2::, -0.5::)
    3. HTML unescape
    4. Normalize whitespace and lowercase
    """
    # 1. Remove brackets
    text = re.sub(r"[[\]{}]", "", text)

    # 2. Remove weighting syntax
    text = re.sub(r"-?\d*\.?\d*::", "", text)

    # 3. HTML unescape
    text = html.unescape(html.unescape(text)).strip()

    # 4. Whitespace and lowercase
    text = re.sub(r'\s+', ' ', text).strip().lower()

    return text

def main():
    parser = argparse.ArgumentParser(description="NovelAI Tokenizer Client")
    parser.add_argument("text", nargs="*", help="Text to tokenize")
    args = parser.parse_args()

    if args.text:
        text = " ".join(args.text)
    else:
        print("Please provide text to tokenize.")
        return

    print(f"\nProcessing Text: {text[:50]}..." if len(text) > 50 else f"\nProcessing Text: {text}")

    # --- CLIP Mode (Raw) ---
    try:
        clip_tokenizer = get_clip_tokenizer()
        # For CLIP, we tokenize the RAW text (NovelAI seems to count weights as tokens in the 'Raw' view)
        clip_tokens = clip_tokenizer.encode(text)
        print(f"\n[Raw Token Count] (CLIP, includes weights): {len(clip_tokens)}")
        # print(f"IDs: {clip_tokens}")
    except Exception as e:
        print(f"Error loading CLIP tokenizer: {e}")

    # --- T5 Mode (Effective) ---
    try:
        t5_tokenizer = get_t5_tokenizer()

        # Preprocess
        processed_text = preprocess_t5(text)

        # Add Quality Tags (NovelAI adds these by default for V3)
        # Note: If the prompt already has them, this might duplicate, but usually the UI adds them hiddenly.
        # "masterpiece, best quality, "
        text_with_tags = "masterpiece, best quality, " + processed_text

        encoded = t5_tokenizer.encode(text_with_tags)

        # Calculate count: T5 adds EOS (</s>, id=1) at the end.
        # The official "Used Tokens" count typically excludes this or matches the 'effective' prompt tokens.
        # Based on analysis, the official count is (Total T5 Tokens - 1).

        count = len(encoded.ids)
        if encoded.ids[-1] == 1: # EOS token check
            count -= 1

        print(f"\n[Effective Token Count] (T5, weights removed + quality tags): {count}")
        # print(f"IDs: {encoded.ids}")

    except ImportError as e:
        print(f"\n[Effective Token Count] Unavailable: {e}")
    except Exception as e:
        print(f"\nError loading T5 tokenizer: {e}")

if __name__ == "__main__":
    main()
