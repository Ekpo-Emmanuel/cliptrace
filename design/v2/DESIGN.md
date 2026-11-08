# Cliptrace — Design system

**Concept: the trace.** Your clipboard history is drawn as one vertical line through time. Every clip is a point on it. Selecting a clip lights the line in amber from "now" back to that point. Everything else stays quiet, like an instrument panel at night: a warm near-black surface, one sodium-amber signal colour, and monospaced readouts.

Dark is the primary theme. The light theme is a matching "paper" version with the same structure.

---

## 1. Colour tokens

All colours are CSS custom properties on the app root (`.tr` = full view, `.tq` = quick paste).

| Token | Role | Dark | Light |
|---|---|---|---|
| `--bg` | Window background | `#0F0E0C` | `#F5F1E8` |
| `--panel` | Hover rows, cards, dialog surface | `#16140F` | `#ECE6D9` |
| `--raise` | Selected row, active segment, strips | `#1D1A14` | `#E3DCCB` |
| `--line` | Hairlines, unlit trace, borders, outline numerals | `#2C2922` | `#D3CABA` |
| `--text` | Primary text | `#EFE8DA` | `#1C1813` |
| `--muted` | Secondary text, icon default | `#B3AB9A` | `#4F483D` |
| `--faint` | Timestamps, labels, placeholders | `#8C8474` | `#6E6556` |
| `--accent` | The signal: lit trace, selection, primary button, pins | `#FFB547` | `#985400` |
| `--ink` | Text on an accent fill | `#1A1206` | `#FFF9EF` |
| `--bad` | Destructive and error states | `#FF7A66` | `#B4321F` |

**Accent presets** (dark theme, swappable): amber `#FFB547` (default), mint `#5EE6C8`, pink `#FF7AA2`, periwinkle `#A9B8FF`. For light mode, darken any preset until it reaches 4.5:1 on `--bg`.

**Rules**
- Use exactly one accent. Never place two accent-coloured blocks next to each other. The accent marks *where you are* or *what will happen on Enter*.
- Use `--bad` only for delete actions, the one-way warning in the secret dialog, the error strip, and failed action icons.
- All text tokens pass WCAG AA on `--bg`, `--panel` and `--raise` in both themes (`--faint` is at least 4.6:1).
- Mixes such as accent at 9–14% for tints and the hatched bands are built with `color-mix(in srgb, var(--accent) N%, transparent)`.

## 2. Typography

| Family | Use |
|---|---|
| **Bricolage Grotesque** (variable, opsz 12–96, wght 400–800) | All content and UI text: clip text, headings, buttons |
| **Martian Mono** (300–500) | Readouts: times, type codes, app names, sizes, tags, key caps, eyebrows, counts |
| Fallbacks | `'Segoe UI Variable Display', sans-serif`, and `ui-monospace, Consolas, monospace` |

| Step | Size / line-height / weight / tracking | Font | Used for |
|---|---|---|---|
| Ghost numeral | 250 / 1 / 800 / −0.06em, outline only (1px `--line` stroke) | Bricolage | Clip position behind the preview ("03") |
| Display L | 46 / 1 / 800 / −0.04em, opsz 96 | Bricolage | Link host in preview |
| Display M | 40–44 / 1.02 / 600 / −0.035em | Bricolage | Empty-state headlines |
| Display S | 31 / 1.16 / 500 / −0.022em, opsz 72 | Bricolage | Text clip in preview |
| Heading | 25–26 / 1.1 / 600 / −0.03em | Bricolage | Dialogs, bulk summary |
| Wordmark | 19 / 1 / 800 / −0.03em | Bricolage | `clip`**`trace`** (second half in accent) |
| Search | 16 / — / 400 / −0.01em | Bricolage | Search input |
| Row | 14.5 / 1.25 / 500 / −0.005em | Bricolage | History row line |
| Body | 13.5–15 / 1.45–1.5 / 400–500 | Bricolage | Explanations, buttons |
| Mono content | 11.5 / 1.95 | Martian Mono | Code, URLs in rows |
| Readout | 10–11 / — / 400 | Martian Mono | Times, meta lines, stats values |
| Micro | 9–9.5 / — / 400, +0.04em for caps | Martian Mono | Row sub-line, stat keys, key caps |

Use lower-case mono for labels ("today · sun 08 nov", "pinned · 0"). Use upper case only for stat keys (COPIED, FROM, SIZE, USED).

## 3. Spacing

Base unit **2px**, used mostly in steps of 4.

| Token | px | Typical use |
|---|---|---|
| `space-1` | 2 | Gap between icon buttons and segment items |
| `space-2` | 4 | Key-cap gap, row inner margin |
| `space-3` | 6–8 | Icon + label gap |
| `space-4` | 10–12 | Row padding, chip gaps |
| `space-5` | 14–16 | Strip padding, header gaps |
| `space-6` | 18–22 | Preview section gap (tags, stats) |
| `space-7` | 24–26 | Preview vertical rhythm |
| `space-8` | 30–36 | Preview / empty-state outer padding |

**Fixed layout metrics**
- Full view: **880 × 580**. Header 58. History column 344. Preview fills the rest.
- Quick paste: **380 × 580**. Header 52. Footer 46.
- Trace grid (full view): `58px time | 26px spine | 1fr body`. Quick paste: `44px number | 22px spine | 1fr | auto`.
- Row height: 62 (full view), 46 (quick paste). Day label row: 34 / 30.
- Trace node centre: 21px (full) / 23px (quick) from the top of the row.

## 4. Radii

| Token | px | Use |
|---|---|---|
| `r-xs` | 1–2 | Key caps, hatch bars, pin diamonds, checkboxes |
| `r-sm` | 3 | Buttons, icon buttons, segmented items |
| `r-md` | 4 | Rows, segmented control, strips, dialog |
| `r-window` | 8 | App window |

Avoid pills and large radii. The look is machined, not soft.

## 5. Shadows and glow

| Token | Value | Use |
|---|---|---|
| `glow-trace` | `0 0 10px var(--accent)` | Lit trace segment |
| `glow-dot` | `0 0 6px var(--accent)` | Active segment dot |
| `glow-button` | `0 0 24px color-mix(accent 30%)` | Primary (Paste) button |
| `ping` | keyframes 0 → `0 0 0 9px transparent`, 1.6s, infinite | Selected node pulse |
| `float` | `0 18px 44px rgba(0,0,0,.6)` | Undo / error strip |
| `pop` | `0 24px 50px rgba(0,0,0,.6)` | Tag suggestion list |
| `modal` | `0 30px 80px rgba(0,0,0,.65)` | Dialog / sheet |
| `slip` | `0 14px 30px rgba(0,0,0,.45)` | Fanned multi-select cards |
| `scrim` | `rgba(8,7,5,.76)` + `backdrop-filter: blur(3px)` (light: `rgba(40,32,20,.42)`) | Behind dialogs |

## 6. Textures

- **Dot grid**: `radial-gradient(circle, var(--line) 1px, transparent 1.3px)` at 20px, masked with `linear-gradient(200deg, #000, transparent 70%)`. Preview and empty-state panels only.
- **Hatch**: `repeating-linear-gradient(135deg, C 0 2px, transparent 2px 5px)`. Used for redacted secrets (C = `--muted`), placeholder bars (`--line`), the paused band and selection bar (accent at 6–10%, 6/12px steps), and the encrypt sweep (accent mix).
- **Crop marks**: four 12–14px L-corners in accent, offset −5 to −6px. Frame images and the "try it" sample.

## 7. Iconography

- **Style**: inline SVG, 24×24 viewBox, `stroke="currentColor"`, **stroke-width 1.6** (1.8–2.2 at ≤14px), round caps and joins, no fills (except the pause bars and the checkbox tick on accent).
- **Sizes**: 16 (actions), 17 (collapse/expand), 13–15 (close, inline).
- **Colour**: `--muted` by default, `--text` on hover, `--accent` when on (pinned, secret), `--bad` for a failed action or delete hover.
- **Set**: copy (two rects), pin (pushpin), tag (tag + hole), lock (shackle + body), trash, close ×, collapse (arrows in), expand (arrows out), warning triangle, check, pause (two bars), external ↗ (text glyph).
- **Type codes instead of icons** in the trace: `TXT URL KEY SRC IMG` in Martian Mono.
- No emoji, no filled icon tiles.

## 8. Components

### Trace spine
- 1px `--line` vertical rule through the spine column.
- **Lit segment**: 2px accent with `glow-trace`. Every row above the selection is fully lit, and the selected row is lit to its node. Height transition 180ms linear, staggered 22ms per row.
- **Nodes**: 7px circle, `--bg` fill, 1.5px `--faint` border. *Past* (above the selection): accent border. *Selected*: 11px accent fill + `ping`. *Pinned*: diamond (rotated square, 1px radius). *Checkbox* (multi-select): 13px square, radius 2. When checked it gets an accent fill, an ink tick and a soft glow. *Pending delete*: dashed `--bad` border, hollow. *Waiting* (empty): 11px hollow accent ring with a slow breathe animation.
- **Dashed spine** (`3px on / 6px off`, `--faint` or `--line`): future or unknown time, paused capture, empty pins.
- **Cut** (amber ×): a search that ends with no hits.

### History row (full view)
- 62px grid row: time (mono 10px, accent when selected) | spine | body.
- Body: line (14.5/500, mono for links and code) + sub-line (`CODE · app  #tag  ◆ pinned`, mono 9.5).
- States: hover `--panel` · selected `--raise` · checked accent 9% · pending delete 50% opacity with a `--bad` strikethrough and "deleted".
- Secrets are never rendered as text. They show a 150×12 hatch bar.

### Quick-paste row
- 46px. The number 1–9 (mono 13/500) replaces the time column. Time sits on the right (or ◆ for pinned).

### Header (full view)
- Wordmark → hairline divider → search input (no box, 16px) → **segmented toggle** → collapse icon button.
- Segmented toggle: 30px tall, 1px border, radius 4, 2px inner padding. Items are 24px mono 10.5 with a 5px marker (dot for *all*, diamond for *pinned*) and a count. The active item gets `--raise` and a glowing accent marker.

### Preview pane
- Meta line (mono 10: `CODE app / window`) with the action cluster on the right: Copy · Pin · Tag · Secret | Delete. Each is a 32px icon button.
- Content by type: text → Display S · link → host Display L + full URL mono + "open in browser ↗" · code → mono with accent line numbers · image → 16:9 frame with crop marks · secret → hatch bars + "hold to reveal".
- Below the content: **Paste** primary button, divider, tags (`#tag ×`) and `+ tag`.
- Stats strip pinned to the bottom: COPIED | FROM | SIZE | USED, mono, separated by 1px rules.
- Ghost numeral (clip position) sits behind everything, top-right, cropped.

### Buttons
| Variant | Spec |
|---|---|
| Primary (`go`) | 36–38px, radius 3, accent fill, ink text 14/600, key cap inside, `glow-button` |
| Ghost (`gh`) | 34–38px, transparent (or a 1px `--line` border on empty states), `--muted` → `--text` on hover with a `--raise` fill |
| Destructive (`badbtn`) | 34px, 1px `--bad` border, `--bad` at 14% fill, `--bad` text 14/600 |
| Icon (`ico`) | 32×32, radius 3, states as in Iconography |
| Text link (`txt`) | Mono 10–10.5, accent, underline offset 4 |
| Resume | 28–30px, accent fill, ink text 12.5–13/600 |

### Key cap
18–22px tall, min-width 18–22, mono 9–9.5, 1px `--line` border (2px bottom on the larger caps), radius 2–3. Inside the primary button the border is ink at 35%.

### Tag suggestion list
- Anchored under the search field: 2px accent top border, `--panel` surface, `pop` shadow.
- Option row 36px: `#` + **typed prefix in accent** + remainder (mono 12) | usage meter (3px bar, accent fill) | count.
- First option is pre-selected (`--raise`). An optional "+ new tag #x" row. Footer key hints: ↑↓ choose · Tab complete · Esc close.

### Selection bar (multi-select)
- Replaces the header (full view) or footer (quick paste). Accent hairline and a faint accent hatch.
- Contents: close × · count (22/800 accent) · "selected of N" · select all / none · Pin · Tag · **Delete N** (destructive).
- Full-view preview: up to four selected clips fanned as cards (`slip`), rotated −5° to +6°, with a ghost count numeral.

### Strip (undo and error)
- Floating, 52px tall, 540px wide (full view) or full width minus 20px (quick paste). 16px from the bottom (quick paste: sits above the footer). `--raise`, radius 4, `float`.
- **Undo**: trash icon · label · countdown `5s` (mono accent) · **Undo ^Z** (accent text button) · ×. A 2px accent bar along the top edge drains from 100% to 0 over 5s (1s linear steps). The clip is removed when it reaches 0.
- **Error**: `--bad` tint (8%) and border (55%), with a full-width 2px `--bad` top bar. Warning icon · title (14/600) + detail (mono 9.5: cause · code) · **Retry ↵** · ×. The icon of the action that failed turns `--bad`.

### Dialog (mark secret)
- Full view: a centred 460px dialog. Quick paste: a bottom sheet. Both have a 2px accent top border, sit on `--panel`, and use the `modal` shadow over a `scrim`.
- Order: mono eyebrow (`ctrl l · mark secret`) → heading → specimen (the clip text with an animated accent hatch sweeping across it, 3s) → three ✓ facts → **one-way warning** box (`--bad` 7% fill, 40% border) → Cancel (Esc) + **Encrypt & lock** (↵).

### Paused band
- At the top of the history list (full view) or under the header (quick paste). Accent 45% border with an accent hatch. Pause bars · "Capture paused" + mono "since 04:31 · copies aren't saved" · **Resume**.
- Header extras: "trace" in the wordmark dims to `--faint`, and an accent "❚❚ paused" tag appears. The top of the spine turns dashed.

### Empty states
- They use the same frame and header, so the spine always stays visible. The visual tells the story: a waiting node, empty diamond slots, or a cut trace.
- Headline (Display M) → explanation → ruled list of steps or tips with key caps → one action.

## 9. Motion

| Moment | Spec |
|---|---|
| Trace lights to a new selection | height 180ms linear, 22ms stagger per row |
| Selected node | `ping` 1.6s ease-out, infinite |
| Waiting node (empty) | breathe 2.4s ease-in-out |
| Encrypt specimen | hatch sweep, clip-path, 3s cubic-bezier(.6,0,.2,1), infinite |
| Undo countdown | bar width 1s linear per tick |
| Pin diamonds (empty pins) | 60s linear rotation |

All animation is off under `prefers-reduced-motion: reduce`. Never use entrance animations that start from `opacity: 0`: content must stay visible if an animation is paused or frozen.

## 10. Accessibility

- Every control is a real `<button>`, `<a href>` or `<input>` with a `<label>`. Icon-only buttons carry `aria-label` and a `title` showing the shortcut.
- Selected row: `aria-current="true"`. Multi-select rows: `role="checkbox"` + `aria-checked`. Toggles: `aria-pressed`.
- Tag list: `role="listbox"` / `option` + `aria-selected`. Undo strip: `role="status"` with `aria-live="polite"`. Error strip: `role="alert"`. Dialog: `role="dialog" aria-modal="true" aria-labelledby`.
- Focus ring: 1.5px accent outline, inset.
- Secrets are hidden from screen readers too, with `aria-label="Hidden secret"`.
