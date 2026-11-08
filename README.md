# Cliptrace

A clipboard history for Windows that lives on your PC and searches by meaning. Every text you copy is saved and indexed on your machine, so you can type "that Stripe key thing from Tuesday" and get it back.

Nothing leaves your PC. The only network call is a one-time download of the search model on first run.

## Features

- Captures every text copy, with the source app and window title.
- Searches by meaning and keywords together, and understands time phrases such as `yesterday`, `last week` or `3 days ago`.
- Keeps secrets safe. Copies from password managers are skipped. Secret-shaped text (API keys, tokens, private keys) is encrypted with Windows DPAPI, shown masked, and kept out of the search indexes.
- Pin clips so they never expire. Tag clips and filter with `#tag`.
- Two views: a full view (880×580) with a preview pane, and a narrow Quick paste view (380×440).
- Light and dark themes.
- Pause capture, ignore apps, and choose how long history and secrets are kept.

## Install

1. Download `Cliptrace-<version>-win64.zip` from the [latest release](https://github.com/Ekpo-Emmanuel/cliptrace/releases/latest) and unzip it anywhere.
2. Run `cliptrace-app.exe` as administrator. Keep `onnxruntime.dll` and `onnxruntime_providers_shared.dll` in the same folder.
3. Press `Ctrl+Alt+V` to open the popup.

The first launch downloads the search model once, so it needs an internet connection. After that Cliptrace works offline. Until the model is ready, and if the ONNX Runtime files are missing, search falls back to keywords only. Settings → About shows whether search is `hybrid` or `keyword-only`.

The app is not code signed yet, so Windows SmartScreen may warn about an unknown publisher. Choose "More info", then "Run anyway".

Requirements: Windows 10 or 11 with WebView2 (included in Windows 11).

## Using it

Cliptrace runs in the system tray. Press `Ctrl+Alt+V` from any app to open the popup, type to search, and press `Enter` to copy the selected clip. Clicking a clip copies it too. In Quick paste the popup then closes. In full view it stays open.

Search tips:

- Plain words search by meaning: `stripe key`.
- Add a time phrase: `invoice last week`, `meeting notes yesterday`.
- Filter by tag with `#`: `#work`.

### Keyboard shortcuts

| Keys | Action |
|---|---|
| `↑` `↓` | Previous or next clip |
| `PgUp` `PgDn` | Jump six clips |
| `Home` `End` | First or last clip (empty search) |
| `Ctrl+1` `Ctrl+2` | All clips or pinned clips |
| `Ctrl+K` | Focus search |
| `Enter` | Copy and close |
| `Ctrl+P` | Pin or unpin |
| `Ctrl+T` | Add a tag |
| `Ctrl+L` | Mark as secret |
| `Del` | Delete (undo with `Ctrl+Z`) |
| `Shift+↑` `Shift+↓` | Extend the selection |
| `Ctrl+Space` | Check or uncheck a clip |
| `Ctrl+A` | Check all clips (empty search) |
| `Tab` | Switch between full view and Quick paste |
| `Ctrl+,` | Settings |
| `Ctrl+Shift+D` | Switch between light and dark |
| `Esc` | Close or step back |

### Settings

Open Settings with the gear button or `Ctrl+,`.

- **Theme:** system, light or dark.
- **Pause capture:** stop saving copies until you turn it back on.
- **Launch at sign-in:** start Cliptrace with Windows.
- **Secrets self-clear:** delete secrets after 1 hour, 6 hours, 24 hours, 3 days, or never.
- **Keep history for:** 30 days, 90 days, 6 months or 1 year. Pinned clips stay.
- **Ignored apps:** copies from these apps are never saved. Add the exact executable name, for example `notepad.exe`.

The hotkey is changed in `config.toml` (see below).

## Your data

Everything is stored under `%LOCALAPPDATA%\Cliptrace\`. Set the `CLIPTRACE_DATA_DIR` environment variable to use another folder.

| Path | Contents |
|---|---|
| `cliptrace.db` | Your clip history (SQLite) |
| `config.toml` | Blocklist, retention, hotkey, ONNX Runtime path |
| `models\` | The downloaded search model |
| `runtime\` | Optional place for `onnxruntime.dll` |

Cliptrace looks for `onnxruntime.dll` in the path set in `config.toml`, then next to the exe, then in `runtime\`.

## Privacy

- No telemetry, no cloud, no sync.
- Logs never contain clip content.
- Clipboard exclusion formats and the ignored-apps list are checked before any text is read.
- Raw secret values are never stored in plain text, in the search index, or in the vector index. The UI shows masked previews only.

## Build from source

Needs Rust (stable), Node.js, and the WebView2 runtime. You also need the official ONNX Runtime CPU `onnxruntime.dll` (and `onnxruntime_providers_shared.dll`) for hybrid search.

Run the core tests:

```bash
cargo test -p cliptrace-core
```

Run the UI in a browser with mock data:

```bash
npm --prefix ui run dev
```

Run the desktop app in development, with the Vite dev server running:

```bash
cd src-tauri
cargo run
```

Build a release exe that serves the bundled UI:

```bash
npm --prefix ui run build
cd src-tauri
cargo build --release --features custom-protocol
```

The exe is `target\release\cliptrace-app.exe`. Without the `custom-protocol` feature it loads the dev server URL instead of the bundled UI.

### Command line

A CLI exists for development and scripting.

```bash
cargo run -p cliptrace-cli -- watch
cargo run -p cliptrace-cli -- search "stripe key from tuesday"
cargo run -p cliptrace-cli -- list --limit 20
```

Run it with no arguments to see all commands, including `pin`, `tag`, `copy`, `purge`, `backfill`, `autostart` and `no-autostart`.

### Layout

- `crates/cliptrace-core`: capture, store, embeddings, search, time parsing, secrets, config.
- `crates/cliptrace-cli`: the command-line tool.
- `src-tauri`: the tray, hotkey, popup window and Tauri commands.
- `ui/`: the React, TypeScript and Tailwind interface.
- `design/v2/`: the design reference and screens.
- `SPEC.md`: architecture, schema and milestones.

## License

Cliptrace is released under the [MIT License](LICENSE).

## Third-party software

Cliptrace uses [ONNX Runtime](https://github.com/microsoft/onnxruntime) (MIT license) and the `bge-small-en-v1.5` embedding model through `fastembed`. The release zip includes the ONNX Runtime license.
