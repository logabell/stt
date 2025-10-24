# Push-to-Talk STT

Initial scaffolding for the Push-to-Talk speech-to-text desktop application described in the PRD.

## Structure

- `app/`: Frontend (React + TypeScript + Vite) and Tauri configuration.
- `app/src-tauri/`: Rust backend with module stubs for audio, ASR, VAD, autoclean, models, and output pipelines.
- `ci/`: Placeholder automation scripts for native dependency builds and signing.

## Development (planned)

1. Install the Rust toolchain (Rust 1.78+) and Tauri prerequisites.
2. Install Node dependencies with `yarn install`.
3. Run the combined dev environment with `yarn tauri dev`.

The backend currently exposes foundational settings management and IPC hooks; audio/ASR pipelines are stubbed pending native integration.
