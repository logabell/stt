import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type ModelKind =
  | "streaming-asr"
  | "whisper"
  | "polish-llm"
  | "vad";

type RawModelStatus =
  | "notInstalled"
  | "installed"
  | { downloading: { progress: number } }
  | { error: string };

export interface RawModelAsset {
  name: string;
  kind: ModelKind;
  version: string;
  sizeBytes: number;
  checksum?: string | null;
  status: RawModelStatus;
}

export interface ModelSnapshotPayload {
  name: string;
  kind: ModelKind;
  version: string;
  sizeBytes: number;
  checksum?: string | null;
  status: RawModelStatus;
}

export type ModelStateKind =
  | { state: "notInstalled" }
  | { state: "installed" }
  | { state: "downloading"; progress: number }
  | { state: "error"; message: string };

export interface ModelRecord {
  name: string;
  kind: ModelKind;
  version: string;
  sizeBytes: number;
  checksum: string | null;
  status: ModelStateKind;
}

export type HudState =
  | "idle"
  | "listening"
  | "processing"
  | "performance-warning"
  | "secure-blocked";

export interface AppSettings {
  hotkeyMode: "hold" | "toggle";
  hudTheme: "system" | "light" | "dark" | "high-contrast";
  language: string;
  autoDetectLanguage: boolean;
  autocleanMode: "off" | "fast" | "polish" | "cloud";
  polishModelReady: boolean;
  debugTranscripts: boolean;
  audioDeviceId: string | null;
  processingMode: "standard" | "enhanced";
  vadSensitivity: "low" | "medium" | "high";
}

export interface AudioProcessingModeState {
  preferred: "standard" | "enhanced";
  effective: "standard" | "enhanced";
}

export type AudioProcessingReason =
  | "user"
  | "performance-fallback"
  | "performance-recovered"
  | undefined;

export interface AudioProcessingPayload extends AudioProcessingModeState {
  reason?: AudioProcessingReason;
}

export interface PerformanceMetrics {
  lastLatencyMs: number;
  averageCpuPercent: number;
  consecutiveSlow: number;
  performanceMode: boolean;
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
  hotkeyMode: "hold",
  hudTheme: "system",
  language: "auto",
  autoDetectLanguage: true,
  autocleanMode: "fast",
  polishModelReady: false,
  debugTranscripts: false,
  audioDeviceId: null,
  processingMode: "standard",
  vadSensitivity: "medium",
};

interface AppState {
  hudState: HudState;
  settingsVisible: boolean;
  logViewerVisible: boolean;
  settings: AppSettings | null;
  initialize: () => Promise<void>;
  setHudState: (state: HudState) => void;
  toggleSettings: (value?: boolean) => void;
  toggleLogViewer: (value?: boolean) => void;
  updateSettings: (settings: AppSettings) => Promise<void>;
  refreshSettings: () => Promise<void>;
  setSettingsState: (settings: AppSettings) => void;
  lastTranscript: string;
  metrics: PerformanceMetrics | null;
  logs: string[];
  setTranscript: (text: string) => void;
  setMetrics: (metrics: PerformanceMetrics) => void;
  setLogs: (logs: string[]) => void;
  startDictation: () => Promise<void>;
  markDictationProcessing: () => Promise<void>;
  completeDictation: () => Promise<void>;
  secureFieldBlocked: () => Promise<void>;
  simulatePerformance: (latencyMs: number, cpuPercent: number) => Promise<void>;
  simulateTranscription: (
    text: string,
    latencyMs?: number,
    cpuPercent?: number,
  ) => Promise<void>;
  models: ModelRecord[];
  refreshModels: () => Promise<void>;
  setModelSnapshot: (snapshot: ModelSnapshotPayload) => void;
  installStreamingModel: () => Promise<void>;
  installVadModel: () => Promise<void>;
  installPolishModel: () => Promise<void>;
  uninstallStreamingModel: () => Promise<void>;
  uninstallVadModel: () => Promise<void>;
  uninstallPolishModel: () => Promise<void>;
  toasts: Toast[];
  notify: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: number) => void;
  audioDevices: AudioDevice[];
  refreshAudioDevices: () => Promise<void>;
  processingMode: AudioProcessingModeState;
  applyProcessingModeUpdate: (payload: AudioProcessingPayload) => void;
}

export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export const useAppStore = create<AppState>((set, get) => ({
  hudState: "idle",
  settingsVisible: false,
  logViewerVisible: false,
  settings: null,
  lastTranscript: "",
  metrics: null,
  logs: [],
  models: [],
  toasts: [],
  audioDevices: [],
  processingMode: {
    preferred: DEFAULT_APP_SETTINGS.processingMode,
    effective: DEFAULT_APP_SETTINGS.processingMode,
  },
  initialize: async () => {
    await get().refreshSettings();
    await get().refreshModels();
    await get().refreshAudioDevices();
  },
  setHudState: (state) => set({ hudState: state }),
  toggleSettings: (value) =>
    set((prev) => ({
      settingsVisible:
        value !== undefined ? value : !prev.settingsVisible,
    })),
  toggleLogViewer: (value) =>
    set((prev) => ({
      logViewerVisible:
        value !== undefined ? value : !prev.logViewerVisible,
    })),
  updateSettings: async (settings) => {
    await invoke("update_settings", { settings });
    await get().refreshSettings();
    await get().refreshAudioDevices();
  },
  refreshSettings: async () => {
    const settings = await invoke<AppSettings>("get_settings");
    set((state) => ({
      settings,
      processingMode: {
        preferred: settings.processingMode,
        effective: state.processingMode.effective,
      },
    }));
  },
  setSettingsState: (settings) =>
    set((state) => ({
      settings,
      processingMode: {
        preferred: settings.processingMode,
        effective: state.processingMode.effective,
      },
    })),
  setTranscript: (text) => set({ lastTranscript: text }),
  setMetrics: (metrics) => set({ metrics }),
  setLogs: (logs) => set({ logs }),
  startDictation: async () => {
    await invoke("begin_dictation");
  },
  markDictationProcessing: async () => {
    await invoke("mark_dictation_processing");
  },
  completeDictation: async () => {
    await invoke("complete_dictation");
  },
  secureFieldBlocked: async () => {
    await invoke("secure_field_blocked");
  },
  simulatePerformance: async (latencyMs, cpuPercent) => {
    await invoke("simulate_performance", {
      latencyMs,
      cpuPercent,
    });
  },
  simulateTranscription: async (text, latencyMs, cpuPercent) => {
    await invoke("simulate_transcription", {
      rawText: text,
      latencyMs,
      cpuPercent,
    });
  },
  refreshModels: async () => {
    const raw = await invoke<RawModelAsset[]>("list_models");
    const normalized = raw.map(normalizeModelRecord);
    set({ models: normalized });
  },
  setModelSnapshot: (snapshot) => {
    set((state) => {
      const next = normalizeModelRecord(snapshot);
      const existingIndex = state.models.findIndex((model) => model.name === next.name);
      const models = (() => {
        if (existingIndex === -1) {
          return [...state.models, next];
        }
        const updated = state.models.slice();
        updated[existingIndex] = next;
        return updated;
      })();

      let toasts = state.toasts;
      if (next.status.state === "installed") {
        toasts = [
          ...toasts,
          {
            id: Date.now(),
            title: `${formatModelName(next.name)} installed`,
            variant: "success",
          },
        ];
      } else if (next.status.state === "error") {
        toasts = [
          ...toasts,
          {
            id: Date.now(),
            title: `${formatModelName(next.name)} failed`,
            description: next.status.message,
            variant: "error",
          },
        ];
      }

      return { models, toasts };
    });
  },
  installStreamingModel: async () => {
    try {
      await invoke("install_streaming_asr");
      get().notify({
        title: "Streaming model download started",
        variant: "info",
      });
    } catch (error) {
      console.error("Failed to start streaming model install", error);
      get().notify({
        title: "Streaming model install failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  installVadModel: async () => {
    try {
      await invoke("install_vad_model");
      get().notify({
        title: "VAD model download started",
        variant: "info",
      });
    } catch (error) {
      console.error("Failed to start VAD model install", error);
      get().notify({
        title: "VAD install failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  installPolishModel: async () => {
    try {
      await invoke("install_polish_model");
      get().notify({
        title: "Polish model download started",
        variant: "info",
      });
      await get().refreshSettings();
    } catch (error) {
      console.error("Failed to start polish model install", error);
      get().notify({
        title: "Polish install failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  uninstallStreamingModel: async () => {
    try {
      await invoke("uninstall_streaming_asr");
      get().notify({
        title: "Streaming model removed",
        variant: "info",
      });
    } catch (error) {
      console.error("Failed to uninstall streaming model", error);
      get().notify({
        title: "Streaming uninstall failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  uninstallVadModel: async () => {
    try {
      await invoke("uninstall_vad_model");
      get().notify({
        title: "VAD model removed",
        variant: "info",
      });
    } catch (error) {
      console.error("Failed to uninstall VAD model", error);
      get().notify({
        title: "VAD uninstall failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  uninstallPolishModel: async () => {
    try {
      await invoke("uninstall_polish_model");
      get().notify({
        title: "Polish model removed",
        variant: "info",
      });
      await get().refreshSettings();
    } catch (error) {
      console.error("Failed to uninstall polish model", error);
      get().notify({
        title: "Polish uninstall failed",
        description: String(error),
        variant: "error",
      });
    }
  },
  notify: (toast) =>
    set((state) => ({
      toasts: [...state.toasts, { id: Date.now(), ...toast }],
    })),
  dismissToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    })),
  refreshAudioDevices: async () => {
    const devices = await invoke<AudioDevice[]>("list_audio_devices");
    set({ audioDevices: devices });
  },
  applyProcessingModeUpdate: (payload) => {
    const previous = get().processingMode;
    set({
      processingMode: {
        preferred: payload.preferred,
        effective: payload.effective,
      },
    });

    const notify = get().notify;
    if (
      payload.reason === "performance-fallback" &&
      previous.effective === "enhanced" &&
      payload.effective === "standard"
    ) {
      notify({
        title: "Enhanced audio paused",
        description:
          "System load is high, so Enhanced audio is temporarily disabled.",
        variant: "warning",
      });
    } else if (
      payload.reason === "performance-recovered" &&
      previous.effective === "standard" &&
      payload.effective === "enhanced"
    ) {
      notify({
        title: "Enhanced audio restored",
        description: "Load has recovered; Enhanced audio is active again.",
        variant: "info",
      });
    }
  },
}));

export interface Toast {
  id: number;
  title: string;
  description?: string;
  variant?: "info" | "success" | "warning" | "error";
}

function formatModelName(name: string): string {
  if (name.includes("zipformer")) {
    return "Streaming ASR";
  }
  if (name.includes("silero")) {
    return "Silero VAD";
  }
  return name;
}

function normalizeModelRecord(raw: RawModelAsset): ModelRecord {
  return {
    name: raw.name,
    kind: raw.kind,
    version: raw.version,
    sizeBytes: raw.sizeBytes ?? 0,
    checksum: raw.checksum ?? null,
    status: normalizeStatus(raw.status),
  };
}

function normalizeStatus(status: RawModelStatus): ModelStateKind {
  if (typeof status === "string") {
    if (status === "installed") {
      return { state: "installed" };
    }
    if (status === "notInstalled") {
      return { state: "notInstalled" };
    }
  } else if ("downloading" in status) {
    return {
      state: "downloading",
      progress: status.downloading.progress ?? 0,
    };
  } else if ("error" in status) {
    return { state: "error", message: status.error }; 
  }
  return { state: "notInstalled" };
}
