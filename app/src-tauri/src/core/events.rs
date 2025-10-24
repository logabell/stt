use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioProcessingMode;
use crate::core::pipeline::EngineMetrics;
use crate::llm::AutocleanMode;

pub const EVENT_HUD_STATE: &str = "hud-state";
pub const EVENT_PERFORMANCE_WARNING: &str = "performance-warning";
pub const EVENT_PERFORMANCE_RECOVERED: &str = "performance-recovered";
pub const EVENT_SECURE_BLOCKED: &str = "secure-field-blocked";
pub const EVENT_OPEN_SETTINGS: &str = "open-settings";
pub const EVENT_TRANSCRIPTION_OUTPUT: &str = "transcription-output";
pub const EVENT_PERFORMANCE_METRICS: &str = "performance-metrics";
pub const EVENT_MODEL_STATUS: &str = "model-status";
pub const EVENT_AUDIO_PROCESSING_MODE: &str = "audio-processing-mode";

pub fn emit_hud_state(app: &AppHandle, state: &str) {
    let _ = app.emit(EVENT_HUD_STATE, state.to_string());
}

pub fn emit_performance_warning(app: &AppHandle, metrics: &EngineMetrics) {
    let _ = app.emit(EVENT_PERFORMANCE_WARNING, metrics.clone());
}

pub fn emit_performance_recovered(app: &AppHandle, metrics: &EngineMetrics) {
    let _ = app.emit(EVENT_PERFORMANCE_RECOVERED, metrics.clone());
}

pub fn emit_secure_blocked(app: &AppHandle) {
    let _ = app.emit(EVENT_SECURE_BLOCKED, ());
}

pub fn emit_autoclean_mode(app: &AppHandle, mode: AutocleanMode) {
    let _ = app.emit("autoclean-mode", mode);
}

pub fn emit_transcription_output(app: &AppHandle, text: &str) {
    let _ = app.emit(EVENT_TRANSCRIPTION_OUTPUT, text.to_string());
}

#[derive(Debug, Clone, Serialize)]
struct MetricsPayload {
    last_latency_ms: u64,
    average_cpu_percent: f32,
    consecutive_slow: u32,
    performance_mode: bool,
}

pub fn emit_metrics(app: &AppHandle, metrics: &EngineMetrics) {
    let payload = MetricsPayload {
        last_latency_ms: metrics.last_latency.as_millis() as u64,
        average_cpu_percent: metrics.average_cpu * 100.0,
        consecutive_slow: metrics.consecutive_slow,
        performance_mode: metrics.performance_mode,
    };
    let _ = app.emit(EVENT_PERFORMANCE_METRICS, payload);
}

pub fn emit_model_status<T: Serialize + Clone>(app: &AppHandle, payload: T) {
    let _ = app.emit(EVENT_MODEL_STATUS, payload);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProcessingModePayload {
    pub preferred: AudioProcessingMode,
    pub effective: AudioProcessingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub fn emit_audio_processing_mode(
    app: &AppHandle,
    preferred: AudioProcessingMode,
    effective: AudioProcessingMode,
    reason: Option<&str>,
) {
    let payload = AudioProcessingModePayload {
        preferred,
        effective,
        reason: reason.map(ToOwned::to_owned),
    };
    let _ = app.emit(EVENT_AUDIO_PROCESSING_MODE, payload);
}
