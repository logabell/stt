use std::time::Duration;

use serde::{Deserialize, Serialize};
#[cfg(feature = "asr-sherpa")]
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AsrMode {
    Streaming,
    Whisper,
}

impl Default for AsrMode {
    fn default() -> Self {
        AsrMode::Streaming
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AsrConfig {
    pub mode: AsrMode,
    pub language: String,
    pub auto_language_detect: bool,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            mode: AsrMode::Streaming,
            language: "auto".into(),
            auto_language_detect: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecognitionResult {
    pub text: String,
    pub latency: Duration,
}

pub struct AsrEngine {
    config: AsrConfig,
    #[cfg(feature = "asr-sherpa")]
    sherpa: Option<std::sync::Arc<crate::asr::sherpa::SherpaAsr>>,
    streaming: std::sync::Mutex<StreamingState>,
}

impl AsrEngine {
    pub fn new(config: AsrConfig) -> Self {
        #[cfg(feature = "asr-sherpa")]
        let sherpa = if config.mode == AsrMode::Streaming {
            crate::asr::sherpa::SherpaAsr::from_env()
                .map(std::sync::Arc::new)
                .map_err(|error| {
                    warn!("Failed to init sherpa recognizer: {error:?}");
                    error
                })
                .ok()
        } else {
            None
        };

        Self {
            config,
            #[cfg(feature = "asr-sherpa")]
            sherpa,
            streaming: std::sync::Mutex::new(StreamingState::default()),
        }
    }

    pub fn recognize(&self, _frames: &[f32]) -> RecognitionResult {
        if matches!(self.config.mode, AsrMode::Streaming) {
            return self.recognize_streaming(_frames);
        }

        let simulated_text = match self.config.mode {
            AsrMode::Whisper => "simulated whisper accuracy output".to_string(),
            AsrMode::Streaming => "simulated streaming dictation output".to_string(),
        };
        let latency = match self.config.mode {
            AsrMode::Whisper => Duration::from_millis(2800),
            AsrMode::Streaming => Duration::from_millis(1200),
        };
        RecognitionResult {
            text: simulated_text,
            latency,
        }
    }

    fn recognize_streaming(&self, frames: &[f32]) -> RecognitionResult {
        let mut state = self.streaming.lock().unwrap_or_else(|err| err.into_inner());
        state.buffer.extend_from_slice(frames);
        state.truncate_if_needed();

        #[cfg(feature = "asr-sherpa")]
        match self.recognize_with_sherpa(&mut state, frames) {
            SherpaDecision::Result(result) => return result,
            SherpaDecision::Pending => {
                return RecognitionResult {
                    text: String::new(),
                    latency: Duration::from_millis(0),
                }
            }
            SherpaDecision::Disabled => {}
        }

        if state.last_text.is_empty() {
            state.last_text = "simulated streaming dictation output".into();
        }
        RecognitionResult {
            text: state.last_text.clone(),
            latency: Duration::from_millis(1200),
        }
    }

    pub fn config(&self) -> &AsrConfig {
        &self.config
    }

    pub fn reset(&self) {
        if let Ok(mut guard) = self.streaming.lock() {
            guard.buffer.clear();
            guard.last_text.clear();
            #[cfg(feature = "asr-sherpa")]
            {
                guard.reset_sherpa();
            }
        }
    }

    pub fn finalize(&self) -> Option<RecognitionResult> {
        match self.config.mode {
            AsrMode::Streaming => self.finalize_streaming(),
            AsrMode::Whisper => self.finalize_whisper(),
        }
    }

    fn finalize_streaming(&self) -> Option<RecognitionResult> {
        let mut state = self.streaming.lock().unwrap_or_else(|err| err.into_inner());

        #[cfg(feature = "asr-sherpa")]
        if let Some(result) = self.finalize_with_sherpa(&mut state) {
            let text = result.text.trim().to_string();
            state.buffer.clear();
            state.last_text.clear();
            state.reset_sherpa();
            return if text.is_empty() {
                None
            } else {
                Some(RecognitionResult {
                    text,
                    latency: result.latency,
                })
            };
        }

        let text = state.last_text.trim().to_string();
        state.buffer.clear();
        state.last_text.clear();
        #[cfg(feature = "asr-sherpa")]
        state.reset_sherpa();

        if text.is_empty() {
            None
        } else {
            Some(RecognitionResult {
                text,
                latency: Duration::from_millis(650),
            })
        }
    }

    fn finalize_whisper(&self) -> Option<RecognitionResult> {
        let mut state = self.streaming.lock().unwrap_or_else(|err| err.into_inner());
        let text = state.last_text.trim().to_string();
        state.buffer.clear();
        state.last_text.clear();
        #[cfg(feature = "asr-sherpa")]
        state.reset_sherpa();
        if text.is_empty() {
            None
        } else {
            Some(RecognitionResult {
                text,
                latency: Duration::from_millis(2800),
            })
        }
    }
}

struct StreamingState {
    buffer: Vec<f32>,
    last_text: String,
    #[cfg(feature = "asr-sherpa")]
    sherpa_stream: Option<crate::asr::sherpa::SherpaStream>,
    #[cfg(feature = "asr-sherpa")]
    sherpa_failed: bool,
}

impl StreamingState {
    #[cfg(feature = "asr-sherpa")]
    fn reset_sherpa(&mut self) {
        self.sherpa_stream = None;
        self.sherpa_failed = false;
    }

    fn truncate_if_needed(&mut self) {
        const MAX_SAMPLES: usize = 16_000 * 120; // roughly 2 minutes of audio
        if self.buffer.len() > MAX_SAMPLES {
            let overflow = self.buffer.len() - MAX_SAMPLES;
            self.buffer.drain(..overflow);
        }
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            last_text: String::new(),
            #[cfg(feature = "asr-sherpa")]
            sherpa_stream: None,
            #[cfg(feature = "asr-sherpa")]
            sherpa_failed: false,
        }
    }
}

#[cfg(feature = "asr-sherpa")]
enum SherpaDecision {
    Disabled,
    Pending,
    Result(RecognitionResult),
}

#[cfg(feature = "asr-sherpa")]
impl AsrEngine {
    fn recognize_with_sherpa(&self, state: &mut StreamingState, frames: &[f32]) -> SherpaDecision {
        let Some(sherpa) = self.sherpa.as_ref() else {
            return SherpaDecision::Disabled;
        };
        if state.sherpa_failed {
            return SherpaDecision::Disabled;
        }

        if state.sherpa_stream.is_none() {
            match sherpa.create_stream() {
                Ok(stream) => state.sherpa_stream = Some(stream),
                Err(error) => {
                    warn!("Failed to create sherpa stream: {error:?}");
                    state.sherpa_failed = true;
                    return SherpaDecision::Disabled;
                }
            }
        }

        let Some(stream) = state.sherpa_stream.as_ref() else {
            return SherpaDecision::Disabled;
        };

        if let Err(error) = stream.accept_waveform(frames) {
            warn!("Sherpa accept waveform failed: {error:?}");
            state.sherpa_failed = true;
            state.sherpa_stream = None;
            return SherpaDecision::Disabled;
        }

        let decoded_text = match stream.decode() {
            Ok(value) => value,
            Err(error) => {
                warn!("Sherpa decode failed: {error:?}");
                state.sherpa_failed = true;
                state.sherpa_stream = None;
                return SherpaDecision::Disabled;
            }
        };

        let endpoint = stream.is_endpoint().unwrap_or(false);

        let trimmed = decoded_text.trim();
        if trimmed.is_empty() {
            if endpoint && !state.last_text.is_empty() {
                let final_text = match stream.finish() {
                    Ok(text) => text,
                    Err(error) => {
                        warn!("Sherpa finalize failed: {error:?}");
                        String::new()
                    }
                };
                if !final_text.trim().is_empty() {
                    state.last_text = final_text.trim().to_string();
                }
                state.sherpa_stream = None;
                return SherpaDecision::Result(RecognitionResult {
                    text: state.last_text.clone(),
                    latency: Duration::from_millis(650),
                });
            }
            return SherpaDecision::Pending;
        }

        if trimmed != state.last_text {
            state.last_text = trimmed.to_string();
        }

        if endpoint {
            let final_text = match stream.finish() {
                Ok(text) => text,
                Err(error) => {
                    warn!("Sherpa finalize failed: {error:?}");
                    state.last_text.clone()
                }
            };
            let final_trimmed = final_text.trim();
            if !final_trimmed.is_empty() {
                state.last_text = final_trimmed.to_string();
            }
            state.sherpa_stream = None;
        }

        let latency = if endpoint {
            Duration::from_millis(620)
        } else {
            Duration::from_millis(450)
        };

        if endpoint {
            SherpaDecision::Result(RecognitionResult {
                text: state.last_text.clone(),
                latency,
            })
        } else {
            SherpaDecision::Pending
        }
    }

    fn finalize_with_sherpa(&self, state: &mut StreamingState) -> Option<RecognitionResult> {
        if state.sherpa_failed {
            return None;
        }

        if let Some(stream) = state.sherpa_stream.take() {
            match stream.finish() {
                Ok(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() && state.last_text.trim().is_empty() {
                        None
                    } else {
                        if !trimmed.is_empty() {
                            state.last_text = trimmed.to_string();
                        }
                        Some(RecognitionResult {
                            text: state.last_text.clone(),
                            latency: Duration::from_millis(620),
                        })
                    }
                }
                Err(error) => {
                    warn!("Sherpa finalize failed: {error:?}");
                    state.sherpa_failed = true;
                    None
                }
            }
        } else if state.last_text.trim().is_empty() {
            None
        } else {
            Some(RecognitionResult {
                text: state.last_text.clone(),
                latency: Duration::from_millis(620),
            })
        }
    }
}
