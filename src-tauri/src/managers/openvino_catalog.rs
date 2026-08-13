use super::model::{EngineType, ModelInfo, ModelSource};

struct Spec {
    slug: &'static str,
    name: &'static str,
    revision: &'static str,
    size_mb: u64,
}

const SPECS: &[Spec] = &[
    Spec {
        slug: "distil-whisper-large-v2-fp16-ov",
        name: "Distil-Whisper Large V2 FP16",
        revision: "b20103cebc60bbbda55210d183ce73fc068741e1",
        size_mb: 1523,
    },
    Spec {
        slug: "distil-whisper-large-v2-int4-ov",
        name: "Distil-Whisper Large V2 INT4",
        revision: "666e9bda616be53cf93ca9ff658fc3efd3ddc633",
        size_mb: 450,
    },
    Spec {
        slug: "distil-whisper-large-v2-int8-ov",
        name: "Distil-Whisper Large V2 INT8",
        revision: "cd23bf3655e8ccd72cde9d182f9ffb7a285067d7",
        size_mb: 776,
    },
    Spec {
        slug: "distil-whisper-large-v3-fp16-ov",
        name: "Distil-Whisper Large V3 FP16",
        revision: "147fc406c025905fa774599450c3ca98b72e5671",
        size_mb: 1523,
    },
    Spec {
        slug: "distil-whisper-large-v3-int4-ov",
        name: "Distil-Whisper Large V3 INT4",
        revision: "954b8ce3ca0e1d668d6ec41ea2b03e8420d95158",
        size_mb: 450,
    },
    Spec {
        slug: "distil-whisper-large-v3-int8-ov",
        name: "Distil-Whisper Large V3 INT8",
        revision: "ab5db836c48303e296237013d7385924f2828e9d",
        size_mb: 776,
    },
    Spec {
        slug: "whisper-base-fp16-ov",
        name: "Whisper Base FP16",
        revision: "84fbe975a79a8c996fd32c036558f29e2db6670f",
        size_mb: 155,
    },
    Spec {
        slug: "whisper-base-int4-ov",
        name: "Whisper Base INT4",
        revision: "21b22adb8e49b79dab004804a1b40655a4767c37",
        size_mb: 64,
    },
    Spec {
        slug: "whisper-base-int8-ov",
        name: "Whisper Base INT8",
        revision: "0606293f0511136ada21755a265492f623a934b8",
        size_mb: 85,
    },
    Spec {
        slug: "whisper-base.en-fp16-ov",
        name: "Whisper Base English FP16",
        revision: "51c73092da9828e752decacb460aada348527803",
        size_mb: 155,
    },
    Spec {
        slug: "whisper-base.en-int4-ov",
        name: "Whisper Base English INT4",
        revision: "b0da25f7e43548df7f35fb69fc7609c08242150a",
        size_mb: 64,
    },
    Spec {
        slug: "whisper-base.en-int8-ov",
        name: "Whisper Base English INT8",
        revision: "3b292a83752fbfcad0bd6384bcf71d0b1fc4fe74",
        size_mb: 85,
    },
    Spec {
        slug: "whisper-large-v2-fp16-ov",
        name: "Whisper Large V2 FP16",
        revision: "bb163820f2a615bdf4418b2c6faafd076c24c632",
        size_mb: 3097,
    },
    Spec {
        slug: "whisper-large-v2-int4-ov",
        name: "Whisper Large V2 INT4",
        revision: "51bebb47ea02bc300f7a3b7f7767e18f1b887ba3",
        size_mb: 863,
    },
    Spec {
        slug: "whisper-large-v2-int8-ov",
        name: "Whisper Large V2 INT8",
        revision: "64d610787b33f4cfdb7503642b009903c0a920f8",
        size_mb: 1567,
    },
    Spec {
        slug: "whisper-large-v3-fp16-ov",
        name: "Whisper Large V3 FP16",
        revision: "220761e60602a5ca694c409d5f424563b75d6820",
        size_mb: 3100,
    },
    Spec {
        slug: "whisper-large-v3-int4-ov",
        name: "Whisper Large V3 INT4",
        revision: "95f08bc1b2b53dafaecae3d806b056adecc0be33",
        size_mb: 865,
    },
    Spec {
        slug: "whisper-large-v3-int8-ov",
        name: "Whisper Large V3 INT8",
        revision: "a888a75cc8b494a8a45400fd85f6bfa379ba3955",
        size_mb: 1569,
    },
    Spec {
        slug: "whisper-large-v3-turbo-fp16-ov",
        name: "Whisper Large V3 Turbo FP16",
        revision: "131d663658f94202779b0bb98ee7a5f71d5bde1a",
        size_mb: 1628,
    },
    Spec {
        slug: "whisper-large-v3-turbo-int4-ov",
        name: "Whisper Large V3 Turbo INT4",
        revision: "ae50b4d9a9dbaf16f2df59c23f3984e42f864dfc",
        size_mb: 478,
    },
    Spec {
        slug: "whisper-large-v3-turbo-int8-ov",
        name: "Whisper Large V3 Turbo INT8",
        revision: "4929ae83ea2d1df59f4b5898a9aab8aa1c29e711",
        size_mb: 829,
    },
    Spec {
        slug: "whisper-medium-fp16-ov",
        name: "Whisper Medium FP16",
        revision: "4508616c9c0774807e7d315c26cec49dcfe1f0a8",
        size_mb: 1539,
    },
    Spec {
        slug: "whisper-medium-int4-ov",
        name: "Whisper Medium INT4",
        revision: "14bba652dc6604717bf1cbdf358645d414548522",
        size_mb: 447,
    },
    Spec {
        slug: "whisper-medium-int8-ov",
        name: "Whisper Medium INT8",
        revision: "8d43cce846729381f56bd45a1c70925cee2222ff",
        size_mb: 785,
    },
    Spec {
        slug: "whisper-medium.en-fp16-ov",
        name: "Whisper Medium English FP16",
        revision: "6d58674d6e92d11b724ba93f03be179a6db0d281",
        size_mb: 1539,
    },
    Spec {
        slug: "whisper-medium.en-int4-ov",
        name: "Whisper Medium English INT4",
        revision: "d9d0cb981105544237fea92524f2a34a7c677ab5",
        size_mb: 447,
    },
    Spec {
        slug: "whisper-medium.en-int8-ov",
        name: "Whisper Medium English INT8",
        revision: "6d1cdfd1b6065e651ed9fa6f9bbaf09a8538ea11",
        size_mb: 784,
    },
    Spec {
        slug: "whisper-small-fp16-ov",
        name: "Whisper Small FP16",
        revision: "2410d022171ca8a97343182f88eec8807a324db9",
        size_mb: 494,
    },
    Spec {
        slug: "whisper-small-int4-ov",
        name: "Whisper Small INT4",
        revision: "60d873403eb252f1de6b895833feaadfa64ea4cd",
        size_mb: 163,
    },
    Spec {
        slug: "whisper-small-int8-ov",
        name: "Whisper Small INT8",
        revision: "8c593cfd15717c025c5ef99ea0b1879c88d03ed2",
        size_mb: 257,
    },
    Spec {
        slug: "whisper-small.en-fp16-ov",
        name: "Whisper Small English FP16",
        revision: "12c7b47492597de0a3b5b552cdb0e62c81d3679a",
        size_mb: 494,
    },
    Spec {
        slug: "whisper-small.en-int4-ov",
        name: "Whisper Small English INT4",
        revision: "00df8882e77a1f0358fcfccf16ee1b33594dd401",
        size_mb: 163,
    },
    Spec {
        slug: "whisper-small.en-int8-ov",
        name: "Whisper Small English INT8",
        revision: "cc80452e452a46978219a7069ed96a6e47cabe44",
        size_mb: 257,
    },
    Spec {
        slug: "whisper-tiny-fp16-ov",
        name: "Whisper Tiny FP16",
        revision: "44662d68573bd732e50fc295d1c1ec47e77f67df",
        size_mb: 85,
    },
    Spec {
        slug: "whisper-tiny-int4-ov",
        name: "Whisper Tiny INT4",
        revision: "2a7888b86d3b30c07b88a945ec6e5f2b6a98c913",
        size_mb: 42,
    },
    Spec {
        slug: "whisper-tiny-int8-ov",
        name: "Whisper Tiny INT8",
        revision: "a850762d97243dee30f46ca309720541af619ab0",
        size_mb: 49,
    },
    Spec {
        slug: "whisper-tiny.en-fp16-ov",
        name: "Whisper Tiny English FP16",
        revision: "82b00e6b7ce190802f654f891257ccdbc50faadb",
        size_mb: 85,
    },
    Spec {
        slug: "whisper-tiny.en-int4-ov",
        name: "Whisper Tiny English INT4",
        revision: "2c4a4cb35a33f827f324c55f474d9197c586b485",
        size_mb: 41,
    },
    Spec {
        slug: "whisper-tiny.en-int8-ov",
        name: "Whisper Tiny English INT8",
        revision: "5b32d1ceafd990018579cb0501ee0623865feb8f",
        size_mb: 49,
    },
];

pub fn models(multilingual_languages: &[String]) -> Vec<ModelInfo> {
    let mut models: Vec<ModelInfo> = SPECS
        .iter()
        .map(|spec| {
            let english_only = spec.slug.contains(".en-") || spec.slug.starts_with("distil-");
            let int4 = spec.slug.contains("-int4-");
            let turbo = spec.slug.contains("turbo");
            let small = spec.slug.contains("small")
                || spec.slug.contains("base")
                || spec.slug.contains("tiny");
            ModelInfo {
                id: format!("openvino-{}", spec.slug.trim_end_matches("-ov")),
                name: format!("{} (Intel NPU)", spec.name),
                description: format!(
                    "Official pinned OpenVINO {} model for Intel NPU.",
                    if english_only {
                        "English"
                    } else {
                        "multilingual"
                    }
                ),
                filename: format!("openvino-{}", spec.slug.trim_end_matches("-ov")),
                source: ModelSource::OpenVinoSnapshot {
                    repo_id: format!("OpenVINO/{}", spec.slug),
                    revision: spec.revision.to_string(),
                },
                size_mb: spec.size_mb,
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: true,
                engine_type: EngineType::OpenVinoNpu,
                accuracy_score: if spec.slug.contains("large-v3") {
                    0.98
                } else if spec.slug.contains("large") {
                    0.94
                } else if spec.slug.contains("medium") {
                    0.88
                } else {
                    0.78
                },
                speed_score: if turbo {
                    0.98
                } else if small {
                    0.95
                } else if int4 {
                    0.90
                } else {
                    0.80
                },
                supports_translation: !english_only,
                is_recommended: spec.slug == "whisper-large-v3-turbo-int8-ov",
                supported_languages: if english_only {
                    vec!["en".to_string()]
                } else {
                    multilingual_languages.to_vec()
                },
                supports_language_selection: !english_only,
                is_custom: false,
                supports_streaming: false,
                supports_language_detection: !english_only,
            }
        })
        .collect();

    models.push(ModelInfo {
        id: "openvino-parakeet-tdt-v3".to_string(),
        name: "Parakeet TDT V3 (Intel NPU)".to_string(),
        description: "Fast multilingual Parakeet transcription, independently verified on Intel NPU by Handy.".to_string(),
        filename: "openvino-parakeet-tdt-v3".to_string(),
        source: ModelSource::OpenVinoSnapshot {
            repo_id: "FluidInference/parakeet-tdt-0.6b-v3-ov".to_string(),
            revision: "dfd55eb6c85a9a8546a162bed84784245d5743c2".to_string(),
        },
        size_mb: 1225,
        is_downloaded: false,
        is_downloading: false,
        partial_size: 0,
        is_directory: true,
        engine_type: EngineType::OpenVinoNpu,
        accuracy_score: 0.95,
        speed_score: 0.99,
        supports_translation: false,
        is_recommended: false,
        supported_languages: [
            "en", "es", "it", "fr", "de", "nl", "ru", "pl", "uk", "sk", "bg", "fi",
            "ro", "hr", "cs", "sv", "et", "hu", "lt", "da", "mt", "sl", "lv", "el",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        supports_language_selection: false,
        is_custom: false,
        supports_streaming: false,
        supports_language_detection: true,
    });

    for (precision, size_mb, accuracy, speed, recommended) in [
        ("int8", 2215, 0.98, 0.88, true),
        ("int4", 1330, 0.94, 0.94, false),
    ] {
        models.push(ModelInfo {
            id: format!("openvino-qwen3-asr-1.7b-{precision}"),
            name: format!("Qwen3-ASR 1.7B {} (Intel NPU)", precision.to_uppercase()),
            description: if precision == "int8" {
                "High-accuracy multilingual Qwen3-ASR using Handy's NPU-native decoder. Recommended Qwen precision."
            } else {
                "Smaller multilingual Qwen3-ASR using Handy's NPU-native INT4 decoder. Faster and leaner, with some accuracy tradeoff."
            }
            .to_string(),
            filename: format!("handy-qwen3-asr-1.7b-{precision}-npu"),
            source: ModelSource::OpenVinoSnapshot {
                repo_id: format!(
                    "iamwaqargulzar/handy-qwen3-asr-1.7b-openvino-npu-{precision}"
                ),
                revision: "main".to_string(),
            },
            size_mb,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::OpenVinoNpu,
            accuracy_score: accuracy,
            speed_score: speed,
            supports_translation: false,
            is_recommended: recommended,
            supported_languages: multilingual_languages.to_vec(),
            supports_language_selection: true,
            is_custom: false,
            supports_streaming: false,
            supports_language_detection: true,
        });
    }

    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_catalog_is_large_and_unique() {
        let models = models(&["en".into(), "ur".into()]);
        assert_eq!(models.len(), 42);
        let ids: std::collections::HashSet<_> = models.iter().map(|m| &m.id).collect();
        assert_eq!(ids.len(), models.len());
        assert!(models
            .iter()
            .all(|m| matches!(m.engine_type, EngineType::OpenVinoNpu)));
    }
}
