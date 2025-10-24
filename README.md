# Push-to-Talk STT

Initial scaffolding for the Push-to-Talk speech-to-text desktop application described in the PRD.

## Structure

- `app/`: Frontend (React + TypeScript + Vite) and Tauri configuration.
- `app/src-tauri/`: Rust backend with module stubs for audio, ASR, VAD, autoclean, models, and output pipelines. `tauri.conf.json` lives here so Windows clones don't need symlink support.
- `ci/`: Placeholder automation scripts for native dependency builds and signing.

## Windows setup

1. Install the [Microsoft Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the *Desktop development with C++* workload, and ensure the Windows 10/11 SDK option is checked.
2. Install the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section) (Tauri uses it to render the UI).
3. Install [rustup](https://rustup.rs/) and run `rustup default stable` so a recent stable toolchain is available.
4. Install the [LLVM toolchain](https://releases.llvm.org/download.html) (or `winget install LLVM.LLVM`) so `libclang.dll` and `llvm-nm.exe` are available. After installing:  
   - PowerShell (current session):  
     ```
     $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
     $env:PATH = "$env:PATH;C:\Program Files\LLVM\bin"
     ```
   - To persist between sessions:  
     ```
     [Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")
     [Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Program Files\LLVM\bin", "User")
     ```
     Alternatively set `NM_PATH` directly: `[Environment]::SetEnvironmentVariable("NM_PATH", "C:\Program Files\LLVM\bin\llvm-nm.exe", "User")`.
5. Install [MSYS2](https://www.msys2.org/) (provides `libtoolize`, `pkg-config`, etc.). From an MSYS2 UCRT64 shell run:  
   `pacman -Syu --noconfirm` (twice if prompted), then  
   `pacman -S --needed --noconfirm mingw-w64-ucrt-x86_64-{toolchain,libtool,pkg-config,autoconf,automake}`  
   Add `C:\msys64\ucrt64\bin` and `C:\msys64\usr\bin` to your system `PATH` so Cargo can find these tools when building.  
6. Install [Node.js 18+](https://nodejs.org/) and enable Corepack (`corepack enable`) or install Yarn globally (`npm install -g yarn`).
7. Inside `app/`, run `yarn install` to fetch the frontend dependencies.

Once the prerequisites are installed you can iterate entirely from PowerShell or Windows Terminal:

- `yarn tauri dev` (or `cargo tauri dev`) launches the dev build with hot reload for the React frontend and Rust backend.
- `yarn tauri build` produces a signed bundle (MSI/MSIX by default) that you can double-click to install.
- When making iterative changes, leave `yarn tauri dev` running; edits in `app/src` or `app/src-tauri` will rebuild automatically.
- If the Rust build fails because `yarn` didn't generate the Vite assets yet, run `yarn build` once or restart `yarn tauri dev`.

## Development (planned cross-platform)

1. Install the Rust toolchain (Rust 1.78+) and Tauri prerequisites.
2. Install Node dependencies with `yarn install`.
3. Run the combined dev environment with `yarn tauri dev`.

The backend currently exposes foundational settings management and IPC hooks; audio/ASR pipelines are stubbed pending native integration.
