"""Build Handy's NPU-native Qwen3-ASR model directory.

This is a release-engineering tool, not an application runtime dependency.
Run it in an isolated conversion environment containing PyTorch, qwen-asr,
Optimum Intel, NNCF, and the OpenVINO version used by the Handy release.
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

import nncf
import openvino as ov
import qwen_asr  # noqa: F401 - registers Qwen3-ASR with Transformers
import torch
from optimum.exporters.openvino.convert import export_models
from optimum.exporters.openvino.model_configs import get_vlm_text_generation_config
from transformers import AutoModel, Qwen3Config, Qwen3ForCausalLM
from transformers.models.qwen3.modeling_qwen3 import Qwen3RotaryEmbedding


def export_standard_decoder(source: Path, output: Path) -> None:
    full = AutoModel.from_pretrained(
        source,
        local_files_only=True,
        trust_remote_code=True,
        dtype=torch.float32,
        low_cpu_mem_usage=True,
    )
    config_dict = full.thinker.config.text_config.to_dict()
    config_dict["model_type"] = "qwen3"
    config_dict["rope_scaling"] = None
    text_config = Qwen3Config.from_dict(config_dict)
    with torch.device("meta"):
        decoder = Qwen3ForCausalLM(text_config)
    state = {"model." + key: value for key, value in full.thinker.model.state_dict().items()}
    state["lm_head.weight"] = full.thinker.lm_head.weight
    missing, unexpected = decoder.load_state_dict(state, strict=False, assign=True)
    if missing or unexpected:
        raise RuntimeError(f"weight mapping mismatch: missing={missing}, unexpected={unexpected}")
    decoder.model.rotary_emb = Qwen3RotaryEmbedding(text_config)
    decoder.eval()
    export_config = get_vlm_text_generation_config(
        "qwen3", text_config, "int64", "fp32", task="text-generation-with-past"
    )
    output.mkdir(parents=True, exist_ok=True)
    export_models(
        {"decoder": (decoder, export_config)},
        output,
        output_names=["openvino_decoder_model.xml"],
        stateful=True,
        library_name="transformers",
        input_shapes={"batch_size": 1, "sequence_length": 4},
    )


def extract_prompt_embeddings(official_export: Path, hidden_size: int) -> ov.Model:
    model = ov.Core().read_model(official_export / "openvino_decoder_model.xml")
    encoder_input = next(port for port in model.inputs if port.any_name == "encoder_hidden_states")
    for node in list(model.get_ordered_ops()):
        if node.get_type_name() == "ReadValue" and "encoder_hidden_states" in node.get_variable_id():
            node.output(0).replace(encoder_input)
        if node.get_type_name() == "Assign" and "encoder_hidden_states" in node.get_variable_id():
            model.remove_sink(node)
    model.validate_nodes_and_infer_types()
    merge = next(
        node
        for node in model.get_ordered_ops()
        if node.get_type_name() == "Select"
        and str(node.output(0).partial_shape) == f"[?,?,{hidden_size}]"
    )
    return ov.Model(
        [merge.output(0)],
        [
            port.get_node()
            for port in model.inputs
            if port.any_name in ("input_ids", "encoder_hidden_states")
        ],
        "qwen_asr_prompt_embeddings",
    )


def build(args: argparse.Namespace) -> None:
    decoder_dir = args.work_dir / "standard-decoder"
    export_standard_decoder(args.source, decoder_dir)
    embeddings = extract_prompt_embeddings(args.official_export, args.hidden_size)

    modes = {
        "int8": (nncf.CompressWeightsMode.INT8_SYM, {}),
        "int4": (
            nncf.CompressWeightsMode.INT4_SYM,
            {"group_size": 128, "ratio": 1.0, "all_layers": True},
        ),
    }
    for precision in args.precision:
        mode, options = modes[precision]
        decoder = nncf.compress_weights(
            ov.Core().read_model(decoder_dir / "openvino_decoder_model.xml"),
            mode=mode,
            **options,
        )
        target = args.output / f"handy-qwen3-asr-{args.model_size}-{precision}-npu"
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(args.official_export, target)
        ov.save_model(decoder, target / "openvino_decoder_model.xml", compress_to_fp16=False)
        ov.save_model(embeddings, target / "openvino_embeddings_model.xml", compress_to_fp16=False)
        marker = {
            "format": 2,
            "decoder_precision": f"{precision}_sym",
            "max_prompt_tokens": 1024,
            "hidden_size": args.hidden_size,
            "feature_size": 128,
            "encoder_chunk_frames": 100,
            "encoder_tokens_per_chunk": 13,
        }
        (target / "handy_qwen_npu.json").write_text(json.dumps(marker, indent=2) + "\n")
        print(target)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True, help="local Hugging Face snapshot")
    parser.add_argument("--official-export", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--model-size", choices=("0.6b", "1.7b"), required=True)
    parser.add_argument("--hidden-size", type=int, choices=(1024, 2048), required=True)
    parser.add_argument("--precision", choices=("int8", "int4"), action="append", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    build(parse_args())
