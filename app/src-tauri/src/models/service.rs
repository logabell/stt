use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use tauri::{AppHandle, Manager};

use crate::core::{app_state::AppState, events};

use super::{
    build_download_plan, download_and_extract_with_progress, DownloadOutcome, ModelAsset,
    ModelKind, ModelManager, ModelStatus,
};

#[derive(Debug, Clone)]
pub struct ModelDownloadJob {
    pub kind: ModelKind,
}

#[derive(Debug)]
pub struct ModelDownloadService {
    sender: Sender<ModelDownloadJob>,
}

impl Clone for ModelDownloadService {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl ModelDownloadService {
    pub fn new(app: AppHandle, manager: Arc<Mutex<ModelManager>>) -> Result<Self> {
        let (sender, receiver) = unbounded();
        let models_dir = {
            let guard = manager.lock().map_err(|err| anyhow!(err.to_string()))?;
            guard.root().to_path_buf()
        };
        thread::spawn(move || worker_loop(receiver, manager, models_dir, app));
        Ok(Self { sender })
    }

    pub fn queue(&self, job: ModelDownloadJob) -> Result<()> {
        self.sender
            .send(job)
            .context("send model download job to worker")
    }
}

fn worker_loop(
    receiver: Receiver<ModelDownloadJob>,
    manager: Arc<Mutex<ModelManager>>,
    models_dir: PathBuf,
    app: AppHandle,
) {
    for job in receiver.iter() {
        let mut initial_events: Vec<ModelAsset> = Vec::new();
        let selection_plan = {
            let mut guard = match manager.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            let result = guard.assets_mut().into_iter().find_map(|asset| {
                if asset.kind != job.kind
                    || !matches!(
                        asset.status,
                        ModelStatus::NotInstalled | ModelStatus::Error(_)
                    )
                {
                    return None;
                }

                if asset.source.is_none() {
                    asset.status = ModelStatus::Error("missing download source".into());
                    initial_events.push(asset.clone());
                    return Some((asset.name.clone(), None));
                }

                asset.status = ModelStatus::Downloading { progress: 0.0 };
                let name = asset.name.clone();
                let plan = build_download_plan(asset, models_dir.clone());
                initial_events.push(asset.clone());
                Some((name, plan))
            });

            let _ = guard.save();
            drop(guard);

            result
        };
        for snapshot in initial_events {
            emit_status(&app, snapshot);
        }

        let Some((asset_name, plan)) = selection_plan else {
            continue;
        };

        let Some(plan) = plan else {
            continue;
        };

        match download_and_extract_with_progress(&plan, |downloaded| {
            on_progress(
                &manager,
                &app,
                &asset_name,
                downloaded,
                plan.expected_size_bytes,
            );
        }) {
            Ok(outcome) => on_download_success(&manager, &app, job.kind, &asset_name, &outcome),
            Err(error) => on_download_failure(&manager, &app, &asset_name, error),
        }
    }
}

fn on_download_success(
    manager: &Arc<Mutex<ModelManager>>,
    app: &AppHandle,
    kind: ModelKind,
    asset_name: &str,
    outcome: &DownloadOutcome,
) {
    let (snapshot, manager_result) = {
        let mut guard = match manager.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut snapshot = None;

        if let Some(asset) = guard.asset_by_name_mut(asset_name) {
            let extracted_size = total_size(&outcome.final_path);
            match kind {
                ModelKind::StreamingAsr => {
                    if let Some(tokens) = find_tokens_file(&outcome.final_path) {
                        let _ = asset.update_from_file(tokens);
                    }
                }
                ModelKind::Vad => {
                    if let Some(model) = find_first_with_extension(&outcome.final_path, "onnx") {
                        let _ = asset.update_from_file(model);
                    }
                }
                ModelKind::PolishLlm => {
                    if let Some(model) = find_first_with_extension(&outcome.final_path, "gguf") {
                        let _ = asset.update_from_file(model);
                    }
                }
                _ => {}
            }

            let recorded_size = if extracted_size > 0 {
                extracted_size
            } else {
                outcome.archive_size_bytes
            };
            asset.set_size_bytes(recorded_size);
            if asset.checksum.is_none() {
                asset.set_checksum(Some(outcome.checksum.clone()));
            }
            asset.status = ModelStatus::Installed;
            snapshot = Some(asset.clone());
        }

        let save_result = guard.save();
        let sync_result = sync_runtime_environment(&*guard);

        (snapshot, save_result.and(sync_result))
    };

    if let Err(error) = manager_result {
        tracing::warn!("Failed to persist model updates: {error:?}");
    }

    if let Some(snapshot) = snapshot {
        emit_status(app, snapshot);
    }

    if let Some(state) = app.try_state::<AppState>() {
        if let Err(error) = state.reload_pipeline(app) {
            tracing::warn!("Failed to rebuild speech pipeline after model install: {error:?}");
        }
    }
}

fn on_download_failure(
    manager: &Arc<Mutex<ModelManager>>,
    app: &AppHandle,
    asset_name: &str,
    error: anyhow::Error,
) {
    let snapshot = {
        let mut guard = match manager.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut snapshot = None;
        if let Some(asset) = guard.asset_by_name_mut(asset_name) {
            asset.status = ModelStatus::Error(error.to_string());
            snapshot = Some(asset.clone());
        }
        if let Err(save_error) = guard.save() {
            tracing::warn!("Failed to persist model manifest after error: {save_error:?}");
        }
        snapshot
    };

    if let Some(snapshot) = snapshot {
        emit_status(app, snapshot);
    }
}

fn emit_status(app: &AppHandle, asset: ModelAsset) {
    events::emit_model_status(app, asset);
}

fn on_progress(
    manager: &Arc<Mutex<ModelManager>>,
    app: &AppHandle,
    asset_name: &str,
    downloaded: u64,
    expected: Option<u64>,
) {
    let snapshot = if let Ok(mut guard) = manager.lock() {
        if let Some(asset) = guard.asset_by_name_mut(asset_name) {
            let progress = progress_fraction(downloaded, expected);
            asset.status = ModelStatus::Downloading { progress };
            Some(asset.clone())
        } else {
            None
        }
    } else {
        None
    };

    if let Some(asset) = snapshot {
        emit_status(app, asset);
    }
}

fn progress_fraction(downloaded: u64, expected: Option<u64>) -> f32 {
    if let Some(total) = expected {
        if total > 0 {
            return ((downloaded as f64 / total as f64).clamp(0.0, 1.0)) as f32;
        }
    }
    0.0
}

pub fn sync_runtime_environment(manager: &ModelManager) -> Result<()> {
    sync_streaming_env(manager)?;
    sync_vad_env(manager)?;
    sync_polish_env(manager)?;
    Ok(())
}

fn sync_streaming_env(manager: &ModelManager) -> Result<()> {
    if let Some(asset) = manager.primary_asset(&ModelKind::StreamingAsr) {
        if matches!(asset.status, ModelStatus::Installed) {
            let model_dir = asset.path(manager.root());
            if model_dir.exists() {
                std::env::set_var("SHERPA_ONLINE_MODEL", &model_dir);
                if let Some(tokens) = find_tokens_file(&model_dir) {
                    std::env::set_var("SHERPA_ONLINE_TOKENS", tokens);
                } else {
                    std::env::remove_var("SHERPA_ONLINE_TOKENS");
                }
                return Ok(());
            }
        }
    }
    std::env::remove_var("SHERPA_ONLINE_MODEL");
    std::env::remove_var("SHERPA_ONLINE_TOKENS");
    Ok(())
}

fn sync_vad_env(manager: &ModelManager) -> Result<()> {
    if let Some(asset) = manager.primary_asset(&ModelKind::Vad) {
        if matches!(asset.status, ModelStatus::Installed) {
            let vad_dir = asset.path(manager.root());
            if let Some(model) = find_first_with_extension(&vad_dir, "onnx") {
                std::env::set_var("SILERO_VAD_MODEL", model);
                return Ok(());
            }
        }
    }
    std::env::remove_var("SILERO_VAD_MODEL");
    Ok(())
}

fn sync_polish_env(manager: &ModelManager) -> Result<()> {
    if let Some(asset) = manager.primary_asset(&ModelKind::PolishLlm) {
        if matches!(asset.status, ModelStatus::Installed) {
            let llm_dir = asset.path(manager.root());
            if let Some(model) = find_first_with_extension(&llm_dir, "gguf") {
                std::env::set_var("LLAMA_POLISH_MODEL", model);
                return Ok(());
            }
        }
    }
    std::env::remove_var("LLAMA_POLISH_MODEL");
    Ok(())
}

fn find_tokens_file(dir: &Path) -> Option<PathBuf> {
    let default = dir.join("tokens.txt");
    if default.exists() {
        return Some(default);
    }
    let predicate = |entry: &fs::DirEntry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.contains("token"))
            .unwrap_or(false)
    };
    find_first_matching(dir, &predicate)
}

fn find_first_with_extension(dir: &Path, extension: &str) -> Option<PathBuf> {
    let predicate = |entry: &fs::DirEntry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.ends_with(extension))
            .unwrap_or(false)
    };
    find_first_matching(dir, &predicate)
}

fn find_first_matching<F>(dir: &Path, predicate: &F) -> Option<PathBuf>
where
    F: Fn(&fs::DirEntry) -> bool,
{
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_first_matching(&path, predicate) {
                return Some(found);
            }
        } else if predicate(&entry) {
            return Some(path);
        }
    }
    None
}

fn total_size(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            size += total_size(&entry.path());
        }
    }
    size
}
