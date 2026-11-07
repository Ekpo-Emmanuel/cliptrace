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
| DB | SQLite via `rusqlite` (bundled) + FTS5 + `sqlite-vec` |
| Embeddings | `fastembed` with `bge-small-en-v1.5` (384 dims) |
| Time parsing | `chrono` + small custom parser in `timeparse.rs` |
| Secret storage | DPAPI (`CryptProtectData`) per-value encryption |
| Config | `toml` + `serde`, paths via `dirs` |
| UI (last) | Tauri v2, plain TS or Svelte |

Fallback: if `ort`/ONNX or `sqlite-vec` loading blocks progress for more than
~90 minutes, stop and propose a switch to C#/.NET 8. Architecture is identical.

## Data location
Root: `%LOCALAPPDATA%\Cliptrace\` (override with env var `CLIPTRACE_DATA_DIR` for dev/tests)

```
%LOCALAPPDATA%\Cliptrace\
  cliptrace.db      # SQLite database
  config.toml       # blocklist, retention, hotkey
  models\           # cached embedding model files
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
  search_text, content='clips', content_rowid='id', tokenize='porter unicode61'
);
CREATE VIRTUAL TABLE clip_vec USING vec0(embedding float[384]);  -- rowid = clips.id
```
- Dedupe: on hash match, bump `last_seen_at` and `copy_count`; do not insert a new row.
- `search_text` for normal clips = content (truncated to ~2k chars for embedding).
- `search_text` for secrets = context only, e.g. `stripe api key copied from chrome.exe, "Stripe Dashboard - API keys"`. The raw key never enters FTS or the vector index.
- Purge job: delete rows where `expires_at < now` (default secret TTL 24h).

### embed (M4)
- `fastembed` bge-small, model cached in `models\`.
- Embed on a background worker so capture never blocks.
- `cliptrace backfill` embeds any clip missing a vector.

### search (M3, M5)
1. `timeparse` extracts a time range from the query ("tuesday", "yesterday",
   "last week", "this morning", "3 days ago") and returns the remaining text.
   Bare weekday = most recent past occurrence.
2. Run FTS5 (BM25) and vector KNN over the remaining text, both filtered by the time range.
3. Merge with reciprocal rank fusion (k = 60). Return top 5: id, time, source app,
   preview (secrets shown masked, e.g. `sk_live_••••••••`).
4. `cliptrace copy <id>` puts the value back on the clipboard (decrypting secrets via DPAPI).

### secrets (M6)
- Detect: known prefixes (`sk_live_`, `sk_test_`, `ghp_`, `AKIA`, `xox[bp]-`, JWT shape,
  PEM headers) + high-entropy token heuristic (length >= 20, Shannon entropy threshold).
- On detection: `is_secret = 1`, encrypt value with DPAPI, build context-only
  `search_text`, set `expires_at`.
- Config: `blocklist` (exe names), `secret_ttl_hours`, `retention_days` (default 90 for normal clips).

## Milestones and acceptance criteria
Work one milestone at a time. Commit after each one passes.

| # | Slice | Acceptance |
|---|---|---|
| M1 | Capture listener | `cliptrace watch` prints timestamp, exe, window title, and first 60 chars for every copy; password-manager copy prints nothing |
| M2 | SQLite storage | Clips persist across restarts; duplicate copy bumps `copy_count`; blocklisted app is skipped |
| M3 | CLI keyword search | `cliptrace search "stripe"` returns ranked results via FTS5 with preview and time |
| M4 | Embeddings | New clips get vectors without blocking capture; `backfill` works |
| M5 | Hybrid + time | `cliptrace search "stripe key from tuesday"` returns the right clip in the top 5 |
| M6 | Secrets | Fake `sk_live_...` copy is stored encrypted, not findable by its characters, findable by "stripe key", purged after TTL |
| M7 | Popup UI | Win+Shift+V opens a search box, Enter copies the result back and closes |

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
