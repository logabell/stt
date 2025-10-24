import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../state/appStore";
import type {
  AppSettings,
  AudioDevice,
  AudioProcessingModeState,
  ModelRecord,
  ModelStateKind,
  PerformanceMetrics,
} from "../state/appStore";

const SettingsPanel = () => {
  const {
    settings,
    updateSettings,
    toggleSettings,
    startDictation,
    markDictationProcessing,
    completeDictation,
    simulatePerformance,
    simulateTranscription,
    lastTranscript,
    metrics,
    hudState,
    toggleLogViewer,
    setLogs,
    models,
    installStreamingModel,
    installVadModel,
    installPolishModel,
    uninstallStreamingModel,
    uninstallVadModel,
    uninstallPolishModel,
    audioDevices,
    refreshAudioDevices,
    processingMode,
  } = useAppStore();

  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [performanceTest, setPerformanceTest] = useState({
    latencyMs: 2500,
    cpuPercent: 85,
  });
  const [transcriptionTest, setTranscriptionTest] = useState({
    text: "Testing push to talk",
    latencyMs: 1800,
    cpuPercent: 60,
  });

  useEffect(() => {
    void refreshAudioDevices();
  }, [refreshAudioDevices]);

  useEffect(() => {
    if (settings) {
      setDraft(settings);
    }
  }, [settings]);

  const streamingModel = useMemo(
    () => models.find((model) => model.kind === "streaming-asr"),
    [models],
  );
  const vadModel = useMemo(
    () => models.find((model) => model.kind === "vad"),
    [models],
  );
  const polishModel = useMemo(
    () => models.find((model) => model.kind === "polish-llm"),
    [models],
  );

  if (!draft) {
    return null;
  }

  const handleChange = <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K],
  ) => {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const handleSave = async () => {
    if (!draft) {
      return;
    }
    await updateSettings(draft);
    toggleSettings(false);
  };

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-6">
      <div className="max-h-[90vh] w-[720px] max-w-full overflow-y-auto rounded-2xl bg-[#0f172a] p-8 text-slate-200 shadow-2xl">
        <header className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">Settings</h2>
          <button
            type="button"
            className="rounded-full bg-white/5 px-3 py-1 text-sm text-white hover:bg-white/10"
            onClick={() => toggleSettings(false)}
          >
            Close
          </button>
        </header>

        <div className="mt-6 space-y-6">
          <GeneralSection draft={draft} onChange={handleChange} />
          <SpeechSection draft={draft} onChange={handleChange} />
          <AudioSection
            draft={draft}
            audioDevices={audioDevices}
            processingMode={processingMode}
            onChange={handleChange}
            onRefresh={refreshAudioDevices}
          />
          <AutocleanSection draft={draft} onChange={handleChange} />
          <ModelSection
            streamingModel={streamingModel}
            vadModel={vadModel}
            polishModel={polishModel}
            onInstallStreaming={() => {
              void installStreamingModel();
            }}
            onInstallVad={() => {
              void installVadModel();
            }}
            onInstallPolish={() => {
              void installPolishModel();
            }}
            onUninstallStreaming={() => {
              void uninstallStreamingModel();
            }}
            onUninstallVad={() => {
              void uninstallVadModel();
            }}
            onUninstallPolish={() => {
              void uninstallPolishModel();
            }}
          />
          <DiagnosticsSection
            draft={draft}
            onToggle={(value) => handleChange("debugTranscripts", value)}
            performanceTest={performanceTest}
            setPerformanceTest={setPerformanceTest}
            transcriptionTest={transcriptionTest}
            setTranscriptionTest={setTranscriptionTest}
            simulatePerformance={simulatePerformance}
            simulateTranscription={simulateTranscription}
            startDictation={startDictation}
            markDictationProcessing={markDictationProcessing}
            completeDictation={completeDictation}
            hudState={hudState}
            metrics={metrics}
            lastTranscript={lastTranscript}
            toggleLogViewer={toggleLogViewer}
            setLogs={setLogs}
          />
        </div>

        <footer className="mt-8 flex justify-end gap-3">
          <button
            type="button"
            className="rounded-md border border-white/10 px-4 py-2 text-sm text-slate-300 hover:bg-white/10"
            onClick={() => toggleSettings(false)}
          >
            Cancel
          </button>
          <button
            type="button"
            className="rounded-md bg-cyan-500 px-4 py-2 text-sm font-semibold text-slate-950 hover:bg-cyan-400"
            onClick={handleSave}
          >
            Save changes
          </button>
        </footer>
      </div>
    </div>
  );
};

const GeneralSection = ({
  draft,
  onChange,
}: {
  draft: AppSettings;
  onChange: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
}) => (
  <section>
    <h3 className="text-lg font-medium text-white">General</h3>
    <div className="mt-3 grid gap-3">
      <label className="flex items-center justify-between gap-3">
        <span>Hotkey Mode</span>
        <select
          className="rounded-md bg-slate-900 px-3 py-2"
          value={draft.hotkeyMode}
          onChange={(event) =>
            onChange("hotkeyMode", event.target.value as AppSettings["hotkeyMode"])
          }
        >
          <option value="hold">Hold to Talk</option>
          <option value="toggle">Toggle to Talk</option>
        </select>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>HUD Theme</span>
        <select
          className="rounded-md bg-slate-900 px-3 py-2"
          value={draft.hudTheme}
          onChange={(event) =>
            onChange("hudTheme", event.target.value as AppSettings["hudTheme"])
          }
        >
          <option value="system">System</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
          <option value="high-contrast">High Contrast</option>
        </select>
      </label>
    </div>
  </section>
);

const SpeechSection = ({
  draft,
  onChange,
}: {
  draft: AppSettings;
  onChange: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
}) => (
  <section>
    <h3 className="text-lg font-medium text-white">Speech</h3>
    <div className="mt-3 grid gap-3">
      <label className="flex items-center justify-between gap-3">
        <span>Language</span>
        <select
          className="rounded-md bg-slate-900 px-3 py-2"
          value={draft.language}
          onChange={(event) => onChange("language", event.target.value)}
        >
          <option value="auto">Auto Detect</option>
          <option value="en">English</option>
          <option value="es">Spanish</option>
          <option value="de">German</option>
          <option value="fr">French</option>
        </select>
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={draft.autoDetectLanguage}
          onChange={(event) => onChange("autoDetectLanguage", event.target.checked)}
        />
        Enable automatic language detection (when supported)
      </label>
    </div>
  </section>
);

const AudioSection = ({
  draft,
  audioDevices,
  processingMode,
  onChange,
  onRefresh,
}: {
  draft: AppSettings;
  audioDevices: AudioDevice[];
  processingMode: AudioProcessingModeState;
  onChange: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
  onRefresh: () => Promise<void>;
}) => (
  <section>
    <div className="flex items-center justify-between">
      <h3 className="text-lg font-medium text-white">Audio</h3>
      <button
        type="button"
        className="rounded bg-white/10 px-2 py-1 text-xs uppercase text-white hover:bg-white/20"
        onClick={() => {
          void onRefresh();
        }}
      >
        Refresh Devices
      </button>
    </div>
    <div className="mt-3 grid gap-3">
      <label className="flex items-center justify-between gap-3">
        <span>Input Device</span>
        <select
          className="w-56 rounded-md bg-slate-900 px-3 py-2"
          value={draft.audioDeviceId ?? ""}
          onChange={(event) =>
            onChange(
              "audioDeviceId",
              event.target.value === "" ? null : event.target.value,
            )
          }
        >
          <option value="">System Default</option>
          {audioDevices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
              {device.isDefault ? " (Default)" : ""}
            </option>
          ))}
        </select>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>Processing Mode</span>
        <select
          className="w-56 rounded-md bg-slate-900 px-3 py-2"
          value={draft.processingMode}
          onChange={(event) =>
            onChange(
              "processingMode",
              event.target.value as AppSettings["processingMode"],
            )
          }
        >
          <option value="standard">Standard (WebRTC APM)</option>
          <option value="enhanced">Enhanced (+ dtln denoise)</option>
        </select>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>VAD Sensitivity</span>
        <select
          className="w-56 rounded-md bg-slate-900 px-3 py-2"
          value={draft.vadSensitivity}
          onChange={(event) =>
            onChange(
              "vadSensitivity",
              event.target.value as AppSettings["vadSensitivity"],
            )
          }
        >
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </label>
      <p className="text-xs text-slate-400">
        Standard mode runs the WebRTC audio processing chain. Enhanced mode adds the
        heavier denoiser; we automatically fall back to Standard if the system is under
        load. Currently running: <span className="font-medium text-slate-200">
          {processingMode.effective === "enhanced" ? "Enhanced" : "Standard"}
        </span>
        {processingMode.preferred !== processingMode.effective
          ? " (temporary fallback)."
          : "."}
      </p>
    </div>
  </section>
);

const AutocleanSection = ({
  draft,
  onChange,
}: {
  draft: AppSettings;
  onChange: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
}) => (
  <section>
    <h3 className="text-lg font-medium text-white">Autoclean</h3>
    <div className="mt-3 grid gap-3">
      <label className="flex items-center justify-between gap-3">
        <span>Mode</span>
        <select
          className="rounded-md bg-slate-900 px-3 py-2"
          value={draft.autocleanMode}
          onChange={(event) =>
            onChange(
              "autocleanMode",
              event.target.value as AppSettings["autocleanMode"],
            )
          }
        >
          <option value="off">Off</option>
          <option value="fast">Fast (Tier-1)</option>
          <option value="polish" disabled={!draft.polishModelReady}>
            Polish (Tier-2)
          </option>
          <option value="cloud">Cloud (BYO)</option>
        </select>
      </label>
      {!draft.polishModelReady && (
        <p className="text-xs text-slate-400">
          Download a local polish model to enable Tier-2 refinement.
        </p>
      )}
    </div>
  </section>
);

const ModelSection = ({
  streamingModel,
  vadModel,
  polishModel,
  onInstallStreaming,
  onInstallVad,
  onInstallPolish,
  onUninstallStreaming,
  onUninstallVad,
  onUninstallPolish,
}: {
  streamingModel: ModelRecord | undefined;
  vadModel: ModelRecord | undefined;
  polishModel: ModelRecord | undefined;
  onInstallStreaming: () => void;
  onInstallVad: () => void;
  onInstallPolish: () => void;
  onUninstallStreaming: () => void;
  onUninstallVad: () => void;
  onUninstallPolish: () => void;
}) => (
  <section>
    <h3 className="text-lg font-medium text-white">Models & Downloads</h3>
    <div className="mt-3 space-y-3">
      {renderModelRow({
        title: "Streaming ASR (Zipformer)",
        description: "Enables real-time transcription via sherpa-onnx.",
        record: streamingModel,
        onInstall: onInstallStreaming,
        onUninstall: onUninstallStreaming,
      })}
      {renderModelRow({
        title: "Voice Activity Detection (Silero)",
        description: "Improves speech gating and noise handling.",
        record: vadModel,
        onInstall: onInstallVad,
        onUninstall: onUninstallVad,
      })}
      {renderModelRow({
        title: "Polish LLM (Tier-2)",
        description: "Enables optional llama.cpp cleanup for the Polish autoclean mode.",
        record: polishModel,
        onInstall: onInstallPolish,
        onUninstall: onUninstallPolish,
      })}
    </div>
  </section>
);

const DiagnosticsSection = ({
  draft,
  onToggle,
  performanceTest,
  setPerformanceTest,
  transcriptionTest,
  setTranscriptionTest,
  simulatePerformance,
  simulateTranscription,
  startDictation,
  markDictationProcessing,
  completeDictation,
  hudState,
  metrics,
  lastTranscript,
  toggleLogViewer,
  setLogs,
}: {
  draft: AppSettings;
  onToggle: (value: boolean) => void;
  performanceTest: { latencyMs: number; cpuPercent: number };
  setPerformanceTest: React.Dispatch<
    React.SetStateAction<{ latencyMs: number; cpuPercent: number }>
  >;
  transcriptionTest: { text: string; latencyMs: number; cpuPercent: number };
  setTranscriptionTest: React.Dispatch<
    React.SetStateAction<{ text: string; latencyMs: number; cpuPercent: number }>
  >;
  simulatePerformance: (latencyMs: number, cpuPercent: number) => Promise<void>;
  simulateTranscription: (
    text: string,
    latencyMs?: number,
    cpuPercent?: number,
  ) => Promise<void>;
  startDictation: () => Promise<void>;
  markDictationProcessing: () => Promise<void>;
  completeDictation: () => Promise<void>;
  hudState: string;
  metrics: PerformanceMetrics | null;
  lastTranscript: string;
  toggleLogViewer: (value?: boolean) => void;
  setLogs: (logs: string[]) => void;
}) => (
  <section>
    <h3 className="text-lg font-medium text-white">Diagnostics</h3>
    <label className="mt-3 flex items-center gap-2 text-sm">
      <input
        type="checkbox"
        checked={draft.debugTranscripts}
        onChange={(event) => onToggle(event.target.checked)}
      />
      Enable debug transcripts (auto-disables after 24h)
    </label>
    <DiagnosticsControls
      performanceTest={performanceTest}
      setPerformanceTest={setPerformanceTest}
      transcriptionTest={transcriptionTest}
      setTranscriptionTest={setTranscriptionTest}
      simulatePerformance={simulatePerformance}
      simulateTranscription={simulateTranscription}
      startDictation={startDictation}
      markDictationProcessing={markDictationProcessing}
      completeDictation={completeDictation}
      hudState={hudState}
      metrics={metrics}
      lastTranscript={lastTranscript}
      toggleLogViewer={toggleLogViewer}
      setLogs={setLogs}
    />
  </section>
);

const DiagnosticsControls = ({
  performanceTest,
  setPerformanceTest,
  transcriptionTest,
  setTranscriptionTest,
  simulatePerformance,
  simulateTranscription,
  startDictation,
  markDictationProcessing,
  completeDictation,
  hudState,
  metrics,
  lastTranscript,
  toggleLogViewer,
  setLogs,
}: {
  performanceTest: { latencyMs: number; cpuPercent: number };
  setPerformanceTest: React.Dispatch<
    React.SetStateAction<{ latencyMs: number; cpuPercent: number }>
  >;
  transcriptionTest: { text: string; latencyMs: number; cpuPercent: number };
  setTranscriptionTest: React.Dispatch<
    React.SetStateAction<{ text: string; latencyMs: number; cpuPercent: number }>
  >;
  simulatePerformance: (latencyMs: number, cpuPercent: number) => Promise<void>;
  simulateTranscription: (
    text: string,
    latencyMs?: number,
    cpuPercent?: number,
  ) => Promise<void>;
  startDictation: () => Promise<void>;
  markDictationProcessing: () => Promise<void>;
  completeDictation: () => Promise<void>;
  hudState: string;
  metrics: PerformanceMetrics | null;
  lastTranscript: string;
  toggleLogViewer: (value?: boolean) => void;
  setLogs: (logs: string[]) => void;
}) => (
  <div className="mt-4 flex flex-col gap-3 text-sm">
    <div className="flex flex-wrap gap-2 text-sm">
      <button
        type="button"
        className="rounded-md bg-white/10 px-3 py-2 text-white hover:bg-white/20"
        onClick={() => {
          void startDictation().catch(console.error);
        }}
      >
        Simulate Listening
      </button>
      <button
        type="button"
        className="rounded-md bg-white/10 px-3 py-2 text-white hover:bg-white/20"
        onClick={() => {
          void markDictationProcessing().catch(console.error);
        }}
      >
        Simulate Processing
      </button>
      <button
        type="button"
        className="rounded-md bg-white/10 px-3 py-2 text-white hover:bg-white/20"
        onClick={() => {
          void completeDictation().catch(console.error);
        }}
      >
        Reset HUD
      </button>
      <div className="flex items-center gap-3 rounded-md bg-white/5 px-3 py-2">
        <label className="flex flex-col text-xs text-slate-300">
          Latency (ms)
          <input
            type="number"
            min={0}
            className="mt-1 w-24 rounded bg-slate-900 px-2 py-1 text-white"
            value={performanceTest.latencyMs}
            onChange={(event) =>
              setPerformanceTest((prev) => ({
                ...prev,
                latencyMs: Number(event.target.value),
              }))
            }
          />
        </label>
        <label className="flex flex-col text-xs text-slate-300">
          CPU %
          <input
            type="number"
            min={0}
            max={100}
            className="mt-1 w-20 rounded bg-slate-900 px-2 py-1 text-white"
            value={performanceTest.cpuPercent}
            onChange={(event) =>
              setPerformanceTest((prev) => ({
                ...prev,
                cpuPercent: Number(event.target.value),
              }))
            }
          />
        </label>
        <button
          type="button"
          className="rounded bg-cyan-500 px-3 py-2 text-slate-900 hover:bg-cyan-400"
          onClick={() => {
            void simulatePerformance(
              performanceTest.latencyMs,
              performanceTest.cpuPercent,
            ).catch(console.error);
          }}
        >
          Perf Alert
        </button>
      </div>
      <div className="grid w-full gap-3 rounded-md bg-white/5 p-3 text-xs text-slate-200">
        <div className="flex flex-wrap items-center gap-3">
          <span className="font-semibold text-slate-200">Live Metrics</span>
          <span className="rounded-full bg-slate-900 px-2 py-1 text-[0.7rem] uppercase tracking-wide text-slate-300">
            {metrics?.performanceMode ? "Performance Mode" : "Normal"}
          </span>
          {import.meta.env.DEV && (
            <button
              type="button"
              className="rounded bg-white/10 px-2 py-1 text-[0.65rem] uppercase text-white hover:bg-white/20"
              onClick={() => {
                void (async () => {
                  try {
                    const snapshot = await invoke<string[]>("get_logs");
                    if (Array.isArray(snapshot)) {
                      setLogs(snapshot);
                    }
                  } catch (error) {
                    console.error("Failed to fetch logs", error);
                  }
                  toggleLogViewer(true);
                })();
              }}
            >
              View Logs
            </button>
          )}
        </div>
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
          <MetricTile label="Latency" value={`${metrics?.lastLatencyMs ?? "—"} ms`} />
          <MetricTile
            label="CPU"
            value={metrics ? `${metrics.averageCpuPercent.toFixed(1)} %` : "—"}
          />
          <MetricTile
            label="Slow Count"
            value={metrics ? String(metrics.consecutiveSlow) : "—"}
          />
          <MetricTile label="HUD State" value={hudState.replace("-", " ")} />
        </div>
      </div>
      <div className="flex w-full flex-col gap-2 rounded-md bg-white/5 p-3 text-xs text-slate-200">
        <div className="flex w-full flex-wrap items-center gap-3">
          <label className="flex flex-1 flex-col gap-1">
            Sample Text
            <input
              type="text"
              className="rounded bg-slate-900 px-3 py-2 text-white"
              value={transcriptionTest.text}
              onChange={(event) =>
                setTranscriptionTest((prev) => ({
                  ...prev,
                  text: event.target.value,
                }))
              }
            />
          </label>
          <label className="flex flex-col">
            Latency (ms)
            <input
              type="number"
              min={0}
              className="mt-1 w-24 rounded bg-slate-900 px-2 py-1 text-white"
              value={transcriptionTest.latencyMs}
              onChange={(event) =>
                setTranscriptionTest((prev) => ({
                  ...prev,
                  latencyMs: Number(event.target.value),
                }))
              }
            />
          </label>
          <label className="flex flex-col">
            CPU %
            <input
              type="number"
              min={0}
              max={100}
              className="mt-1 w-20 rounded bg-slate-900 px-2 py-1 text-white"
              value={transcriptionTest.cpuPercent}
              onChange={(event) =>
                setTranscriptionTest((prev) => ({
                  ...prev,
                  cpuPercent: Number(event.target.value),
                }))
              }
            />
          </label>
          <button
            type="button"
            className="rounded bg-emerald-400 px-3 py-2 text-slate-900 hover:bg-emerald-300"
            onClick={() => {
              void simulateTranscription(
                transcriptionTest.text,
                transcriptionTest.latencyMs,
                transcriptionTest.cpuPercent,
              ).catch(console.error);
            }}
          >
            Run Transcription
          </button>
        </div>
        <div className="flex flex-col gap-1 text-slate-300">
          <span className="font-semibold text-slate-200">Last Output</span>
          <div className="rounded bg-slate-900 p-3 text-slate-100">
            {lastTranscript || "—"}
          </div>
        </div>
      </div>
    </div>
  </div>
);

function renderModelRow({
  title,
  description,
  record,
  onInstall,
  onUninstall,
}: {
  title: string;
  description: string;
  record: ModelRecord | undefined;
  onInstall: () => void;
  onUninstall: () => void;
}) {
  const status: ModelStateKind = record?.status ?? { state: "notInstalled" };
  let statusLabel = "Not Installed";
  let statusDetail: string | undefined;
  let installLabel = "Install";
  let installDisabled = false;
  let uninstallDisabled = status.state !== "installed";
  let progressValue = 0;

  switch (status.state) {
    case "installed":
      statusLabel = "Installed";
      installLabel = "Reinstall";
      break;
    case "downloading":
      progressValue = status.progress;
      statusLabel = `Downloading ${Math.round(progressValue * 100)}%`;
      installLabel = "Downloading…";
      installDisabled = true;
      uninstallDisabled = true;
      break;
    case "error":
      statusLabel = "Error";
      statusDetail = status.message;
      installLabel = "Retry Install";
      break;
    default:
      statusLabel = "Not Installed";
  }

  const sizeText = formatBytes(record?.sizeBytes ?? 0);
  const checksumText = record?.checksum ? record.checksum.slice(0, 12) : "—";

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-4">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="text-sm font-semibold text-white">{title}</p>
          <p className="text-xs text-slate-300">{description}</p>
          <div className="mt-2 flex flex-wrap gap-3 text-xs text-slate-300">
            <span className="rounded bg-white/10 px-2 py-1">
              Status: <span className="font-medium text-white">{statusLabel}</span>
            </span>
            <span className="rounded bg-white/10 px-2 py-1">
              Size: <span className="font-medium text-white">{sizeText}</span>
            </span>
            <span className="rounded bg-white/10 px-2 py-1">
              Checksum: <span className="font-mono text-white">{checksumText}</span>
            </span>
          </div>
          {statusDetail && (
            <p className="mt-2 text-xs text-amber-300">{statusDetail}</p>
          )}
          {status.state === "downloading" && (
            <div className="mt-3 h-2 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-cyan-400 transition-all"
                style={{
                  width: `${Math.min(100, Math.max(0, progressValue * 100)).toFixed(1)}%`,
                }}
              />
            </div>
          )}
        </div>
        <div className="mt-3 flex-shrink-0 md:mt-0">
          <div className="flex gap-2">
            <button
              type="button"
              className="rounded-md bg-cyan-500 px-3 py-2 text-sm font-medium text-slate-900 hover:bg-cyan-400 disabled:cursor-not-allowed disabled:bg-white/20 disabled:text-slate-400"
              onClick={onInstall}
              disabled={installDisabled}
            >
              {installLabel}
            </button>
            <button
              type="button"
              className="rounded-md border border-white/20 px-3 py-2 text-sm text-slate-200 hover:bg-white/10 disabled:cursor-not-allowed disabled:border-white/10 disabled:text-slate-500"
              onClick={onUninstall}
              disabled={uninstallDisabled}
            >
              Uninstall
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

const MetricTile = ({ label, value }: { label: string; value: string }) => (
  <div className="rounded bg-white/5 p-3">
    <p className="text-xs uppercase tracking-wide text-slate-400">{label}</p>
    <p className="mt-1 text-lg font-semibold text-white">{value}</p>
  </div>
);

function formatBytes(bytes: number): string {
  if (!bytes) {
    return "—";
  }
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

export default SettingsPanel;
