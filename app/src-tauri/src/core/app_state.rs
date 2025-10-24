use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use parking_lot::Mutex;

use crate::asr::{AsrConfig, AsrMode};
use crate::audio::{AudioPipelineConfig, AudioProcessingMode};
use crate::core::events;
use crate::llm::AutocleanMode;
use crate::models::{
    sync_runtime_environment, ModelDownloadJob, ModelDownloadService, ModelKind, ModelManager,
    ModelStatus,
};
use crate::vad::VadConfig;
use tauri::AppHandle;

use super::pipeline::SpeechPipeline;
use super::settings::SettingsManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Listening,
    Processing,
}

pub struct AppState {
    settings: Arc<SettingsManager>,
    pipeline: Arc<Mutex<Option<SpeechPipeline>>>,
    session: Arc<Mutex<SessionState>>,
    models: Arc<StdMutex<ModelManager>>,
    downloads: Arc<Mutex<Option<ModelDownloadService>>>,
}

impl AppState {
    pub fn new() -> Self {
        let models = ModelManager::new().expect("failed to initialize model manager");
        Self {
            settings: Arc::new(SettingsManager::new()),
            pipeline: Arc::new(Mutex::new(None)),
            session: Arc::new(Mutex::new(SessionState::Idle)),
            models: Arc::new(StdMutex::new(models)),
            downloads: Arc::new(Mutex::new(None)),
        }
    }

    pub fn settings_manager(&self) -> Arc<SettingsManager> {
        self.settings.clone()
    }

    pub fn pipeline(&self) -> Arc<Mutex<Option<SpeechPipeline>>> {
        self.pipeline.clone()
    }

    pub fn model_manager(&self) -> Arc<StdMutex<ModelManager>> {
        self.models.clone()
    }

    pub fn start_session(&self, app: &AppHandle) {
        let should_start = {
            let mut guard = self.session.lock();
            if *guard == SessionState::Listening {
                false
            } else {
                *guard = SessionState::Listening;
                true
            }
        };
        if !should_start {
            return;
        }

        if let Some(pipeline) = self.pipeline.lock().as_ref() {
            pipeline.set_listening(true);
        }

        events::emit_hud_state(app, "listening");
    }

    pub fn mark_processing(&self, app: &AppHandle) {
        let mut guard = self.session.lock();
        *guard = SessionState::Processing;
        events::emit_hud_state(app, "processing");
    }

    pub fn complete_session(&self, app: &AppHandle) {
        let previous = {
            let mut guard = self.session.lock();
            let prev = *guard;
            *guard = SessionState::Idle;
            prev
        };

        if let Some(pipeline) = self.pipeline.lock().as_ref() {
            pipeline.set_listening(false);
        }

        if previous != SessionState::Idle {
            events::emit_hud_state(app, "idle");
        } else {
            events::emit_hud_state(app, "idle");
        }
    }

    pub fn secure_blocked(&self, app: &AppHandle) {
        events::emit_secure_blocked(app);
        self.complete_session(app);
    }

    pub fn simulate_performance(&self, latency_ms: u64, cpu_percent: f32) -> Result<()> {
        let latency = Duration::from_millis(latency_ms);
        let cpu_fraction = (cpu_percent / 100.0).clamp(0.0, 1.0);

        let guard = self.pipeline.lock();
        let pipeline = guard
            .as_ref()
            .ok_or_else(|| anyhow!("pipeline not initialized"))?;
        pipeline.simulate_performance(latency, cpu_fraction);
        Ok(())
    }

    pub fn simulate_transcription(
        &self,
        app: &AppHandle,
        raw_text: &str,
        latency_ms: u64,
        cpu_percent: f32,
    ) -> Result<()> {
        let latency = Duration::from_millis(latency_ms);
        let cpu_fraction = (cpu_percent / 100.0).clamp(0.0, 1.0);

        let guard = self.pipeline.lock();
        let pipeline = guard
            .as_ref()
            .ok_or_else(|| anyhow!("pipeline not initialized"))?;

        self.start_session(app);
        self.mark_processing(app);
        pipeline.process_transcription(raw_text, latency, cpu_fraction);
        self.complete_session(app);

        Ok(())
    }

    pub fn is_listening(&self) -> bool {
        matches!(*self.session.lock(), SessionState::Listening)
    }

    pub fn hotkey_mode(&self) -> String {
        self.settings
            .read_frontend()
            .map(|settings| settings.hotkey_mode)
            .unwrap_or_else(|_| "hold".into())
    }

    pub fn initialize_pipeline(&self, app: &AppHandle) -> Result<()> {
        self.sync_model_environment();
        let settings = self.settings.read_frontend()?;
        self.configure_pipeline(Some(app), &settings)
    }

    pub fn configure_pipeline(
        &self,
        app: Option<&AppHandle>,
        settings: &crate::core::settings::FrontendSettings,
    ) -> Result<()> {
        let mut guard = self.pipeline.lock();
        if let Some(existing) = guard.as_ref() {
            let desired_device = settings.audio_device_id.clone();
            if existing.audio_device_id() != desired_device {
                *guard = None;
            }
        }

        let processing_mode = parse_processing_mode(&settings.processing_mode);
        let vad_config = VadConfig {
            sensitivity: settings.vad_sensitivity.clone(),
            ..VadConfig::default()
        };

        if let Some(pipeline) = guard.as_mut() {
            pipeline.set_mode(parse_autoclean_mode(&settings.autoclean_mode));
            pipeline.set_processing_mode(processing_mode);
            pipeline.set_vad_config(vad_config.clone());
            if let Some(app) = app {
                events::emit_autoclean_mode(app, parse_autoclean_mode(&settings.autoclean_mode));
            }
            return Ok(());
        }

        let app = app.ok_or_else(|| anyhow!("app handle required to construct pipeline"))?;
        self.sync_model_environment();
        let audio_config = AudioPipelineConfig {
            device_id: settings.audio_device_id.clone(),
            processing_mode,
        };
        let mut asr_config = AsrConfig::default();
        asr_config.language = settings.language.clone();
        asr_config.auto_language_detect = settings.auto_detect_language;
        if self.streaming_model_installed() {
            asr_config.mode = AsrMode::Streaming;
        } else {
            asr_config.mode = AsrMode::Whisper;
        }

        let pipeline =
            SpeechPipeline::new(app.clone(), audio_config, vad_config.clone(), asr_config);
        pipeline.set_mode(parse_autoclean_mode(&settings.autoclean_mode));
        pipeline.set_processing_mode(processing_mode);
        pipeline.set_vad_config(vad_config);
        *guard = Some(pipeline);
        events::emit_autoclean_mode(app, parse_autoclean_mode(&settings.autoclean_mode));
        Ok(())
    }

    pub fn initialize_models(&self, app: &AppHandle) -> Result<()> {
        self.ensure_download_service(app)?;
        self.sync_model_environment();
        Ok(())
    }

    pub fn queue_model_download(&self, app: &AppHandle, kind: ModelKind) -> Result<()> {
        self.ensure_download_service(app)?;
        let service = self
            .downloads
            .lock()
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("download service unavailable"))?;
        service.queue(ModelDownloadJob { kind })
    }

    pub fn reload_pipeline(&self, app: &AppHandle) -> Result<()> {
        let settings = self.settings.read_frontend()?;
        {
            let mut guard = self.pipeline.lock();
            *guard = None;
        }
        self.configure_pipeline(Some(app), &settings)
    }

    fn ensure_download_service(&self, app: &AppHandle) -> Result<()> {
        let mut guard = self.downloads.lock();
        if guard.is_none() {
            let manager = self.models.clone();
            let service = ModelDownloadService::new(app.clone(), manager)?;
            *guard = Some(service);
        }
        Ok(())
    }

    fn sync_model_environment(&self) {
        if let Ok(manager) = self.models.lock() {
            let polish_ready = manager
                .primary_asset(&ModelKind::PolishLlm)
                .map(|asset| matches!(asset.status, ModelStatus::Installed))
                .unwrap_or(false);

            if let Err(error) = sync_runtime_environment(&*manager) {
                tracing::warn!("Failed to sync model runtime environment: {error:?}");
            }

            drop(manager);

            if let Err(error) = self.settings.set_polish_ready(polish_ready) {
                tracing::warn!("Failed to update polish readiness: {error:?}");
            }
        }
    }

    pub fn uninstall_model(&self, app: &AppHandle, kind: ModelKind) -> Result<()> {
        let snapshot = {
            let mut guard = self.models.lock().map_err(|err| anyhow!(err.to_string()))?;
            let result = guard.uninstall(&kind)?;
            result
        };
        self.sync_model_environment();
        if let Some(asset) = snapshot {
            events::emit_model_status(app, asset);
        }
        self.reload_pipeline(app)?;
        Ok(())
    }

    fn streaming_model_installed(&self) -> bool {
        self.models
            .lock()
            .map(|guard| {
                guard
                    .primary_asset(&ModelKind::StreamingAsr)
                    .map(|asset| matches!(asset.status, ModelStatus::Installed))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
}

fn parse_autoclean_mode(value: &str) -> AutocleanMode {
    match value {
        "off" => AutocleanMode::Off,
        "polish" => AutocleanMode::Polish,
        "cloud" => AutocleanMode::Cloud,
        _ => AutocleanMode::Fast,
    }
}

fn parse_processing_mode(value: &str) -> AudioProcessingMode {
    match value {
        "enhanced" => AudioProcessingMode::Enhanced,
        _ => AudioProcessingMode::Standard,
    }
}
