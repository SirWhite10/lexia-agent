#!/usr/bin/env bash
set -euo pipefail

model_name='sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01'
model_dir="models/${model_name}"
archive="models/${model_name}.tar.bz2"

mkdir -p models assets
if [[ ! -d "$model_dir" ]]; then
	url="https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/${model_name}.tar.bz2"
	curl --fail --location --output "$archive" "$url"
	tar -xjf "$archive" -C models
	rm "$archive"
fi

printf '\nPrepared model at %s; the bundled token list listens for “LEXIA”.\n' "$model_dir"
