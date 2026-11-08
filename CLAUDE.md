# Cliptrace

Local-only Windows clipboard history with semantic search. Rust workspace plus Tauri popup plus React UI.

Read `SPEC.md` before doing anything. It is the source of truth for architecture, schema and milestones. Read `design/v2/DESIGN.md` before any UI change.

## Layout
- `crates/cliptrace-core`: library (capture, store, embed, search, timeparse, secrets, config).
- `crates/cliptrace-cli`: CLI binary.
- `src-tauri`: tray, hotkey, popup window, Tauri commands.
- `ui/`: React + TS + Tailwind (Vite). `ui/src/api.ts` is the only `invoke` caller. `ui/src/mock.ts` serves browser-only dev.

## Process
- Work on exactly ONE milestone at a time (M1..M8b in SPEC.md). Do not start the next until the user confirms the current one passes its acceptance criteria.
- Before writing code for a milestone, state in 3-5 bullets which files you will create or change.
- Keep changes small and runnable. After each step, give the exact command to run and the expected output.
- Do not add dependencies outside the SPEC stack table without asking. Say why when proposing one.
- Do not refactor or reformat code unrelated to the current milestone.
- If something fails twice in a row (build error, DLL load), stop and explain the root cause instead of trying a third variation.
- If a dependency blocks progress for long (`ort`/ONNX), stop and tell the user so they can decide whether to switch approaches.

## Code rules
- Rust stable. Windows only. Use the `windows` crate for Win32 calls.
- Keep every `unsafe` block small, commented with why it is sound, and wrapped in a safe function.
- `anyhow` in the binary, `thiserror` in the library. No `unwrap()`/`expect()` outside tests.
- Code that parses time phrases takes `now` as a parameter. Never read the clock inside it.
- Add unit tests for `timeparse` and `secrets`.
- Data paths come from `%LOCALAPPDATA%\Cliptrace\`, overridable with `CLIPTRACE_DATA_DIR`.

## Privacy (non-negotiable)
- Never log clip content. Log only metadata (length, kind, source exe).
- Check clipboard exclusion formats and the blocklist BEFORE reading clipboard text.
- Raw secret values never go in `content`, `search_text`, the FTS index or the vector index.
- Never send raw secrets over IPC or render them in the UI. Masked previews only.
- No network calls except the one-time embedding model download.

## UI
- Two popup modes: Full view 880x580 and Quick paste 380x580. Reference screens: `design/v2/screens/*.html` (`-compact` = Quick paste).
- Dark and light themes both required.
- Fonts (Bricolage Grotesque, Martian Mono) are bundled in `ui/src/assets/fonts/` via `@font-face`. Never load Google Fonts. No network assets.
- Ignore design items that break privacy rules: "hold to reveal" on secrets. Image clips are out of scope for v1.
- Do not invent styles absent from `DESIGN.md`. If something is missing, extend it in the same spirit and tell the user.
- If `DESIGN.md` conflicts with `SPEC.md` or the privacy rules, `SPEC.md` and the privacy rules win. Tell the user about the conflict.

## Commands
- Build and test the core: `cargo test -p cliptrace-core`
- Run the CLI: `cargo run -p cliptrace-cli -- watch`
- UI dev server: `npm --prefix ui run dev`
- UI typecheck and build: `npm --prefix ui run build`
