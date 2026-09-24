# OpenVINO model conversion

These scripts document reproducible build-time conversion work. They are not
included in Handy's runtime package and their Python dependencies must not be
added to the application.

`export_qwen3_asr_npu.py` converts an official Optimum Intel Qwen3-ASR export
into Handy's NPU-native split format. It preserves the official speech encoder
and ASR-trained language-model weights, extracts the prompt/audio embedding
merge for CPU, and exports the stateful causal decoder contract required by
OpenVINO NPUW. Both INT8 and INT4 are supported.

Every generated model must still pass a native worker load, an
`actual_device=NPU` check, and reference-audio transcription before it is
published or added to `openvino_catalog.rs`.
