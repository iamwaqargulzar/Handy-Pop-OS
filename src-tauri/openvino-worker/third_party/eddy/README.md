# Eddy Parakeet integration

This directory contains the minimal Apache-2.0-licensed Parakeet/OpenVINO
implementation vendored from
[`FluidInference/eddy`](https://github.com/FluidInference/eddy) at commit
`07028cf333f97244f0f3ff718cc748d7dd0a8915`.

Only the backend, Parakeet TDT pipeline, tokenizer, and required utility files
are retained. Handy deliberately changes Eddy's NPU-to-CPU fallback into a
hard error: a model presented as Intel NPU-capable must not silently execute
its encoder, decoder, or joint graph on CPU. Mel-spectrogram preprocessing
continues to run on CPU as designed by the upstream implementation.

The original license is preserved in `LICENSE`.
