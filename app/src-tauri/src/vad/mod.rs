mod engine;
#[cfg(feature = "vad-silero")]
pub mod silero;

pub use engine::{VadConfig, VadDecision, VoiceActivityDetector};
