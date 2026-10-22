#!/usr/bin/env python3
"""Spell wake words with the detector model's own BPE vocabulary.

The sherpa-onnx keyword spotter matches pre-tokenized keywords, so a new wake
word has to be written the way the model's SentencePiece model spells it.

Usage: tokenize-keywords.py <bpe.model> <name> [<name> ...]

Prints one pre-tokenized keyword line per name, in the order given. Requires
sentencepiece: python3 -m pip install sentencepiece
"""

import sys


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        sys.stderr.write(f"usage: {argv[0]} <bpe.model> <name> [<name> ...]\n")
        return 2
    try:
        import sentencepiece as spm
    except ImportError:
        sys.stderr.write(
            "sentencepiece is not installed. Install it with: python3 -m pip install sentencepiece\n"
        )
        return 2

    model_path, names = argv[1], argv[2:]
    processor = spm.SentencePieceProcessor(model_file=model_path)
    if not processor:
        sys.stderr.write(f"could not load the SentencePiece model: {model_path}\n")
        return 2

    for name in names:
        tokens = processor.encode(name, out_type=str)
        if not tokens:
            sys.stderr.write(f"no tokens for: {name}\n")
            return 1
        print(" ".join(tokens))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
