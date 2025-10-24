use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    StreamingAsr,
    Whisper,
    PolishLlm,
    Vad,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ModelStatus {
    NotInstalled,
    Downloading { progress: f32 },
    Installed,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAsset {
    pub kind: ModelKind,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default)]
    pub size_bytes: u64,
    pub status: ModelStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ModelSource>,
}

impl ModelAsset {
    #[must_use]
    pub fn path(&self, base_dir: &Path) -> PathBuf {
        base_dir
            .join(&self.kind_path())
            .join(format!("{}-{}", self.name, self.version))
    }

    #[must_use]
    fn kind_path(&self) -> String {
        match self.kind {
            ModelKind::StreamingAsr => "streaming".into(),
            ModelKind::Whisper => "whisper".into(),
            ModelKind::PolishLlm => "polish".into(),
            ModelKind::Vad => "vad".into(),
        }
    }

    pub fn set_checksum(&mut self, checksum: Option<String>) {
        self.checksum = checksum;
    }

    pub fn set_size_bytes(&mut self, size_bytes: u64) {
        self.size_bytes = size_bytes;
    }

    pub fn update_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            let metadata = std::fs::metadata(path).context("stat asset file")?;
            self.size_bytes = metadata.len();
            let checksum = crate::models::compute_sha256(path)?;
            self.checksum = Some(checksum);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSource {
    pub uri: String,
    pub archive_format: ArchiveFormat,
    #[serde(default)]
    pub strip_prefix_components: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveFormat {
    Zip,
    TarGz,
    TarBz2,
    File,
}

pub struct ModelManager {
    root: PathBuf,
    manifest: PathBuf,
    assets: Vec<ModelAsset>,
}

impl ModelManager {
    pub fn new() -> Result<Self> {
        let root = resolve_model_dir()?;
        let manifest = root.join("manifest.json");
        let mut manager = Self {
            root,
            manifest,
            assets: vec![],
        };
        manager.load_manifest()?;
        manager.register_defaults();
        manager.save()?;
        Ok(manager)
    }

    pub fn ensure_directory(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root).context("create model directory")?;
        Ok(())
    }

    pub fn register_asset(&mut self, asset: ModelAsset) {
        if let Some(existing) = self
            .assets
            .iter_mut()
            .find(|current| current.kind == asset.kind && current.name == asset.name)
        {
            *existing = asset;
        } else {
            self.assets.push(asset);
        }
    }

    pub fn asset(&self, kind: &ModelKind) -> Option<&ModelAsset> {
        self.primary_asset(kind)
    }

    pub fn assets(&self) -> Vec<&ModelAsset> {
        self.assets.iter().collect()
    }

    pub fn assets_mut(&mut self) -> Vec<&mut ModelAsset> {
        self.assets.iter_mut().collect()
    }

    pub fn assets_by_kind(&self, kind: &ModelKind) -> Vec<&ModelAsset> {
        self.assets
            .iter()
            .filter(|asset| &asset.kind == kind)
            .collect()
    }

    pub fn asset_by_name(&self, name: &str) -> Option<&ModelAsset> {
        self.assets.iter().find(|asset| asset.name == name)
    }

    pub fn primary_asset(&self, kind: &ModelKind) -> Option<&ModelAsset> {
        self.assets_by_kind(kind).into_iter().max_by_key(|asset| {
            (
                matches!(asset.status, ModelStatus::Installed),
                asset.size_bytes,
            )
        })
    }

    pub fn asset_by_name_mut(&mut self, name: &str) -> Option<&mut ModelAsset> {
        self.assets.iter_mut().find(|asset| asset.name == name)
    }

    pub fn save(&self) -> Result<()> {
        let manifest = File::create(&self.manifest).context("create model manifest")?;
        serde_json::to_writer_pretty(manifest, &self.assets).context("write model manifest")?;
        Ok(())
    }

    pub fn uninstall(&mut self, kind: &ModelKind) -> Result<Option<ModelAsset>> {
        if let Some(asset) = self.assets.iter_mut().find(|asset| &asset.kind == kind) {
            let path = asset.path(&self.root);
            if path.exists() {
                fs::remove_dir_all(&path)
                    .with_context(|| format!("remove model directory {}", path.display()))?;
            }
            asset.checksum = None;
            asset.size_bytes = 0;
            asset.status = ModelStatus::NotInstalled;
            let snapshot = asset.clone();
            self.save()?;
            return Ok(Some(snapshot));
        }
        Ok(None)
    }

    fn load_manifest(&mut self) -> Result<()> {
        if self.manifest.exists() {
            let manifest = File::open(&self.manifest).context("open model manifest")?;
            let assets: Vec<ModelAsset> =
                serde_json::from_reader(manifest).context("parse model manifest")?;
            self.assets = assets;
        }
        Ok(())
    }

    pub fn assets_by_kind_mut(&mut self, kind: &ModelKind) -> Vec<&mut ModelAsset> {
        self.assets
            .iter_mut()
            .filter(|asset| &asset.kind == kind)
            .collect()
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    fn register_defaults(&mut self) {
        for asset in default_assets() {
            if let Some(existing) = self
                .assets
                .iter_mut()
                .find(|current| current.name == asset.name)
            {
                if existing.status == ModelStatus::NotInstalled {
                    *existing = asset;
                }
            } else {
                self.assets.push(asset);
            }
        }
    }
}

fn resolve_model_dir() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "PushToTalk", "PushToTalk")
        .context("missing project directories")?;
    let dir = project_dirs.data_dir().join("models");
    std::fs::create_dir_all(&dir).context("create models dir")?;
    Ok(dir)
}

fn default_assets() -> Vec<ModelAsset> {
    vec![
        ModelAsset {
            kind: ModelKind::StreamingAsr,
            name: "sherpa-onnx-streaming-zipformer-en-20M".into(),
            version: "2023-02-17".into(),
            checksum: None,
            size_bytes: 127_887_156,
            status: ModelStatus::NotInstalled,
            source: Some(ModelSource {
                uri: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2"
                    .into(),
                archive_format: ArchiveFormat::TarBz2,
                strip_prefix_components: 0,
            }),
        },
        ModelAsset {
            kind: ModelKind::StreamingAsr,
            name: "sherpa-onnx-streaming-zipformer-en".into(),
            version: "2023-06-26".into(),
            checksum: None,
            size_bytes: 310_414_022,
            status: ModelStatus::NotInstalled,
            source: Some(ModelSource {
                uri: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-2023-06-26.tar.bz2"
                    .into(),
                archive_format: ArchiveFormat::TarBz2,
                strip_prefix_components: 0,
            }),
        },
        ModelAsset {
            kind: ModelKind::Vad,
            name: "silero-vad-onnx".into(),
            version: "v4.0".into(),
            checksum: None,
            size_bytes: 0,
            status: ModelStatus::NotInstalled,
            source: Some(ModelSource {
                uri: "https://github.com/snakers4/silero-vad/releases/download/v4.0/silero_vad.onnx".into(),
                archive_format: ArchiveFormat::File,
                strip_prefix_components: 0,
            }),
        },
        ModelAsset {
            kind: ModelKind::PolishLlm,
            name: "tiny-llama-1.1b-chat-q4_k_m".into(),
            version: "2024-01-01".into(),
            checksum: None,
            size_bytes: 0,
            status: ModelStatus::NotInstalled,
            source: Some(ModelSource {
                uri: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/TinyLlama-1.1B-Chat-v1.0-Q4_K_M.gguf?download=1".into(),
                archive_format: ArchiveFormat::File,
                strip_prefix_components: 0,
            }),
        },
    ]
}
