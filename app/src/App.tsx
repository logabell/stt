import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  useAppStore,
  type HudState,
  type AppSettings,
  type PerformanceMetrics,
  DEFAULT_APP_SETTINGS,
  type ModelSnapshotPayload,
  type AudioProcessingPayload,
} from "./state/appStore";
import HUD from "./components/HUD";
import SettingsPanel from "./components/SettingsPanel";
import LogViewer from "./components/LogViewer";
import ToastStack from "./components/ToastStack";

const App = () => {
  const {
    initialize,
    settingsVisible,
    setHudState,
    toggleSettings,
    setSettingsState,
    setTranscript,
    setMetrics,
    logViewerVisible,
    toggleLogViewer,
    setLogs,
    setModelSnapshot,
    applyProcessingModeUpdate,
  } = useAppStore();

  useEffect(() => {
    initialize();
    const unlisteners: Array<() => void> = [];

    const registerListener = async () => {
      const hudDispose = await listen<HudState>("hud-state", (event) => {
        if (event.payload) {
          setHudState(event.payload);
        }
      });
      unlisteners.push(() => hudDispose());

      const performanceDispose = await listen("performance-warning", () => {
        setHudState("performance-warning");
      });
      unlisteners.push(() => performanceDispose());

      const performanceRecoveredDispose = await listen(
        "performance-recovered",
        () => {
          setHudState("idle");
        },
      );
      unlisteners.push(() => performanceRecoveredDispose());

      const processingModeDispose = await listen<AudioProcessingPayload>(
        "audio-processing-mode",
        (event) => {
          if (event.payload) {
            applyProcessingModeUpdate(event.payload);
          }
        },
      );
      unlisteners.push(() => processingModeDispose());

      const secureDispose = await listen("secure-field-blocked", () => {
        setHudState("secure-blocked");
      });
      unlisteners.push(() => secureDispose());

      const autocleanDispose = await listen<AppSettings["autocleanMode"]>(
        "autoclean-mode",
        (event) => {
          if (event.payload) {
            const current =
              useAppStore.getState().settings ?? DEFAULT_APP_SETTINGS;
            setSettingsState({
              ...current,
              autocleanMode: event.payload,
            });
          }
        },
      );
      unlisteners.push(() => autocleanDispose());

      const settingsDispose = await listen("open-settings", () => {
        toggleSettings(true);
      });
      unlisteners.push(() => settingsDispose());

      const transcriptDispose = await listen<string>(
        "transcription-output",
        (event) => {
          if (event.payload) {
            setTranscript(event.payload);
          }
        },
      );
      unlisteners.push(() => transcriptDispose());

      const metricsDispose = await listen<PerformanceMetrics>(
        "performance-metrics",
        (event) => {
          if (event.payload) {
            setMetrics(event.payload);
          }
        },
      );
      unlisteners.push(() => metricsDispose());

      const modelStatusDispose = await listen<ModelSnapshotPayload>(
        "model-status",
        (event) => {
          if (event.payload) {
            setModelSnapshot(event.payload);
          }
        },
      );
      unlisteners.push(() => modelStatusDispose());

      if (import.meta.env.DEV) {
        const logsOpenDispose = await listen("open-logs", () => {
          void (async () => {
            try {
              const snapshot = await invoke<string[]>("get_logs");
              setLogs(snapshot ?? []);
            } catch (error) {
              console.error("Failed to fetch logs", error);
            }
            toggleLogViewer(true);
          })();
        });
        unlisteners.push(() => logsOpenDispose());

        const logsUpdateDispose = await listen<string[]>(
          "logs-updated",
          (event) => {
            if (event.payload) {
              setLogs(event.payload);
            }
          },
        );
        unlisteners.push(() => logsUpdateDispose());
      }
    };

    registerListener().catch((error) =>
      console.error("Failed to attach listeners", error),
    );
    invoke("register_hotkeys").catch((error) =>
      console.error("Failed to register hotkeys", error),
    );

    return () => {
      unlisteners.forEach((dispose) => dispose());
      invoke("unregister_hotkeys").catch((error) =>
        console.error("Failed to unregister hotkeys", error),
      );
    };
  }, [
    initialize,
    setHudState,
    toggleSettings,
    setTranscript,
    setMetrics,
    toggleLogViewer,
    setLogs,
    setModelSnapshot,
    applyProcessingModeUpdate,
  ]);

  return (
    <>
      <HUD />
      {settingsVisible && <SettingsPanel />}
      {import.meta.env.DEV && logViewerVisible && <LogViewer />}
      <ToastStack />
    </>
  );
};

export default App;
