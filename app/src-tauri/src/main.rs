#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod asr;
mod audio;
mod core;
mod llm;
mod models;
mod output;
mod vad;

use anyhow::anyhow;
use audio::{list_input_devices, AudioDeviceInfo};
use core::{app_state::AppState, settings::FrontendSettings};
use models::{ModelAsset, ModelKind};
use tauri::{AppHandle, Manager};
use tracing::metadata::LevelFilter;

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> tauri::Result<FrontendSettings> {
    state.settings_manager().read_frontend().map_err(Into::into)
}

#[tauri::command]
async fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: FrontendSettings,
) -> tauri::Result<()> {
    state
        .settings_manager()
        .write_frontend(settings)
        .map_err(tauri::Error::from)?;

    let fresh = state
        .settings_manager()
        .read_frontend()
        .map_err(tauri::Error::from)?;

    state
        .configure_pipeline(Some(&app), &fresh)
        .map_err(tauri::Error::from)?;

    Ok(())
}

#[tauri::command]
async fn register_hotkeys(app: AppHandle) -> tauri::Result<()> {
    core::hotkeys::register(&app)?;
    Ok(())
}

#[tauri::command]
async fn unregister_hotkeys(app: AppHandle) -> tauri::Result<()> {
    core::hotkeys::unregister(&app)?;
    Ok(())
}

#[tauri::command]
async fn begin_dictation(app: AppHandle, state: tauri::State<'_, AppState>) -> tauri::Result<()> {
    state.start_session(&app);
    Ok(())
}

#[tauri::command]
async fn mark_dictation_processing(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state.mark_processing(&app);
    Ok(())
}

#[tauri::command]
async fn complete_dictation(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state.complete_session(&app);
    Ok(())
}

#[tauri::command]
async fn list_models(state: tauri::State<'_, AppState>) -> tauri::Result<Vec<ModelAsset>> {
    let manager_arc = state.model_manager();
    let manager = manager_arc
        .lock()
        .map_err(|err| tauri::Error::from(anyhow!(err.to_string())))?;
    Ok(manager.assets().into_iter().cloned().collect())
}

#[tauri::command]
async fn install_streaming_asr(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state
        .queue_model_download(&app, ModelKind::StreamingAsr)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn install_vad_model(app: AppHandle, state: tauri::State<'_, AppState>) -> tauri::Result<()> {
    state
        .queue_model_download(&app, ModelKind::Vad)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn install_polish_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state
        .queue_model_download(&app, ModelKind::PolishLlm)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn uninstall_streaming_asr(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state
        .uninstall_model(&app, ModelKind::StreamingAsr)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn uninstall_vad_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state
        .uninstall_model(&app, ModelKind::Vad)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn uninstall_polish_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state
        .uninstall_model(&app, ModelKind::PolishLlm)
        .map_err(tauri::Error::from)
}

#[tauri::command]
async fn list_audio_devices() -> tauri::Result<Vec<AudioDeviceInfo>> {
    Ok(list_input_devices())
}

#[tauri::command]
async fn secure_field_blocked(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> tauri::Result<()> {
    state.secure_blocked(&app);
    Ok(())
}

#[tauri::command]
async fn simulate_performance(
    state: tauri::State<'_, AppState>,
    latency_ms: u64,
    cpu_percent: f32,
) -> tauri::Result<()> {
    state
        .simulate_performance(latency_ms, cpu_percent)
        .map_err(tauri::Error::from)?;
    Ok(())
}

#[tauri::command]
async fn simulate_transcription(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    raw_text: String,
    latency_ms: Option<u64>,
    cpu_percent: Option<f32>,
) -> tauri::Result<()> {
    let latency = latency_ms.unwrap_or(1800);
    let cpu = cpu_percent.unwrap_or(65.0);

    state
        .simulate_transcription(&app, &raw_text, latency, cpu)
        .map_err(tauri::Error::from)?;
    Ok(())
}

#[cfg(debug_assertions)]
#[tauri::command]
async fn get_logs() -> Vec<String> {
    crate::output::logs::snapshot()
}

fn setup_logging() {
    let filter = std::env::var("STT_LOG")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(LevelFilter::INFO);

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(filter)
        .with_target(false)
        .compact()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn main() {
    setup_logging();

    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            register_hotkeys,
            unregister_hotkeys,
            begin_dictation,
            mark_dictation_processing,
            complete_dictation,
            secure_field_blocked,
            simulate_performance,
            simulate_transcription,
            list_models,
            install_streaming_asr,
            install_vad_model,
            install_polish_model,
            uninstall_streaming_asr,
            uninstall_vad_model,
            uninstall_polish_model,
            list_audio_devices,
            #[cfg(debug_assertions)]
            get_logs
        ])
        .setup(|app| {
            output::tray::initialize(app)?;
            if let Some(state) = app.try_state::<AppState>() {
                let handle = app.handle();
                state.initialize_models(&handle)?;
                if let Err(error) = state.initialize_pipeline(&handle) {
                    tracing::warn!("Failed to initialize pipeline: {error:?}");
                }
                #[cfg(debug_assertions)]
                {
                    crate::core::dev_simulator::start(&handle);
                    crate::output::logs::initialize(&handle);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
