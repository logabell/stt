use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
#[cfg(feature = "real-audio")]
use tracing::warn;
use tracing::{debug, info};

use super::AudioProcessingMode;

const DEFAULT_FRAME_LEN: usize = 320;
const DEFAULT_FRAME_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioPipelineConfig {
    pub device_id: Option<String>,
    pub processing_mode: AudioProcessingMode,
}

impl Default for AudioPipelineConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            processing_mode: AudioProcessingMode::Standard,
        }
    }
}

#[derive(Debug)]
pub enum AudioEvent {
    Frame(Vec<f32>),
    Stopped,
}

pub struct AudioPipeline {
    #[cfg(feature = "real-audio")]
    real_audio: Option<RealAudioHandle>,
    _worker: JoinHandle<()>,
    sender: Sender<AudioEvent>,
    receiver: Receiver<AudioEvent>,
    config: Arc<AudioPipelineConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

impl AudioPipeline {
    pub fn spawn(config: AudioPipelineConfig) -> Self {
        let (tx, rx) = bounded(16);
        let (out_tx, out_rx) = bounded(64);
        let config = Arc::new(config);
        #[cfg(feature = "real-audio")]
        let real_audio = match RealAudioHandle::spawn(Arc::clone(&config), tx.clone()) {
            Ok(handle) => {
                info!("real audio capture started");
                Some(handle)
            }
            Err(error) => {
                warn!("real audio capture failed, falling back to synthetic: {error:?}");
                None
            }
        };

        #[cfg(not(feature = "real-audio"))]
        let real_audio: Option<RealAudioHandle> = None;

        let use_synthetic = real_audio.is_none();
        let worker = tokio::spawn(async move {
            info!("audio pipeline worker started (synthetic={use_synthetic})");
            let mut phase = 0.0f32;
            let mut frame = Vec::with_capacity(DEFAULT_FRAME_LEN);
            let mut tick = tokio::time::interval(DEFAULT_FRAME_INTERVAL);

            loop {
                if let Ok(event) = rx.try_recv() {
                    let _ = out_tx.send(event);
                }

                if use_synthetic {
                    tick.tick().await;
                    frame.clear();
                    for _ in 0..DEFAULT_FRAME_LEN {
                        let sample = (phase * 2.0 * std::f32::consts::PI).sin() * 0.03;
                        frame.push(sample);
                        phase = (phase + 0.01) % 1.0;
                    }
                    if out_tx.try_send(AudioEvent::Frame(frame.clone())).is_err() {
                        debug!("audio frame dropped (backpressure)");
                    }
                } else {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        });

        Self {
            #[cfg(feature = "real-audio")]
            real_audio,
            _worker: worker,
            sender: tx,
            receiver: out_rx,
            config,
        }
    }

    pub fn push(&self, event: AudioEvent) {
        let _ = self.sender.send(event);
    }

    pub fn receiver(&self) -> &Receiver<AudioEvent> {
        &self.receiver
    }

    pub fn config(&self) -> Arc<AudioPipelineConfig> {
        Arc::clone(&self.config)
    }

    pub fn subscribe(&self) -> Receiver<AudioEvent> {
        self.receiver.clone()
    }

    pub fn device_id(&self) -> Option<String> {
        self.config.device_id.clone()
    }
}

pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    #[cfg(feature = "real-audio")]
    {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());

        host.input_devices()
            .map(|devices| {
                devices
                    .filter_map(|device| {
                        let name = device.name().ok()?;
                        let is_default = default_name
                            .as_ref()
                            .map(|default| default == &name)
                            .unwrap_or(false);
                        Some(AudioDeviceInfo {
                            id: name.clone(),
                            name,
                            is_default,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    #[cfg(not(feature = "real-audio"))]
    {
        Vec::new()
    }
}

#[cfg(feature = "real-audio")]
struct RealAudioHandle {
    stop: Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "real-audio")]
impl RealAudioHandle {
    fn spawn(config: Arc<AudioPipelineConfig>, sender: Sender<AudioEvent>) -> anyhow::Result<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let (stop_tx, stop_rx) = bounded::<()>(1);
        let (ready_tx, ready_rx) = bounded::<Result<(), anyhow::Error>>(1);

        let thread = std::thread::spawn(move || {
            let startup = || -> anyhow::Result<()> {
                let host = cpal::default_host();
                let device = if let Some(device_id) = &config.device_id {
                    host.input_devices()
                        .ok()
                        .and_then(|devices| {
                            devices
                                .into_iter()
                                .find(|d| d.name().ok().as_ref() == Some(device_id))
                        })
                        .or_else(|| host.default_input_device())
                } else {
                    host.default_input_device()
                }
                .ok_or_else(|| anyhow::anyhow!("no input device available"))?;

                let desired_sample_rate = 16_000u32;
                let stream_config = device
                    .supported_input_configs()
                    .ok()
                    .and_then(|mut configs| {
                        configs.find(|cfg| {
                            cfg.sample_format() == cpal::SampleFormat::F32
                                && cfg.min_sample_rate().0 <= desired_sample_rate
                                && cfg.max_sample_rate().0 >= desired_sample_rate
                        })
                    })
                    .map(|cfg| {
                        cfg.with_sample_rate(cpal::SampleRate(desired_sample_rate))
                            .config()
                    })
                    .or_else(|| device.default_input_config().ok().map(|cfg| cfg.config()))
                    .unwrap_or(cpal::StreamConfig {
                        channels: 1,
                        sample_rate: cpal::SampleRate(desired_sample_rate),
                        buffer_size: cpal::BufferSize::Default,
                    });

                let channels = stream_config.channels as usize;
                let frame_samples = ((stream_config.sample_rate.0 as usize) * 20) / 1000;
                let mut buffer = Vec::with_capacity(frame_samples);
                let sender_clone = sender.clone();

                let stream = device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        for frame in data.chunks(channels) {
                            let sample = frame.get(0).copied().unwrap_or(0.0);
                            buffer.push(sample);
                            if buffer.len() >= frame_samples {
                                let mut out = Vec::with_capacity(frame_samples);
                                out.extend_from_slice(&buffer[..frame_samples]);
                                buffer.drain(..frame_samples);
                                if sender_clone.try_send(AudioEvent::Frame(out)).is_err() {
                                    buffer.clear();
                                }
                            }
                        }
                    },
                    |err| warn!("audio input error: {err}"),
                    None,
                )?;

                stream.play()?;
                let _ = ready_tx.send(Ok(()));

                while stop_rx.recv_timeout(Duration::from_millis(200)).is_err() {}

                let _ = sender.try_send(AudioEvent::Stopped);
                drop(stream);
                Ok(())
            };

            if let Err(error) = startup() {
                let _ = ready_tx.send(Err(error));
            }
        });

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop: stop_tx,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = stop_tx.send(());
                let _ = thread.join();
                Err(error)
            }
            Err(err) => {
                let _ = stop_tx.send(());
                let _ = thread.join();
                Err(anyhow::anyhow!("audio thread initialization failed: {err}"))
            }
        }
    }
}

#[cfg(feature = "real-audio")]
impl Drop for RealAudioHandle {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                warn!("audio capture thread exited with panic");
            }
        }
    }
}
