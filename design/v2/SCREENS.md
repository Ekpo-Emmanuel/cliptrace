# Cliptrace — Screens and states

Each HTML file is a self-contained snapshot of one screen. Open it in a browser. The **light / dark** button in the top-right corner switches theme; you can also add `?theme=light` to the address. Collapse/expand buttons link each full view to its quick-paste partner.

Two window sizes:
- **Full view**: 880 × 580, history trace + preview.
- **Quick paste**: 380 × 580, opens near the cursor, numbered for Alt+1–9.

---

## Core

| File | Name | What it shows |
|---|---|---|
| `recent.html` | Recent — full view | All 109 clips on the trace, grouped by day. The newest text clip is selected and the trace is lit down to it. The preview shows the text large, with the action cluster, Paste, tags and the stats strip. |
| `recent-compact.html` | Recent — quick paste | Pinned clips first, then recent, numbered 1–9. The first is selected. Footer: ↵ Paste · Alt 1–9 · Tab Expand. |
| `pinned.html` | Pinned — full view | The segmented toggle is on **pinned**, so only the 2 pinned clips are listed. Their nodes are diamonds. |

## Clip types in the preview

| File | Name | What it shows |
|---|---|---|
| `link-item.html` | Link selected | The site name ("quo.com") in large type, the full URL in mono, and "open in browser ↗". |
| `secret-item.html` | Secret selected | A redacted password: hatch bars, "hold to reveal", and "encrypted · kept out of search · self-clears in 25:00". The Secret icon is lit. |
| `code-item.html` | Code selected | A Rust snippet in mono with amber line numbers. |
| `image-item.html` | Image selected | A 16:9 screenshot frame with amber crop marks and its size. |

## Empty states

| File | Name | What it shows |
|---|---|---|
| `empty.html` | Empty history — full view | No clips yet. A breathing "now" node with a dashed trace below it. "Nothing on the trace yet." A three-step how-to (Ctrl C → Win Shift V → ↵) and a sample line with **Copy this**. |
| `empty-first-copy.html` | Empty history — first copy | The same screen after the sample is copied: the first point lands on the trace, step 01 is struck through, and the headline reads "There it is. Press Enter to paste it." |
| `empty-compact.html` | Empty history — quick paste | A compact version: headline, three steps with key caps, and the sample line. |
| `empty-pins.html` | No pins — full view | The pinned toggle is on, with 0 pins. Six numbered hollow-diamond slots fade down the trace. "Nothing pinned." Explains Ctrl P and the Alt 1–9 slots. **Browse all 109 clips**. |
| `empty-pins-compact.html` | No pins — quick paste | The Pinned section is a single dashed "No pins yet — Ctrl P" slot. Recent clips still fill the window. |
| `no-results.html` | No results — full view | The search "invoice march" finds nothing. The trace stops at an amber ×. "No clips match “invoice march”", tips (`app:chrome`, `#meeting`, fewer words), "Secrets are never searched", **Clear search (Esc)**. |
| `no-results-compact.html` | No results — quick paste | The same, compact, with a clear (×) in the search bar. |

## Interaction states

| File | Name | What it shows |
|---|---|---|
| `multi-select.html` | Multi-select — full view | Checkboxes on. Trace nodes become square checkboxes; 3 are checked. The header is replaced by a selection bar: × · 3 selected of 109 · select all / none · Pin · Tag · **Delete 3**. The preview fans the 3 selected clips as cards above "3 clips" and **Delete 3 clips**. |
| `multi-select-compact.html` | Multi-select — quick paste | Checkbox nodes. The footer becomes a selection bar: × · 3 selected · all · **Delete**. |
| `tags.html` | Tag suggest — full view | `#` typed in search. A list opens under the field: tags with the typed prefix in amber, a usage bar, a count, and key hints. The list behind is already narrowed to tagged clips. |
| `tags-compact.html` | Tag suggest — quick paste | The same list at the full width of the window. |
| `secret-confirm.html` | Mark secret confirm — full view | A dialog over the dimmed window: "Lock this clip as a secret?", the clip with an animated encryption hatch sweeping over it, three facts, a red **one-way** warning, Cancel (Esc) / **Encrypt & lock** (↵). |
| `secret-confirm-compact.html` | Mark secret confirm — quick paste | The same content as a bottom sheet. |
| `delete-undo.html` | Delete undo — full view | A floating strip: "Deleted “quo.com/pricing”", a 5s countdown with a draining amber bar, **Undo ^Z**, ×. The deleted clip stays on the trace, struck through with a dashed red node, until the timer ends. |
| `delete-undo-compact.html` | Delete undo — quick paste | The same strip, sitting above the footer. |
| `error.html` | Error strip — full view | A red-tinted strip: "Couldn't copy to the clipboard" / "another app has the clipboard open · 0x800401D0", **Retry ↵**, ×. The Copy icon turns red. The pin and tag variants use the same strip with "Couldn't pin this clip" / "Couldn't save the tag" and "history database is busy · SQLITE_BUSY". |
| `error-compact.html` | Error strip — quick paste | The same strip, above the footer. |
| `paused.html` | Capture paused — full view | The tray's Pause is on. The wordmark dims, an amber "❚❚ paused" tag appears, and a hatched band reads "Capture paused · since 04:31 · copies aren't saved" with **Resume**. The top of the trace is dashed. |
| `paused-compact.html` | Capture paused — quick paste | The same band under the header. The "ct" mark dims. |

---

## Notes and placeholders

- Sample clips, counts (109 / 7), tag counts, times and error codes are mock data.
- **Win + Shift + V** as the open shortcut, **nine pin slots** (Alt 1–9), **30-minute secret self-clear** and **"every word must match" search** are assumptions. Change them to match the app.
- The HTML files are static snapshots for spec and hand-off. The interactive prototypes (clicking rows, the live undo countdown, Retry, Resume, Encrypt) are on the design canvas.
