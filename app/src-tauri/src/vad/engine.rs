use parking_lot::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VadConfig {
    pub sensitivity: String,
    pub hangover: Duration,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sensitivity: "medium".into(),
            hangover: Duration::from_millis(400),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VadDecision {
    Active,
    Inactive,
}

pub struct VoiceActivityDetector {
    config: VadConfig,
    threshold: f32,
    #[cfg(feature = "vad-silero")]
    silero: Option<std::sync::Arc<tokio::sync::Mutex<crate::vad::silero::SileroVad>>>,
    last_activation: Mutex<Option<Instant>>,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

impl VoiceActivityDetector {
    pub fn new(config: VadConfig) -> Self {
        let threshold = match config.sensitivity.as_str() {
            "high" => 0.015,
            "low" => 0.035,
            _ => 0.025,
        };
        #[cfg(feature = "vad-silero")]
        let silero = crate::vad::silero::SileroVad::from_env()
            .map(|vad| std::sync::Arc::new(tokio::sync::Mutex::new(vad)))
            .ok();
        Self {
            config,
            threshold,
            #[cfg(feature = "vad-silero")]
            silero,
            last_activation: Mutex::new(None),
        }
    }

    pub fn evaluate(&self, _frame: &[f32]) -> VadDecision {
        #[cfg(feature = "vad-silero")]
        if let Some(vad) = &self.silero {
            let vad = vad.clone();
            let speech = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    vad.lock().await.is_speech(_frame).await.unwrap_or(false)
                })
            });
            return self.apply_hangover(speech);
        }

        // Simple energy-based heuristic
        let energy = if _frame.is_empty() {
            0.0
        } else {
            _frame.iter().map(|sample| sample * sample).sum::<f32>() / _frame.len() as f32
        };
        let speech = energy > self.threshold;
        self.apply_hangover(speech)
    }

    pub fn config(&self) -> &VadConfig {
        &self.config
    }

    pub fn set_hangover(&mut self, duration: Duration) {
        self.config.hangover = duration;
    }

    fn apply_hangover(&self, speech_detected: bool) -> VadDecision {
        if speech_detected {
            let mut guard = self.last_activation.lock();
            *guard = Some(Instant::now());
            return VadDecision::Active;
        }

        let mut guard = self.last_activation.lock();
        if let Some(last) = *guard {
            if last.elapsed() < self.config.hangover {
                return VadDecision::Active;
            }
        }
        *guard = None;
        VadDecision::Inactive
    }
}
