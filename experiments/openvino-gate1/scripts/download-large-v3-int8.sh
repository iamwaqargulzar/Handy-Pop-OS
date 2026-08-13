#!/usr/bin/env bash
set -euo pipefail

readonly revision="a888a75cc8b494a8a45400fd85f6bfa379ba3955"
readonly repository="OpenVINO/whisper-large-v3-int8-ov"
readonly destination="${1:-models/whisper-large-v3-int8-ov}"

readonly files=(
  added_tokens.json
  config.json
  generation_config.json
  merges.txt
  normalizer.json
  openvino_config.json
  openvino_decoder_model.bin
  openvino_decoder_model.xml
  openvino_detokenizer.bin
  openvino_detokenizer.xml
  openvino_encoder_model.bin
  openvino_encoder_model.xml
  openvino_tokenizer.bin
  openvino_tokenizer.xml
  preprocessor_config.json
  special_tokens_map.json
  tokenizer.json
  tokenizer_config.json
  vocab.json
)

mkdir -p "$destination"
for file in "${files[@]}"; do
  curl --fail --location --continue-at - \
    "https://huggingface.co/${repository}/resolve/${revision}/${file}" \
    --output "${destination}/${file}"
done

printf '%s  %s\n' \
  "fdf13685d5a9c427b9aa5893c2baef362f1e4dddfbf5bf8a47fc03acb35a45ea" \
  "${destination}/openvino_decoder_model.bin" \
  "fffcbf47a4cfd5a1e3f57c0569f5ef706245b798ef626db5ffea7b84166ed865" \
  "${destination}/openvino_encoder_model.bin" \
  "f2b3c47825a1089525ff65c0c8e49271e1dee69a401a04fc827ac2de5b7766e4" \
  "${destination}/openvino_detokenizer.bin" \
  "adfa3d9a2920d0f314121270a960ab331ec0f05838544bb8ecaaa422282a6fd4" \
  "${destination}/openvino_tokenizer.bin" | sha256sum --check

(
  cd "$destination"
  sha256sum "${files[@]}" > SHA256SUMS
)

printf 'Downloaded pinned model revision %s to %s\n' "$revision" "$destination"

