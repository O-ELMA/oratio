use std::{
    fs,
    io::Write,
    path::PathBuf,
};

use futures_util::StreamExt;
use oratio::store;
use tokio::sync::mpsc::UnboundedSender;

pub const MODEL_BYTES: u64 = 190_085_487;
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin";

#[derive(Clone, Debug)]
pub enum SpeechEvent {
    Recording,
    Transcribing,
    Transcript(String),
    Error(String),
    Idle,
}

pub fn supported() -> bool {
    cfg!(target_os = "android")
}

pub fn model_path() -> PathBuf {
    store::data_dir()
        .join("whisper")
        .join("ggml-small-q5_1.bin")
}

pub fn model_ready() -> bool {
    fs::metadata(model_path())
        .map(|meta| meta.len() == MODEL_BYTES)
        .unwrap_or(false)
}

pub async fn download_model(mut progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let path = model_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let part = path.with_extension("part");
    let _ = fs::remove_file(&part);

    let response = reqwest::Client::new()
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let total = response.content_length().unwrap_or(MODEL_BYTES);
    let mut file = fs::File::create(&part).map_err(|error| error.to_string())?;
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        file.write_all(&chunk).map_err(|error| error.to_string())?;
        received += chunk.len() as u64;
        progress(received, total);
    }
    file.flush().map_err(|error| error.to_string())?;
    drop(file);

    if received != MODEL_BYTES {
        let _ = fs::remove_file(&part);
        return Err(format!("expected {MODEL_BYTES} bytes, got {received}"));
    }
    let _ = fs::remove_file(&path);
    fs::rename(&part, &path).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
mod platform {
    use super::*;

    pub fn set_event_sink(_sender: UnboundedSender<SpeechEvent>) {}
    pub fn has_permission() -> bool {
        false
    }
    pub fn request_permission() {}
    pub fn start(_language: &str) -> Result<(), String> {
        Err("voice input is only available on Android".into())
    }
    pub fn stop() {}
    pub fn cancel() {}
}

#[cfg(target_os = "android")]
mod platform {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    };

    use ndk::audio::{
        AudioDirection, AudioError, AudioFormat, AudioPerformanceMode, AudioSharingMode,
        AudioStreamBuilder,
    };
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

    use super::*;

    const RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";
    const PERMISSION_REQUEST: i32 = 7042;
    const MAX_SECONDS: usize = 30;

    static SINK: OnceLock<UnboundedSender<SpeechEvent>> = OnceLock::new();
    static COMMANDS: OnceLock<mpsc::Sender<Command>> = OnceLock::new();
    static CURRENT: Mutex<Option<Job>> = Mutex::new(None);
    static PENDING: AtomicBool = AtomicBool::new(false);

    #[derive(Clone)]
    struct Job {
        stop: Arc<AtomicBool>,
        cancel: Arc<AtomicBool>,
    }

    enum Command {
        Start { language: String, job: Job },
    }

    pub fn set_event_sink(sender: UnboundedSender<SpeechEvent>) {
        let _ = SINK.set(sender);
    }

    pub fn has_permission() -> bool {
        store::check_permission(RECORD_AUDIO)
    }

    pub fn request_permission() {
        store::request_permissions(&[RECORD_AUDIO], PERMISSION_REQUEST);
    }

    pub fn start(language: &str) -> Result<(), String> {
        log::info!("oratio voice: start ({language})");
        if PENDING.swap(true, Ordering::SeqCst) {
            return Err("busy".into());
        }
        let job = Job {
            stop: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        };
        *CURRENT.lock().unwrap() = Some(job.clone());
        ensure_worker();
        let sent = COMMANDS
            .get()
            .ok_or_else(|| "voice worker is not running".to_string())?
            .send(Command::Start {
                language: language.to_string(),
                job,
            })
            .map_err(|error| error.to_string());
        if sent.is_err() {
            PENDING.store(false, Ordering::SeqCst);
        }
        sent
    }

    pub fn stop() {
        if let Some(job) = CURRENT.lock().unwrap().as_ref() {
            job.stop.store(true, Ordering::SeqCst);
        }
    }

    pub fn cancel() {
        if let Some(job) = CURRENT.lock().unwrap().as_ref() {
            job.stop.store(true, Ordering::SeqCst);
            job.cancel.store(true, Ordering::SeqCst);
        }
    }

    fn emit(event: SpeechEvent) {
        if let Some(sink) = SINK.get() {
            let _ = sink.send(event);
        }
    }

    fn finish(event: SpeechEvent) {
        PENDING.store(false, Ordering::SeqCst);
        emit(event);
    }

    fn ensure_worker() {
        if COMMANDS.get().is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let _ = COMMANDS.set(sender);
        let _ = std::thread::Builder::new()
            .name("oratio-voice".into())
            .spawn(move || worker(receiver));
    }

    fn worker(receiver: mpsc::Receiver<Command>) {
        let mut context: Option<WhisperContext> = None;
        while let Ok(Command::Start { language, job }) = receiver.recv() {
            let samples = match record(&job.stop) {
                Ok(samples) => samples,
                Err(error) => {
                    log::warn!("oratio voice: {error}");
                    finish(SpeechEvent::Error(error));
                    continue;
                }
            };
            if job.cancel.load(Ordering::SeqCst) || samples.len() < 8000 {
                finish(SpeechEvent::Idle);
                continue;
            }
            emit(SpeechEvent::Transcribing);
            if context.is_none() {
                match WhisperContext::new_with_params(model_path(), Default::default()) {
                    Ok(loaded) => context = Some(loaded),
                    Err(error) => {
                        log::warn!("oratio voice: could not load model: {error:?}");
                        finish(SpeechEvent::Error(format!("could not load model: {error:?}")));
                        continue;
                    }
                }
            }
            if job.cancel.load(Ordering::SeqCst) {
                finish(SpeechEvent::Idle);
                continue;
            }
            match transcribe(
                context.as_ref().unwrap(),
                &samples,
                &language,
                job.cancel.clone(),
            ) {
                _ if job.cancel.load(Ordering::SeqCst) => finish(SpeechEvent::Idle),
                Ok(text) => finish(SpeechEvent::Transcript(text)),
                Err(error) => {
                    log::warn!("oratio voice: {error}");
                    finish(SpeechEvent::Error(error));
                }
            }
        }
    }

    fn record(stop: &AtomicBool) -> Result<Vec<f32>, String> {
        let stream = AudioStreamBuilder::new()
            .map_err(|error| format!("audio: {error:?}"))?
            .direction(AudioDirection::Input)
            .sample_rate(16000)
            .channel_count(1)
            .format(AudioFormat::PCM_I16)
            .performance_mode(AudioPerformanceMode::LowLatency)
            .sharing_mode(AudioSharingMode::Shared)
            .open_stream()
            .map_err(|error| format!("audio: {error:?}"))?;
        stream
            .request_start()
            .map_err(|error| format!("audio: {error:?}"))?;
        emit(SpeechEvent::Recording);
        let source_rate = stream.sample_rate().max(1) as f32;
        log::info!("oratio voice: capture rate {source_rate}");

        let mut samples: Vec<i16> = Vec::with_capacity(16000 * 10);
        let mut buffer = vec![0i16; 1600];
        while !stop.load(Ordering::SeqCst) && samples.len() < 16000 * MAX_SECONDS {
            let read = unsafe {
                stream.read(
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as i32,
                    100_000_000,
                )
            };
            // ndk 0.9 wraps AAudioStream_read in from_result, so a successful positive
            // frame count comes back as __Unknown(frames).
            let frames = match read {
                Ok(frames) => frames,
                Err(AudioError::__Unknown(frames)) if frames > 0 => frames as u32,
                Err(error) => return Err(format!("audio: {error:?}")),
            };
            if frames > 0 {
                samples.extend_from_slice(&buffer[..frames as usize]);
            }
        }
        let _ = stream.request_stop();
        drop(stream);

        if samples.is_empty() {
            return Ok(Vec::new());
        }
        if (source_rate - 16000.0).abs() < 0.5 {
            return Ok(samples.iter().map(|sample| *sample as f32 / 32768.0).collect());
        }

        let step = source_rate / 16000.0;
        let count = (samples.len() as f32 / step) as usize;
        let mut resampled = Vec::with_capacity(count);
        for index in 0..count {
            let position = index as f32 * step;
            let left = position as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = position - left as f32;
            let value =
                samples[left] as f32 * (1.0 - fraction) + samples[right] as f32 * fraction;
            resampled.push(value / 32768.0);
        }
        Ok(resampled)
    }

    fn transcribe(
        context: &WhisperContext,
        samples: &[f32],
        language: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<String, String> {
        let mut state = context
            .create_state()
            .map_err(|error| format!("whisper: {error:?}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language));
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_n_threads(4);
        params.set_abort_callback_safe(move || cancel.load(Ordering::SeqCst));
        state
            .full(params, samples)
            .map_err(|error| format!("whisper: {error:?}"))?;

        let mut text = String::new();
        for segment in state.as_iter() {
            text.push_str(segment.to_str().unwrap_or(""));
        }
        Ok(text.trim().to_string())
    }
}

pub use platform::*;
