# Cliptrace — SPEC

A local-only clipboard history for Windows with semantic search.
Every copy is stored and indexed on-device, so you can type
"that Stripe key thing from Tuesday" and get it back.

## Goals
- Capture every text copy on Windows, with source app and window title.
- Search by meaning + keywords + time phrases, entirely offline.
- Never leak secrets: respect password-manager exclusions, flag secret-shaped
  content, keep raw secret values out of the search indexes.

## Non-goals (v1)
- Images / files / rich-text formats (text only; stretch: OCR later)
- Cross-device sync, cloud anything, telemetry
- macOS / Linux support

## Stack (do not deviate without asking)
| Piece | Choice |
|---|---|
| Language | Rust (stable) |
| Workspace | `cliptrace-core` (lib), `cliptrace-cli` (bin), `src-tauri` (M7 only) |
| Win32 | `windows` crate |
| DB | SQLite via `rusqlite` (bundled) + FTS5; embeddings in `clip_embeddings` BLOB |
| Embeddings | `fastembed` `bge-small-en-v1.5` (384), ONNX Runtime loaded dynamically (`onnxruntime.dll`) |
| Time parsing | `chrono` + small custom parser in `timeparse.rs` |
| Secret storage | DPAPI (`CryptProtectData`) per-value encryption |
| Config | `toml` + `serde`, paths via `dirs` |
| frontend | React + TS + Tailwind (Vite) |

Fallback: if `ort`/ONNX loading blocks progress for more than ~90 minutes,
stop and propose a switch to C#/.NET 8. Architecture is identical.

## Data location
Root: `%LOCALAPPDATA%\Cliptrace\` (override with env var `CLIPTRACE_DATA_DIR` for dev/tests)

```
%LOCALAPPDATA%\Cliptrace\
  cliptrace.db      # SQLite database
  config.toml       # blocklist, retention, hotkey, onnxruntime_path
  models\           # cached embedding model files (offline after first download)
  runtime\          # official onnxruntime.dll (CPU build)
  logs\             # rolling logs (never log clip content)
```

Why Local, not Roaming: the DB is machine-specific, can be large, and must not
sync. DPAPI-protected values only decrypt for the same Windows user on the same
machine anyway.

## Architecture

### capture (M1)
- Hidden message-only window + `AddClipboardFormatListener`; handle
  `WM_CLIPBOARDUPDATE`. No polling.
- On each update: skip if clipboard has `ExcludeClipboardContentFromMonitorProcessing`
  or `CanIncludeInClipboardHistory` == 0. Skip if source exe is in blocklist.
- Read `CF_UNICODETEXT`. Ignore empty / whitespace-only / > 1 MB.
- Source app: `GetClipboardOwner` -> PID -> exe name (fallback: foreground window's process).
  Window title: `GetForegroundWindow` -> `GetWindowTextW`.
- Retry opening the clipboard a few times with short backoff (it is often locked).

### store (M2)
Schema:
```sql
CREATE TABLE clips (
  id            INTEGER PRIMARY KEY,
  hash          TEXT NOT NULL UNIQUE,     -- sha256 of normalized text
  first_seen_at INTEGER NOT NULL,         -- unix ms, UTC
  last_seen_at  INTEGER NOT NULL,
  copy_count    INTEGER NOT NULL DEFAULT 1,
  kind          TEXT NOT NULL,            -- url | code | prose | secret | other
  is_secret     INTEGER NOT NULL DEFAULT 0,
  content       TEXT,                     -- plaintext; NULL when is_secret = 1
  content_enc   BLOB,                     -- DPAPI blob; only when is_secret = 1
  search_text   TEXT NOT NULL,            -- what FTS + embeddings are built from
  source_exe    TEXT,
  window_title  TEXT,
  expires_at    INTEGER                   -- unix ms; set for secrets (TTL)
);
CREATE VIRTUAL TABLE clips_fts USING fts5(
  search_text, tags_text, content='clips', content_rowid='id', tokenize='porter unicode61'
);
CREATE TABLE clip_embeddings (
  clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
  model TEXT NOT NULL,                    -- e.g. bge-small-en-v1.5
  embedding BLOB NOT NULL                 -- 384 little-endian f32, L2-normalized
);
```
- Dedupe: on hash match, bump `last_seen_at` and `copy_count`; do not insert a new row.
- `search_text` for normal clips = content (truncated to ~2k chars for embedding).
- `search_text` for secrets = context only, e.g. `stripe api key copied from chrome.exe, "Stripe Dashboard - API keys"`. The raw key never enters FTS or the vector index.
- `pinned_at INTEGER NULL`, `tags_text TEXT NOT NULL DEFAULT ''`.
- `tags(id, name UNIQUE COLLATE NOCASE)` and `clip_tags(clip_id, tag_id)` composite PK, both FKs `ON DELETE CASCADE`.
- Schema migrations use `PRAGMA user_version` (current: 2).
- Keyword rank: `bm25(clips_fts, 1.0, 3.0)` (search_text, tags_text).
- Purge never deletes pinned clips. Pinning a secret clears `expires_at`; unpin restores TTL.
- Tags: trim, collapse whitespace, max 32 chars, max 10 per clip, case-insensitive. `tags_text` stays in sync; FTS + embedding re-queue use search_text plus tags (secrets: context plus tags, never content).
- `mark-secret <id>` converts plaintext in place (DPAPI, content NULL, one-way) with secure-delete/checkpoint.
- `delete <id>` removes clip, FTS, embedding, tag links, then checkpoint/VACUUM.
- `list_recent(offset, limit, filter)` by `last_seen_at DESC`; filter all / pinned / tag. Secrets masked.
- Search optional pinned/tag scope; `#tag` tokens stripped from query text then applied as filters.
- Purge job: delete unpinned rows where `expires_at < now` (default secret TTL 24h).

### embed (M4)
- `fastembed` full `BGESmallENV15` (`bge-small-en-v1.5`), 384 dims, ONNX intra-op threads = 2.
- Load ONNX Runtime dynamically from `config.toml` `onnxruntime_path` (default
  `%LOCALAPPDATA%\Cliptrace\runtime\onnxruntime.dll`). Set `ORT_DYLIB_PATH` and
  prepend that folder to `PATH` before the first model load.
- Cached model files in `models\`. If an `.onnx` file is already there, set
  `HF_HUB_OFFLINE=1` so load makes no network calls.
- Embed on a background worker: load lazily on the first clip, never block capture.
- If the DLL is missing or ONNX fails: one warning; `watch` still stores clips;
  `search` still works.
- Store L2-normalized 384-d vectors in `clip_embeddings` (no sqlite-vec).
- Vector search (`vsearch`) is brute-force cosine (dot product of L2-normalized vectors).
- `cliptrace backfill` embeds clips missing a vector in small batches with a short sleep.
- CLI crate enables the `embeddings` feature by default; `cliptrace-core` keeps it off
  so unit tests do not link ONNX.

### search (M3, M5)
1. `timeparse` extracts a time range from the query ("tuesday", "yesterday",
   "last week", "this morning", "3 days ago") and returns the remaining text.
   Bare weekday = most recent past occurrence. Day bounds are local midnight.
   A clip matches if `first_seen_at <= range_end AND last_seen_at >= range_start`.
2. Keyword: drop English stopwords, quote remaining terms, join with OR, rank with BM25.
3. Vector: embed remaining text (BGE query prefix on the query only) and brute-force
   dot product over time-filtered L2 vectors.
4. Merge with reciprocal rank fusion (k = 60). Default `search` is hybrid.
   `--keyword-only` / `--vector-only` / `--explain` / `--limit N`. If embeddings fail,
   keyword-only plus one warning.
5. Print: id, absolute local time, relative time, source exe, 60-char preview.
6. `cliptrace copy <id>` puts the value back on the clipboard (decrypting secrets via DPAPI).

### popup (M7, M8b)
- One process: tray + clipboard listener + embed worker + hourly purge.
- Named mutex single instance. Borderless always-on-top popup, 760×560, centered, skip taskbar. Click outside (window blur) hides it and flushes pending deletes; ignore blur for ~400ms after show so the open path does not hide immediately.
- Default hotkey `Ctrl+Alt+V` in `config.toml` `hotkey`. Failed register: tray note, keep running.
- frontend: React + TS + Tailwind (Vite). System fonts only. No network assets. Light and dark via `prefers-color-scheme`. Accent is `--accent`.
- Opens on Recent with the first row selected and the search box focused. Empty search browses; typing ranks.
- Tabs: Recent, Pinned (count), Tags. Search stays inside the current tab. `#` offers tag autocomplete.
- List plus preview. Secrets stay masked. Enter copies the real value via `copy_clip` (decrypt in Rust, write clipboard, skip next capture so no duplicate row), toasts “Copied”, then hides.
- Pin, tag, mark-secret (one-way confirm), delete (5-second undo). Optimistic UI; revert with a toast on error. Pending deletes flush on hide and on quit.
- Recent page cached in the app process and pushed on open (`cliptraceOnOpen`) so the list paints in about 100ms, then refreshes. Capture while open emits `clips-changed`; keep selection by id.
- Search debounce 150ms; ignore out-of-order responses. Preview pane loads full non-secret text with `clip_text`; list payloads strip `body` and never send raw secrets over IPC (masked previews only).
- `status()` and tray Pause emit `status-changed` with `{ paused, keyword_only }`. Capture paused and keyword-only (missing embeddings DLL) show in the popup.
- Tauri commands: `browse_clips`, `list_recent`, `search_clips`, `list_tags`, `set_pinned`, `add_tag`, `remove_tag`, `mark_clip_secret`, `delete_clip`, `stage_delete`, `unstage_delete`, `clip_text`, `copy_clip`, `hide_popup`, `status`. Typed errors, no panics. Frontend `api.ts` is the only `invoke` caller; browser mock + `?state=` stay for Vite-only.
- Never show raw secrets. `autostart` / `no-autostart` write HKCU Run.

### secrets (M6)
- Detect: known prefixes (`sk_live_`, `sk_test_`, `rk_live_`, `ghp_`, `github_pat_`,
  `AKIA`+16, `xox[bp]-`, `AIza`, JWT shape, PEM headers, `user:pass@` URLs) plus a
  high-entropy single-token heuristic (length >= 20, Shannon entropy, mixed classes).
- On detection: `is_secret = 1`, `content` NULL, `content_enc` via DPAPI
  (`CryptProtectData`, current-user), context-only `search_text`, `expires_at`.
- Mask previews as `[stripe key sk_t…]`. `copy` decrypts with DPAPI.
- `purge` deletes `expires_at < now` and normal clips older than `retention_days`
  (FTS delete triggers + embedding CASCADE). Watch runs purge at start and hourly.
- `rescan` converts existing plaintext secrets in place and re-queues embeddings
  from the new context text. After convert: FTS rebuild, WAL checkpoint, VACUUM.
- Every connection sets `PRAGMA secure_delete=ON`. `cliptrace scrub` checkpoints,
  rebuilds FTS, and VACUUMs.
- Config: `blocklist` (exe names), `secret_ttl_hours`, `retention_days` (default 90 for normal clips),
  `onnxruntime_path` (default `<data dir>\\runtime\\onnxruntime.dll`).

## Milestones and acceptance criteria
Work one milestone at a time. Commit after each one passes.

| # | Slice | Acceptance |
|---|---|---|
| M1 | Capture listener | `cliptrace watch` prints timestamp, exe, window title, and first 60 chars for every copy; password-manager copy prints nothing |
| M2 | SQLite storage | Clips persist across restarts; duplicate copy bumps `copy_count`; blocklisted app is skipped |
| M3 | CLI keyword search | `cliptrace search "stripe"` returns ranked results via FTS5 with preview and time |
| M4 | Embeddings | New clips get vectors without blocking capture; `backfill` and `vsearch` work; capture/search survive a missing ONNX DLL |
| M5 | Hybrid + time | `cliptrace search "stripe key from tuesday"` returns the right clip in the top 5 |
| M6 | Secrets | Fake `sk_live_...` copy is stored encrypted, not findable by its characters, findable by "stripe key", purged after TTL |
| M7 | Popup UI | Ctrl+Alt+V (configurable `hotkey`) opens a search box, Enter copies the result back and closes |
| M8a | Pins/tags/delete data | pin survives purge; tag FTS + `#tag`; mark-secret/delete wipe plaintext from `.db`/`-wal`; CLI list/tags; no UI yet |
| M8b | Popup redesign | Recent / Pinned / Tags, list plus preview, keyboard actions, light and dark, no raw secrets on screen; React UI talks to Tauri commands (mock only in browser) |

M1-M5 are the core product. If time runs short, cut from the bottom.

## Testing notes
- Unit-test `timeparse` with a fixed "now" injected (never call the system clock inside it).
- Unit-test secret detection with a table of positive and negative samples.
- Use `CLIPTRACE_DATA_DIR` pointing at a temp dir for integration tests.
- Manually verify the privacy path: copy a password from your password manager and confirm nothing is stored.

## Privacy invariants
1. Nothing leaves the machine. No network calls except the one-time embedding model download.
2. Logs never contain clip content.
3. Raw secret values are never in `content`, `search_text`, FTS, or the vector index.
4. Honor clipboard exclusion formats before reading any content.
