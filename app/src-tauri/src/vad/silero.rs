#[cfg(feature = "vad-silero")]
mod silero {
    use anyhow::{anyhow, Context, Result};
    use ort::{
        session::{builder::GraphOptimizationLevel, Session},
        value::Tensor,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    const SAMPLE_RATE: usize = 16_000;
    const FRAME_SIZE: usize = 512;

    pub struct SileroVad {
        session: Arc<Mutex<Session>>,
        hidden_state: Arc<Mutex<Option<(Vec<usize>, Vec<f32>)>>>,
    }

    impl SileroVad {
        pub fn new(model_bytes: &[u8]) -> Result<Self> {
            let session = Session::builder()
                .map_err(|err| anyhow!(err))?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(|err| anyhow!(err))?
                .commit_from_memory(model_bytes)
                .map_err(|err| anyhow!(err))?;

            Ok(Self {
                session: Arc::new(Mutex::new(session)),
                hidden_state: Arc::new(Mutex::new(None)),
            })
        }

        pub fn from_env() -> Result<Self> {
            let path = std::env::var("SILERO_VAD_MODEL").context("SILERO_VAD_MODEL not set")?;
            let bytes = std::fs::read(path).context("read silero model")?;
            Self::new(&bytes)
        }

        pub async fn is_speech(&self, audio: &[f32]) -> Result<bool> {
            let mut session = self.session.lock().await;
            let mut hidden = self.hidden_state.lock().await;

            let chunks: Vec<f32> = audio.iter().copied().collect();
            let frame_count = chunks.len() / FRAME_SIZE;
            let mut speech_detected = false;

            for frame_idx in 0..frame_count {
                let start = frame_idx * FRAME_SIZE;
                let end = start + FRAME_SIZE;
                let frame = &chunks[start..end];

                let audio_tensor =
                    Tensor::from_array(([1usize, FRAME_SIZE], frame.to_vec().into_boxed_slice()))
                        .map_err(|err| anyhow!(err))?;
                let sr_tensor =
                    Tensor::from_array(([1usize], vec![SAMPLE_RATE as f32].into_boxed_slice()))
                        .map_err(|err| anyhow!(err))?;

                let outputs = if let Some((state_shape, state_data)) = hidden.as_ref() {
                    let hidden_tensor = Tensor::from_array((
                        state_shape.clone(),
                        state_data.clone().into_boxed_slice(),
                    ))
                    .map_err(|err| anyhow!(err))?;
                    session
                        .run(ort::inputs![audio_tensor, sr_tensor, hidden_tensor])
                        .map_err(|err| anyhow!(err))?
                } else {
                    session
                        .run(ort::inputs![audio_tensor, sr_tensor])
                        .map_err(|err| anyhow!(err))?
                };

                let (_, speech_tensor) = outputs[0]
                    .try_extract_tensor::<f32>()
                    .map_err(|err| anyhow!(err))?;
                let speech_prob = speech_tensor.first().copied().unwrap_or(0.0);
                if speech_prob > 0.6 {
                    speech_detected = true;
                }

                let (state_shape, state_tensor) = outputs[1]
                    .try_extract_tensor::<f32>()
                    .map_err(|err| anyhow!(err))?;
                *hidden = Some((
                    state_shape.iter().map(|dim| *dim as usize).collect(),
                    state_tensor.to_vec(),
                ));
            }

            Ok(speech_detected)
        }
    }
}

#[cfg(feature = "vad-silero")]
pub use silero::SileroVad;
