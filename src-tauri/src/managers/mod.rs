pub mod audio;
pub mod gguf_meta;
pub mod history;
pub mod model;
pub mod model_capabilities;
#[cfg(target_os = "linux")]
pub mod openvino_npu;
pub mod transcription;
