import sys
import json
import zlib
import html
import requests
import regex as re
import argparse

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

    def encode(self, text, mode="clip"):
        """
        mode:
          "clip" - follows the 'u' class logic (standard CLIP).
          "t5"   - follows the 'p' class logic (NovelAI V3/T5), which removes `2::` syntax.
        """

        # Preprocessing based on mode
        if mode == "t5":
            # JS (p class): e.replace(/[[\]{}]/g,"").replace(/-?\d*\.?\d*::/g,"")
            # Remove brackets completely
            text = re.sub(r"[[\]{}]", "", text)
            # Remove weighting syntax like "2::" or "0.5::"
            text = re.sub(r"-?\d*\.?\d*::", "", text)
        else:
            # JS (u class): e.replace(/[[\]{}]/g," ").trim()
            # Replace brackets with space
            text = re.sub(r"[[\]{}]", " ", text).strip()

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

def fetch_and_load_tokenizer(url="https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true"):
    print(f"Fetching tokenizer definition from {url}...")
    headers = {"User-Agent": "Mozilla/5.0"}
    resp = requests.get(url, headers=headers)
    resp.raise_for_status()

    print("Decompressing data...")
    try:
        data = zlib.decompress(resp.content, -15)
    except zlib.error:
        data = zlib.decompress(resp.content)

    print("Parsing JSON...")
    data_str = data.decode("utf-8")
    json_data = json.loads(data_str)

    return NovelAIClipTokenizer(json_data["text"])

def main():
    parser = argparse.ArgumentParser(description="NovelAI Tokenizer Client")
    parser.add_argument("text", nargs="*", help="Text to tokenize")
    parser.add_argument("--mode", choices=["clip", "t5"], default="clip", help="Tokenizer preprocessing mode. 'clip' for standard behavior, 't5' for V3/T5 behavior (removes '2::' syntax)")
    args = parser.parse_args()

    if args.text:
        text = " ".join(args.text)
    else:
        text = input("Enter text to tokenize: ")

    try:
        tokenizer = fetch_and_load_tokenizer()

        print(f"Mode: {args.mode}")
        tokens = tokenizer.encode(text, mode=args.mode)
        print(f"\nToken IDs: {tokens}")
        print(f"Token Count: {len(tokens)}")

    except Exception as e:
        print(f"Error: {e}")
        # import traceback
        # traceback.print_exc()

if __name__ == "__main__":
    main()
