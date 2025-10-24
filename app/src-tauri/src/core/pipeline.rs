use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use parking_lot::Mutex;
use serde::Serialize;
use sysinfo::System;
use tauri::AppHandle;
use tracing::{info, warn};

use crate::asr::{AsrConfig, AsrEngine, RecognitionResult};
use crate::audio::{
    AudioEvent, AudioPipeline, AudioPipelineConfig, AudioPreprocessor, AudioProcessingMode,
};
use crate::core::events;
use crate::llm::{AutocleanMode, AutocleanService};
#[cfg(debug_assertions)]
use crate::output::logs;
use crate::output::{OutputAction, OutputInjector};
use crate::vad::{VadConfig, VadDecision, VoiceActivityDetector};

#[derive(Debug, Clone, Serialize)]
pub struct EngineMetrics {
    pub last_latency: Duration,
    pub consecutive_slow: u32,
    pub performance_mode: bool,
    pub average_cpu: f32,
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self {
            last_latency: Duration::from_millis(0),
            consecutive_slow: 0,
            performance_mode: false,
            average_cpu: 0.0,
        }
    }
}

pub struct SpeechPipeline {
    inner: Arc<SpeechPipelineInner>,
}

struct SpeechPipelineInner {
    audio: AudioPipeline,
    preprocessor: Mutex<AudioPreprocessor>,
    vad: Mutex<VoiceActivityDetector>,
    vad_default_hangover: Mutex<Duration>,
    asr: AsrEngine,
    autoclean: AutocleanService,
    injector: OutputInjector,
    metrics: Arc<Mutex<EngineMetrics>>,
    mode: Arc<Mutex<AutocleanMode>>,
    app: AppHandle,
    audio_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    listening: AtomicBool,
}

impl SpeechPipeline {
    pub fn new(
        app: AppHandle,
        audio_config: AudioPipelineConfig,
        vad_config: VadConfig,
        asr_config: AsrConfig,
    ) -> Self {
        let preprocessor = AudioPreprocessor::new(audio_config.processing_mode);
        let audio = AudioPipeline::spawn(audio_config);
        let vad = VoiceActivityDetector::new(vad_config.clone());
        let inner = Arc::new(SpeechPipelineInner {
            audio,
            preprocessor: Mutex::new(preprocessor),
            vad: Mutex::new(vad),
            vad_default_hangover: Mutex::new(vad_config.hangover),
            asr: AsrEngine::new(asr_config),
            autoclean: AutocleanService::new(),
            injector: OutputInjector::new(),
            metrics: Arc::new(Mutex::new(EngineMetrics::default())),
            mode: Arc::new(Mutex::new(AutocleanMode::Fast)),
            app,
            audio_thread: Mutex::new(None),
            listening: AtomicBool::new(false),
        });

        SpeechPipelineInner::start_audio_loop(&inner);
        SpeechPipelineInner::start_cpu_sampler(&inner);
        inner.emit_processing_mode(None);

        Self { inner }
    }

    pub fn audio_device_id(&self) -> Option<String> {
        self.inner.audio.device_id()
    }

    pub fn process_frame(&self, frame: AudioEvent) -> Result<()> {
        self.inner.process_frame(frame)
    }

    fn update_metrics(&self, latency: Duration) {
        self.inner.update_metrics(latency)
    }

    pub fn record_cpu_load(&self, cpu_fraction: f32) {
        self.inner.record_cpu_load(cpu_fraction)
    }

    pub fn simulate_performance(&self, latency: Duration, cpu_fraction: f32) {
        self.inner.simulate_performance(latency, cpu_fraction)
    }

    pub fn process_transcription(&self, raw_text: &str, latency: Duration, cpu_fraction: f32) {
        self.inner
            .process_transcription(raw_text, latency, cpu_fraction)
    }

    pub fn set_mode(&self, mode: AutocleanMode) {
        self.inner.set_mode(mode)
    }

    pub fn set_vad_config(&self, config: VadConfig) {
        self.inner.set_vad_config(config);
    }

    pub fn autoclean_mode(&self) -> AutocleanMode {
        self.inner.autoclean_mode()
    }

    pub fn set_processing_mode(&self, mode: AudioProcessingMode) {
        self.inner.set_processing_mode(mode)
    }

    pub fn processing_mode(&self) -> AudioProcessingMode {
        self.inner.processing_mode()
    }

    pub fn reset_recognizer(&self) {
        self.inner.reset_recognizer();
    }

    pub fn set_listening(&self, active: bool) {
        self.inner.set_listening(active);
    }

    pub fn is_listening(&self) -> bool {
        self.inner.is_listening()
    }
}

impl SpeechPipelineInner {
    fn start_audio_loop(this: &Arc<Self>) {
        let receiver = this.audio.subscribe();
        let weak = Arc::downgrade(this);
        let handle = std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                if let Some(inner) = weak.upgrade() {
                    if let Err(error) = inner.process_frame(event) {
                        warn!("audio frame processing failed: {error:?}");
                    }
                } else {
                    break;
                }
            }
        });

        let mut guard = this.audio_thread.lock();
        *guard = Some(handle);
    }

    fn start_cpu_sampler(this: &Arc<Self>) {
        let weak = Arc::downgrade(this);
        tokio::spawn(async move {
            let mut system = System::new();
            system.refresh_cpu_usage();
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            // The first measurement after refresh_cpu_usage is usually 0; wait a cycle.
            interval.tick().await;

            loop {
                interval.tick().await;
                if let Some(inner) = weak.upgrade() {
                    system.refresh_cpu_usage();
                    let usage = system.global_cpu_info().cpu_usage() / 100.0;
                    inner.record_cpu_load(usage.clamp(0.0, 1.0));
                } else {
                    break;
                }
            }
        });
    }

    fn process_frame(&self, frame: AudioEvent) -> Result<()> {
        match frame {
            AudioEvent::Frame(mut samples) => {
                if !self.listening.load(Ordering::Relaxed) {
                    return Ok(());
                }

                {
                    let mut preprocessor = self.preprocessor.lock();
                    preprocessor.process(&mut samples);
                }

                let vad_decision = {
                    let detector = self.vad.lock();
                    detector.evaluate(&samples)
                };
                if matches!(vad_decision, VadDecision::Inactive) {
                    return Ok(());
                }

                let recognition = self.asr.recognize(&samples);
                self.update_metrics(recognition.latency);
                Ok(())
            }
            AudioEvent::Stopped => {
                info!("audio stream stopped");
                Ok(())
            }
        }
    }

    fn update_metrics(&self, latency: Duration) {
        let mut metrics = self.metrics.lock();
        metrics.last_latency = latency;

        if latency > Duration::from_secs(2) && metrics.average_cpu > 0.75 {
            metrics.consecutive_slow += 1;
            if metrics.consecutive_slow >= 2 && !metrics.performance_mode {
                metrics.performance_mode = true;
                self.set_performance_override(true);
                warn!("Entering performance warning mode");
                events::emit_performance_warning(&self.app, &*metrics);
                #[cfg(debug_assertions)]
                logs::push_log(format!(
                    "Performance warning: latency={}ms cpu={:.1}%",
                    latency.as_millis(),
                    metrics.average_cpu * 100.0
                ));
            }
        } else {
            metrics.consecutive_slow = 0;
            if metrics.performance_mode {
                info!("recovering from performance warning");
                metrics.performance_mode = false;
                self.set_performance_override(false);
                events::emit_performance_recovered(&self.app, &*metrics);
                #[cfg(debug_assertions)]
                logs::push_log("Performance recovered".to_string());
            }
        }

        events::emit_metrics(&self.app, &*metrics);
    }

    fn record_cpu_load(&self, cpu_fraction: f32) {
        let mut metrics = self.metrics.lock();
        metrics.average_cpu = cpu_fraction;
        if metrics.average_cpu < 0.75 && metrics.performance_mode {
            metrics.performance_mode = false;
            metrics.consecutive_slow = 0;
            info!("Performance warning cleared by CPU recovery");
            self.set_performance_override(false);
            events::emit_performance_recovered(&self.app, &*metrics);
        }

        events::emit_metrics(&self.app, &*metrics);
    }

    fn simulate_performance(&self, latency: Duration, cpu_fraction: f32) {
        {
            let mut metrics = self.metrics.lock();
            metrics.average_cpu = cpu_fraction;
            events::emit_metrics(&self.app, &*metrics);
        }
        self.update_metrics(latency);
    }

    fn process_transcription(&self, raw_text: &str, latency: Duration, cpu_fraction: f32) {
        self.simulate_performance(latency, cpu_fraction);
        let active_mode = *self.mode.lock();
        self.autoclean.set_mode(active_mode);
        let cleaned = self.autoclean.clean(raw_text);
        self.deliver_output(&cleaned);
    }

    fn set_mode(&self, mode: AutocleanMode) {
        let mut guard = self.mode.lock();
        *guard = mode;
        self.autoclean.set_mode(mode);
    }

    fn set_vad_config(&self, config: VadConfig) {
        let mut vad = self.vad.lock();
        *vad = VoiceActivityDetector::new(config.clone());
        let mut default = self.vad_default_hangover.lock();
        *default = config.hangover;
    }

    fn autoclean_mode(&self) -> AutocleanMode {
        *self.mode.lock()
    }

    fn set_processing_mode(&self, mode: AudioProcessingMode) {
        {
            let mut preprocessor = self.preprocessor.lock();
            preprocessor.set_preferred_mode(mode);
        }
        self.emit_processing_mode(Some("user"));
    }

    fn processing_mode(&self) -> AudioProcessingMode {
        self.preprocessor.lock().effective_mode()
    }

    fn set_performance_override(&self, enabled: bool) {
        {
            let mut preprocessor = self.preprocessor.lock();
            preprocessor.set_performance_override(enabled);
        }
        {
            let mut vad = self.vad.lock();
            let default = *self.vad_default_hangover.lock();
            if enabled {
                vad.set_hangover(default.min(Duration::from_millis(200)));
            } else {
                vad.set_hangover(default);
            }
        }
        let reason = if enabled {
            Some("performance-fallback")
        } else {
            Some("performance-recovered")
        };
        self.emit_processing_mode(reason);
    }

    fn emit_processing_mode(&self, reason: Option<&str>) {
        let (preferred, effective) = {
            let pre = self.preprocessor.lock();
            (pre.preferred_mode(), pre.effective_mode())
        };
        events::emit_audio_processing_mode(&self.app, preferred, effective, reason);
    }

    fn reset_recognizer(&self) {
        self.asr.reset();
    }

    fn set_listening(&self, active: bool) {
        if active {
            self.listening.store(true, Ordering::SeqCst);
            self.reset_recognizer();
            return;
        }

        let was_listening = self.listening.swap(false, Ordering::SeqCst);
        if !was_listening {
            self.reset_recognizer();
            return;
        }

        if let Some(result) = self.asr.finalize() {
            self.consume_result(result);
        }
        self.reset_recognizer();
    }

    fn is_listening(&self) -> bool {
        self.listening.load(Ordering::Relaxed)
    }

    fn consume_result(&self, recognition: RecognitionResult) {
        self.update_metrics(recognition.latency);

        let trimmed = recognition.text.trim();
        if trimmed.is_empty() {
            return;
        }

        let active_mode = *self.mode.lock();
        self.autoclean.set_mode(active_mode);
        let cleaned = self.autoclean.clean(trimmed);
        self.deliver_output(&cleaned);
    }

    fn deliver_output(&self, cleaned: &str) {
        if cleaned.trim().is_empty() {
            return;
        }

        events::emit_transcription_output(&self.app, cleaned);
        #[cfg(debug_assertions)]
        logs::push_log(format!("Transcription -> {}", cleaned));
        self.injector.inject(cleaned, OutputAction::Paste);
    }
}

impl Drop for SpeechPipelineInner {
    fn drop(&mut self) {
        let handle = self.audio_thread.lock().take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}
