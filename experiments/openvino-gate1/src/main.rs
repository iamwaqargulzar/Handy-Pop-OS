use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use openvino::Core;
use openvino_genai::WhisperPipeline;
use serde::Serialize;

const EXPECTED_SAMPLE_RATE: u32 = 16_000;

#[derive(Parser)]
#[command(about = "Isolated OpenVINO/NPU feasibility probe for Handy")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Load OpenVINO and enumerate the inference devices it can actually see.
    Probe {
        /// Write the machine-readable result to this JSON file.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Load an OpenVINO Whisper model and run repeatable transcription timings.
    Benchmark {
        /// Directory containing the complete OpenVINO Whisper model.
        #[arg(long)]
        model: PathBuf,
        /// Mono, 16 kHz PCM or IEEE-float WAV input.
        #[arg(long)]
        wav: PathBuf,
        /// OpenVINO device. Gate 1 normally uses NPU.
        #[arg(long, default_value = "NPU")]
        device: String,
        /// Number of measured runs after model loading.
        #[arg(long, default_value_t = 3)]
        runs: usize,
        /// Whisper language code. Use `auto` to leave it unset.
        #[arg(long, default_value = "en")]
        language: String,
        /// Include timestamp chunks in the result.
        #[arg(long)]
        timestamps: bool,
        /// Write the machine-readable result to this JSON file.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Serialize)]
struct HostInfo {
    timestamp_unix_seconds: u64,
    kernel: String,
    npu_pci_device: Option<String>,
    accel_device: Option<DeviceNode>,
    render_group_member: bool,
}

#[derive(Serialize)]
struct DeviceNode {
    path: String,
    readable: bool,
    writable: bool,
}

#[derive(Serialize)]
struct ProbeReport {
    schema_version: u32,
    host: HostInfo,
    openvino_loaded: bool,
    available_devices: Vec<String>,
    npu_available: bool,
    error: Option<String>,
}

#[derive(Serialize)]
struct BenchmarkRun {
    run: usize,
    elapsed_ms: u128,
    audio_seconds: f64,
    real_time_factor: f64,
    peak_rss_kib: Option<u64>,
    text: String,
    chunks: Vec<Chunk>,
}

#[derive(Serialize)]
struct Chunk {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
}

#[derive(Serialize)]
struct BenchmarkReport {
    schema_version: u32,
    host: HostInfo,
    model_path: String,
    device_requested: String,
    available_devices: Vec<String>,
    sample_rate_hz: u32,
    sample_count: usize,
    audio_seconds: f64,
    model_load_ms: u128,
    runs: Vec<BenchmarkRun>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Probe { output } => probe(output.as_deref()),
        Command::Benchmark {
            model,
            wav,
            device,
            runs,
            language,
            timestamps,
            output,
        } => benchmark(
            &model,
            &wav,
            &device,
            runs,
            &language,
            timestamps,
            output.as_deref(),
        ),
    }
}

fn probe(output: Option<&Path>) -> Result<()> {
    let host = collect_host_info();
    let mut report = ProbeReport {
        schema_version: 1,
        host,
        openvino_loaded: false,
        available_devices: Vec::new(),
        npu_available: false,
        error: None,
    };

    match Core::new() {
        Ok(core) => {
            report.openvino_loaded = true;
            match core.available_devices() {
                Ok(devices) => {
                    report.available_devices = devices
                        .iter()
                        .map(|device| device.as_ref().to_owned())
                        .collect();
                    report.npu_available = report
                        .available_devices
                        .iter()
                        .any(|device| device == "NPU" || device.starts_with("NPU."));
                }
                Err(error) => report.error = Some(format!("device enumeration failed: {error}")),
            }
        }
        Err(error) => report.error = Some(format!("OpenVINO runtime load failed: {error}")),
    }

    emit_report(&report, output)?;
    if report.npu_available {
        Ok(())
    } else {
        bail!("OpenVINO did not report a usable NPU; see the JSON report")
    }
}

#[allow(clippy::too_many_arguments)]
fn benchmark(
    model: &Path,
    wav: &Path,
    device: &str,
    run_count: usize,
    language: &str,
    timestamps: bool,
    output: Option<&Path>,
) -> Result<()> {
    if run_count == 0 {
        bail!("--runs must be at least 1");
    }
    if !model.is_dir() {
        bail!("model directory does not exist: {}", model.display());
    }

    let audio = read_wav(wav)?;
    let audio_seconds = audio.len() as f64 / EXPECTED_SAMPLE_RATE as f64;
    let core = Core::new().context("loading OpenVINO runtime")?;
    let available_devices: Vec<String> = core
        .available_devices()
        .context("enumerating OpenVINO devices")?
        .iter()
        .map(|candidate| candidate.as_ref().to_owned())
        .collect();
    if !available_devices
        .iter()
        .any(|candidate| candidate == device || candidate.starts_with(&format!("{device}.")))
    {
        bail!(
            "requested device {device:?} is unavailable; OpenVINO reported {available_devices:?}"
        );
    }

    let model_path = model.to_str().context("model path is not valid UTF-8")?;
    let load_started = Instant::now();
    let mut pipeline = WhisperPipeline::new(model_path, device)
        .with_context(|| format!("loading Whisper model on {device}"))?;
    let model_load_ms = load_started.elapsed().as_millis();

    // Start from the model's own configuration. Intel's reference C sample
    // follows this path; constructing and validating an empty standalone
    // configuration can fail before model-specific fields are populated.
    let mut config = pipeline
        .get_generation_config()
        .context("reading the model's Whisper config")?;
    config
        .set_task("transcribe")
        .context("setting Whisper task")?;
    if language != "auto" {
        let language_token = if language.starts_with("<|") && language.ends_with("|>") {
            language.to_owned()
        } else {
            format!("<|{language}|>")
        };
        config
            .set_language(&language_token)
            .context("setting Whisper language")?;
    }
    config
        .set_return_timestamps(timestamps)
        .context("setting timestamp behavior")?;

    let mut measured_runs = Vec::with_capacity(run_count);
    for run in 1..=run_count {
        let started = Instant::now();
        let result = pipeline
            .generate(&audio, Some(&config))
            .with_context(|| format!("transcription run {run}"))?;
        let elapsed = started.elapsed();
        let text = result
            .get_string()
            .with_context(|| format!("reading text for run {run}"))?;
        let mut chunks = Vec::new();
        if result
            .has_chunks()
            .with_context(|| format!("checking chunks for run {run}"))?
        {
            let count = result
                .get_chunks_count()
                .with_context(|| format!("reading chunk count for run {run}"))?;
            for index in 0..count {
                let chunk = result
                    .get_chunk_at(index)
                    .with_context(|| format!("reading chunk {index} for run {run}"))?;
                chunks.push(Chunk {
                    start_seconds: f64::from(
                        chunk
                            .get_start_ts()
                            .with_context(|| format!("reading chunk {index} start"))?,
                    ),
                    end_seconds: f64::from(
                        chunk
                            .get_end_ts()
                            .with_context(|| format!("reading chunk {index} end"))?,
                    ),
                    text: chunk
                        .get_text()
                        .with_context(|| format!("reading chunk {index} text"))?,
                });
            }
        }
        measured_runs.push(BenchmarkRun {
            run,
            elapsed_ms: elapsed.as_millis(),
            audio_seconds,
            real_time_factor: elapsed.as_secs_f64() / audio_seconds,
            peak_rss_kib: peak_rss_kib(),
            text,
            chunks,
        });
    }

    let report = BenchmarkReport {
        schema_version: 1,
        host: collect_host_info(),
        model_path: model.display().to_string(),
        device_requested: device.to_owned(),
        available_devices,
        sample_rate_hz: EXPECTED_SAMPLE_RATE,
        sample_count: audio.len(),
        audio_seconds,
        model_load_ms,
        runs: measured_runs,
    };
    emit_report(&report, output)
}

fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("opening WAV input {}", path.display()))?;
    let spec = reader.spec();
    if spec.channels != 1 {
        bail!("WAV must be mono; found {} channels", spec.channels);
    }
    if spec.sample_rate != EXPECTED_SAMPLE_RATE {
        bail!(
            "WAV must be {EXPECTED_SAMPLE_RATE} Hz; found {} Hz",
            spec.sample_rate
        );
    }

    match spec.sample_format {
        hound::SampleFormat::Float if spec.bits_per_sample == 32 => reader
            .samples::<f32>()
            .map(|sample| sample.context("decoding float WAV sample"))
            .collect(),
        hound::SampleFormat::Int => {
            let scale = 2_f32.powi(i32::from(spec.bits_per_sample) - 1);
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .context("decoding PCM WAV sample")
                })
                .collect()
        }
        _ => bail!(
            "unsupported WAV encoding: {:?}, {} bits",
            spec.sample_format,
            spec.bits_per_sample
        ),
    }
}

fn collect_host_info() -> HostInfo {
    let accel_path = Path::new("/dev/accel/accel0");
    HostInfo {
        timestamp_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs(),
        kernel: command_output("uname", &["-srmo"]).unwrap_or_else(|| "unknown".to_owned()),
        npu_pci_device: command_output("lspci", &["-nn", "-d", "8086:643e"]),
        accel_device: accel_path.exists().then(|| DeviceNode {
            path: accel_path.display().to_string(),
            readable: fs::File::open(accel_path).is_ok(),
            writable: fs::OpenOptions::new().write(true).open(accel_path).is_ok(),
        }),
        render_group_member: command_output("id", &["-nG"])
            .map(|groups| groups.split_whitespace().any(|group| group == "render"))
            .unwrap_or(false),
    }
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

fn emit_report<T: Serialize>(report: &T, output: Option<&Path>) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{json}\n"))
            .with_context(|| format!("writing report {}", path.display()))?;
    }
    Ok(())
}
