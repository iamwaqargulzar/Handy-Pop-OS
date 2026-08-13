//! Isolated OpenVINO GenAI NPU worker client.
//!
//! OpenVINO is deliberately kept out of the Handy process. The native worker
//! owns the runtime and model; Handy communicates over an owner-only Unix
//! socket using the bounded protocol implemented in `openvino-worker/main.cpp`.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const PROTOCOL_VERSION: u32 = 1;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const WORKER_START_TIMEOUT: Duration = Duration::from_secs(10);

pub struct OpenVinoNpuEngine {
    child: Child,
    socket_path: PathBuf,
}

impl OpenVinoNpuEngine {
    pub fn load(model_path: &Path) -> Result<Self> {
        let worker_path = worker_path()?;
        let socket_path = socket_path();
        remove_socket(&socket_path);

        let mut command = Command::new(&worker_path);
        let library_path = private_library_path(&worker_path);
        command
            .arg(&socket_path)
            // OpenVINO 2026.3's compatibility allocation path is the verified
            // route for full Large V3 on the tested Lunar Lake driver.
            .env("DISABLE_OPENVINO_GENAI_NPU_L0", "1")
            .env("LD_LIBRARY_PATH", library_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());

        let child = command
            .spawn()
            .with_context(|| format!("failed to start {}", worker_path.display()))?;
        let mut engine = Self { child, socket_path };
        engine.wait_until_ready()?;

        let probe = engine.request(json!({"command": "probe"}), &[])?;
        if probe
            .pointer("/probe/npu_available")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(anyhow!("OpenVINO worker did not find a usable Intel NPU"));
        }

        let loaded = engine.request(
            json!({
                "command": "load_model",
                "model_path": model_path.to_string_lossy(),
            }),
            &[],
        )?;
        if loaded
            .pointer("/loaded/actual_device")
            .and_then(Value::as_str)
            != Some("NPU")
        {
            return Err(anyhow!("OpenVINO model did not load on the Intel NPU"));
        }
        Ok(engine)
    }

    pub fn transcribe(
        &mut self,
        audio: &[f32],
        language: &str,
        translate_to_english: bool,
    ) -> Result<String> {
        let language = if language == "auto" {
            "auto".to_string()
        } else {
            format!("<|{}|>", language)
        };
        // f32 audio is native little-endian on supported Linux x86_64 builds.
        let payload = unsafe {
            std::slice::from_raw_parts(audio.as_ptr().cast::<u8>(), std::mem::size_of_val(audio))
        };
        let response = self.request(
            json!({
                "command": "transcribe",
                "language": language,
                "task": if translate_to_english { "translate" } else { "transcribe" },
                "payload_bytes": payload.len(),
            }),
            payload,
        )?;
        response
            .pointer("/transcription/text")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("OpenVINO worker returned no transcription text"))
    }

    fn wait_until_ready(&mut self) -> Result<()> {
        let started = Instant::now();
        while started.elapsed() < WORKER_START_TIMEOUT {
            if let Some(status) = self.child.try_wait()? {
                return Err(anyhow!("OpenVINO worker exited during startup: {status}"));
            }
            if self.socket_path.exists() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(25));
        }
        Err(anyhow!("timed out waiting for the OpenVINO worker"))
    }

    fn request(&self, mut request: Value, payload: &[u8]) -> Result<Value> {
        request["protocol_version"] = json!(PROTOCOL_VERSION);
        let body = serde_json::to_vec(&request)?;
        let body_len = u32::try_from(body.len()).context("OpenVINO request is too large")?;
        let mut stream = UnixStream::connect(&self.socket_path)
            .context("failed to connect to the OpenVINO worker")?;
        stream.write_all(&body_len.to_be_bytes())?;
        stream.write_all(&body)?;
        stream.write_all(payload)?;

        let mut length = [0_u8; 4];
        stream.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length) as usize;
        if length == 0 || length > MAX_RESPONSE_BYTES {
            return Err(anyhow!("invalid OpenVINO worker response length: {length}"));
        }
        let mut response = vec![0_u8; length];
        stream.read_exact(&mut response)?;
        let response: Value = serde_json::from_slice(&response)?;
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            let code = response
                .pointer("/error/code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = response
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("OpenVINO worker error");
            return Err(anyhow!("{code}: {message}"));
        }
        Ok(response)
    }
}

impl Drop for OpenVinoNpuEngine {
    fn drop(&mut self) {
        let _ = self.request(json!({"command": "shutdown"}), &[]);
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                remove_socket(&self.socket_path);
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        remove_socket(&self.socket_path);
    }
}

pub fn probe_available() -> bool {
    let worker = match worker_path() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let socket = socket_path();
    let library_path = private_library_path(&worker);
    remove_socket(&socket);
    let mut child = match Command::new(worker)
        .arg(&socket)
        .env("DISABLE_OPENVINO_GENAI_NPU_L0", "1")
        .env("LD_LIBRARY_PATH", library_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let started = Instant::now();
    while started.elapsed() < WORKER_START_TIMEOUT && !socket.exists() {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let available = UnixStream::connect(&socket)
        .and_then(|mut stream| {
            let body = serde_json::to_vec(&json!({
                "protocol_version": PROTOCOL_VERSION,
                "command": "probe",
            }))
            .map_err(std::io::Error::other)?;
            stream.write_all(&(body.len() as u32).to_be_bytes())?;
            stream.write_all(&body)?;
            let mut length = [0_u8; 4];
            stream.read_exact(&mut length)?;
            let mut response = vec![0_u8; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut response)?;
            Ok(response)
        })
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| {
            value
                .pointer("/probe/npu_available")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    let _ = child.kill();
    let _ = child.wait();
    remove_socket(&socket);
    available
}

fn worker_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("HANDY_OPENVINO_WORKER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    let executable = std::env::current_exe()?;
    let installed = executable
        .parent()
        .and_then(Path::parent)
        .map(|prefix| prefix.join("lib/Handy/handy-openvino-npu"));
    if let Some(path) = installed.filter(|path| path.is_file()) {
        return Ok(path);
    }
    Err(anyhow!("OpenVINO NPU worker is not installed"))
}

fn socket_path() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    runtime.join(format!("handy-openvino-{}.sock", std::process::id()))
}

fn private_library_path(worker: &Path) -> std::ffi::OsString {
    let private = worker
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("openvino");
    let mut paths = vec![private];
    if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap_or_default()
}

fn remove_socket(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}
