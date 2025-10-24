#[cfg(feature = "asr-sherpa")]
mod binding {
    use anyhow::{anyhow, Context, Result};
    use sherpa_rs::sherpa_rs_sys as sys;
    use std::{
        ffi::{CStr, CString, OsStr},
        path::{Path, PathBuf},
        sync::Arc,
    };

    const DEFAULT_SAMPLE_RATE: i32 = 16_000;
    const DEFAULT_FEATURE_DIM: i32 = 80;

    pub struct SherpaAsr {
        inner: Arc<SherpaInner>,
    }

    struct SherpaInner {
        recognizer: *const sys::SherpaOnnxOnlineRecognizer,
        sample_rate: i32,
        #[allow(dead_code)]
        cstrings: Vec<CString>,
    }

    unsafe impl Send for SherpaInner {}
    unsafe impl Sync for SherpaInner {}

    impl Drop for SherpaInner {
        fn drop(&mut self) {
            unsafe { sys::SherpaOnnxDestroyOnlineRecognizer(self.recognizer) };
        }
    }

    impl SherpaAsr {
        pub fn from_env() -> Result<Self> {
            let model_dir = PathBuf::from(
                std::env::var("SHERPA_ONLINE_MODEL").context("SHERPA_ONLINE_MODEL not set")?,
            );
            let tokens_path = std::env::var("SHERPA_ONLINE_TOKENS")
                .map(PathBuf::from)
                .or_else(|_| find_tokens(&model_dir))
                .context(
                    "SHERPA_ONLINE_TOKENS not set and tokens could not be discovered in model dir",
                )?;

            let encoder_path = find_component(&model_dir, "encoder")?;
            let decoder_path = find_component(&model_dir, "decoder")?;
            let joiner_path = find_component(&model_dir, "joiner")?;

            let provider = std::env::var("SHERPA_ONLINE_PROVIDER").unwrap_or_else(|_| "cpu".into());
            let threads = std::env::var("SHERPA_ONLINE_THREADS")
                .ok()
                .and_then(|value| value.parse::<i32>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(2);

            let feature_dim = std::env::var("SHERPA_ONLINE_FEATURE_DIM")
                .ok()
                .and_then(|value| value.parse::<i32>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_FEATURE_DIM);

            let decoding_method = CString::new("greedy_search").unwrap();
            let provider_c = CString::new(provider).unwrap();
            let tokens_c = path_to_cstring(&tokens_path)?;
            let encoder_c = path_to_cstring(&encoder_path)?;
            let decoder_c = path_to_cstring(&decoder_path)?;
            let joiner_c = path_to_cstring(&joiner_path)?;

            // Keep C strings alive for the lifetime of the recognizer.
            let mut cstrings = vec![
                decoding_method.clone(),
                provider_c.clone(),
                tokens_c.clone(),
                encoder_c.clone(),
                decoder_c.clone(),
                joiner_c.clone(),
            ];

            let mut model_config: sys::SherpaOnnxOnlineModelConfig = unsafe { std::mem::zeroed() };
            model_config.transducer = sys::SherpaOnnxOnlineTransducerModelConfig {
                encoder: encoder_c.as_ptr(),
                decoder: decoder_c.as_ptr(),
                joiner: joiner_c.as_ptr(),
            };
            model_config.tokens = tokens_c.as_ptr();
            model_config.num_threads = threads;
            model_config.provider = provider_c.as_ptr();
            model_config.debug = 0;

            let mut recognizer_config: sys::SherpaOnnxOnlineRecognizerConfig =
                unsafe { std::mem::zeroed() };
            recognizer_config.feat_config = sys::SherpaOnnxFeatureConfig {
                sample_rate: DEFAULT_SAMPLE_RATE,
                feature_dim,
            };
            recognizer_config.model_config = model_config;
            recognizer_config.decoding_method = decoding_method.as_ptr();
            recognizer_config.max_active_paths = 4;
            recognizer_config.enable_endpoint = 1;
            recognizer_config.rule1_min_trailing_silence = 2.4;
            recognizer_config.rule2_min_trailing_silence = 1.2;
            recognizer_config.rule3_min_utterance_length = 30.0;

            let recognizer = unsafe { sys::SherpaOnnxCreateOnlineRecognizer(&recognizer_config) };
            if recognizer.is_null() {
                return Err(anyhow!("SherpaOnnxCreateOnlineRecognizer returned null"));
            }

            // Push a sentinel CString so the vector is never empty
            cstrings.push(CString::new(String::new()).unwrap());

            Ok(Self {
                inner: Arc::new(SherpaInner {
                    recognizer,
                    sample_rate: DEFAULT_SAMPLE_RATE,
                    cstrings,
                }),
            })
        }

        pub fn sample_rate(&self) -> i32 {
            self.inner.sample_rate
        }

        pub fn create_stream(&self) -> Result<SherpaStream> {
            let stream = unsafe { sys::SherpaOnnxCreateOnlineStream(self.inner.recognizer) };
            if stream.is_null() {
                return Err(anyhow!("SherpaOnnxCreateOnlineStream returned null"));
            }
            Ok(SherpaStream {
                inner: Arc::clone(&self.inner),
                stream,
            })
        }
    }

    pub struct SherpaStream {
        inner: Arc<SherpaInner>,
        stream: *const sys::SherpaOnnxOnlineStream,
    }

    unsafe impl Send for SherpaStream {}
    unsafe impl Sync for SherpaStream {}

    impl SherpaStream {
        pub fn accept_waveform(&self, samples: &[f32]) -> Result<()> {
            unsafe {
                sys::SherpaOnnxOnlineStreamAcceptWaveform(
                    self.stream,
                    self.inner.sample_rate,
                    samples.as_ptr(),
                    samples.len().try_into().unwrap_or(i32::MAX),
                );
            }
            Ok(())
        }

        pub fn decode(&self) -> Result<String> {
            unsafe {
                while sys::SherpaOnnxIsOnlineStreamReady(self.inner.recognizer, self.stream) != 0 {
                    sys::SherpaOnnxDecodeOnlineStream(self.inner.recognizer, self.stream);
                }

                let result =
                    sys::SherpaOnnxGetOnlineStreamResult(self.inner.recognizer, self.stream);
                if result.is_null() {
                    return Ok(String::new());
                }

                let text_ptr = (*result).text;
                let text = if text_ptr.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(text_ptr).to_string_lossy().into_owned()
                };
                sys::SherpaOnnxDestroyOnlineRecognizerResult(result);
                Ok(text)
            }
        }

        pub fn finish(&self) -> Result<String> {
            unsafe { sys::SherpaOnnxOnlineStreamInputFinished(self.stream) };
            self.decode()
        }

        pub fn is_endpoint(&self) -> Result<bool> {
            let detected = unsafe {
                sys::SherpaOnnxOnlineStreamIsEndpoint(self.inner.recognizer, self.stream)
            };
            Ok(detected != 0)
        }
    }

    impl Drop for SherpaStream {
        fn drop(&mut self) {
            unsafe { sys::SherpaOnnxDestroyOnlineStream(self.stream) };
        }
    }

    pub(super) fn find_component(model_dir: &Path, component: &str) -> Result<PathBuf> {
        let direct = model_dir.join(format!("{component}.onnx"));
        if direct.exists() {
            return Ok(direct);
        }

        std::fs::read_dir(model_dir)
            .context("read model directory")?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.extension() == Some(OsStr::new("onnx"))
                    && path
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .map(|stem| stem.contains(component))
                        .unwrap_or(false)
            })
            .with_context(|| format!("Could not locate {component} ONNX file in {:?}", model_dir))
    }

    pub(super) fn find_tokens(model_dir: &Path) -> Result<PathBuf> {
        let default = model_dir.join("tokens.txt");
        if default.exists() {
            return Ok(default);
        }

        std::fs::read_dir(model_dir)
            .context("read model directory")?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.extension() == Some(OsStr::new("txt"))
                    && path
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .map(|stem| stem.contains("token"))
                        .unwrap_or(false)
            })
            .with_context(|| {
                format!(
                    "Could not locate tokens file in {:?} (expected tokens.txt or *token*.txt)",
                    model_dir
                )
            })
    }

    fn path_to_cstring(path: &Path) -> Result<CString> {
        CString::new(path.to_string_lossy().into_owned()).map_err(|_| {
            anyhow!(
                "Path {:?} contains interior NUL bytes which are not supported by sherpa-onnx",
                path
            )
        })
    }
}

#[cfg(feature = "asr-sherpa")]
pub use binding::{SherpaAsr, SherpaStream};
