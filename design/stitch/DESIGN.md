---
name: Cliptrace Ambient Utility
colors:
  surface: '#faf8ff'
  surface-dim: '#d2d9f4'
  surface-bright: '#faf8ff'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f2f3ff'
  surface-container: '#eaedff'
  surface-container-high: '#e2e7ff'
  surface-container-highest: '#dae2fd'
  on-surface: '#131b2e'
  on-surface-variant: '#3c4a42'
  inverse-surface: '#283044'
  inverse-on-surface: '#eef0ff'
  outline: '#6c7a71'
  outline-variant: '#bbcabf'
  surface-tint: '#006c49'
  primary: '#006c49'
  on-primary: '#ffffff'
  primary-container: '#10b981'
  on-primary-container: '#00422b'
  inverse-primary: '#4edea3'
  secondary: '#4d661c'
  on-secondary: '#ffffff'
  secondary-container: '#ceee93'
  on-secondary-container: '#536d22'
  tertiary: '#855300'
  on-tertiary: '#ffffff'
  tertiary-container: '#e29100'
  on-tertiary-container: '#523200'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#6ffbbe'
  primary-fixed-dim: '#4edea3'
  on-primary-fixed: '#002113'
  on-primary-fixed-variant: '#005236'
  secondary-fixed: '#ceee93'
  secondary-fixed-dim: '#b3d17a'
  on-secondary-fixed: '#131f00'
  on-secondary-fixed-variant: '#364e03'
  tertiary-fixed: '#ffddb8'
  tertiary-fixed-dim: '#ffb95f'
  on-tertiary-fixed: '#2a1700'
  on-tertiary-fixed-variant: '#653e00'
  background: '#faf8ff'
  on-background: '#131b2e'
  surface-variant: '#dae2fd'
typography:
  headline-lg:
    fontFamily: Space Grotesk
    fontSize: 1.75rem
    fontWeight: '600'
    lineHeight: 2.25rem
    letterSpacing: -0.03em
  headline-md:
    fontFamily: Space Grotesk
    fontSize: 1.375rem
    fontWeight: '600'
    lineHeight: 1.75rem
    letterSpacing: -0.025em
  headline-sm:
    fontFamily: Space Grotesk
    fontSize: 1.125rem
    fontWeight: '500'
    lineHeight: 1.5rem
    letterSpacing: -0.02em
  body-lg:
    fontFamily: Inter
    fontSize: 1rem
    fontWeight: '400'
    lineHeight: 1.5rem
    letterSpacing: -0.011em
  body-md:
    fontFamily: Inter
    fontSize: 0.875rem
    fontWeight: '400'
    lineHeight: 1.375rem
    letterSpacing: -0.006em
  body-sm:
    fontFamily: Inter
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1.125rem
    letterSpacing: 0em
  label-lg:
    fontFamily: Space Grotesk
    fontSize: 0.875rem
    fontWeight: '600'
    lineHeight: 1.25rem
    letterSpacing: 0.01em
  label-md:
    fontFamily: Space Grotesk
    fontSize: 0.75rem
    fontWeight: '600'
    lineHeight: 1rem
    letterSpacing: 0.02em
  label-sm:
    fontFamily: Space Grotesk
    fontSize: 0.6875rem
    fontWeight: '700'
    lineHeight: 0.875rem
    letterSpacing: 0.04em
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  gutter: 0.75rem
  margin: 1rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1.25rem
  space-xl: 1.75rem
---

## Brand & Style

This design system establishes an airy, hyper-focused utility presence tailored for a high-efficiency Windows floating search interface. The visual narrative combines the weightlessness of modern Scandinavian product design with the tactile precision of developer tooling: ultra-clean matte-acrylic surfaces, high-legibility geometric forms, and purposeful micro-interactions.

The aesthetic fuses **Modern Glassmorphic Acrylic** with **Calm Structural Minimalism**:
- **Atmospheric Float**: Windows Acrylic and Mica material logic translated into web surfaces via multi-pass sub-surface backdrops, diffuse ambient occlusion, and whispering 1px specular lightborders.
- **Trust & Confidentiality**: Clipboard management handles sensitive credentials, OTPs, and private tokens. The design language treats masking and encryption states with dedicated security visual cues (granular dot screens, tinted masking pills, biometric unlocked pulses) to establish immediate user trust.
- **Micro-Electric Energy**: A serene, near-achromatic palette of crisp slate and chalk white interrupted by targeted hits of high-voltage chartreuse/lime and warm security amber for pinning, masking, and keyboard shortcut indicators.

## Colors

The palette balances airy luminance with deliberate, crisp contrast. Rather than overwhelming the desktop, the window acts as a tranquil, semi-translucent visor.

### Palette Architecture
- **Primary Accent (`#10b981` / Active Emerald)**: Used for confirmed operations, active hotkey execution, and safe data signals.
- **Secondary Highlight (`#d9f99d` / Electric Chartreuse)**: The signature kinetic spark. Used for search query matches, keyboard focus indicators, tag highlights, and active selection pills. Always contrasted with deep ink slate text for WCAG AAA legibility.
- **Tertiary Utility (`#f59e0b` / Security Amber)**: Reserved strictly for elevated privacy actions—pinned clips, secret detection (passwords, tokens, API keys), and temporary unmask notifications.
- **Neutral Backbone (`#0f172a` / Deep Obsidian Slate)**: Defines high-contrast typography, crisp border hairpins, and grounded container headers.

### Surface Color Semantics
- **Window Base Surface**: `rgba(255, 255, 255, 0.78)` in light mode; `rgba(15, 23, 42, 0.76)` in dark mode.
- **Elevated Item Surface**: `rgba(255, 255, 255, 0.92)` hover, `rgba(241, 245, 249, 0.8)` rest.
- **Masking Fill**: `rgba(245, 158, 11, 0.12)` with amber-tinted alphanumeric blur (`filter: blur(4px)`).
- **Hairline Specular Border**: `rgba(255, 255, 255, 0.65)` top edge highlight, `rgba(15, 23, 42, 0.08)` outer boundary.

## Typography

The typography pairs the structured, slightly eccentric geometry of **Space Grotesk** with the neutral, hyper-legible rendering engine of **Inter**.

- **Display & Labels (`Space Grotesk`)**: Conveys technological utility. The geometric rhythm gives metadata pills, application sources (e.g., `VS Code`, `Chrome`), timestamp offsets, and shortcut badges a distinctly calibrated, command-bar personality.
- **Content & Text Streams (`Inter`)**: Carries all variable clipboard content (code snippets, rich text excerpts, URLs, color values). Inter’s tall x-height and optical clarity ensure zero glyph confusion when browsing code snippets or passwords.
- **Numbers & Metrics**: Uses tabular figures (`font-variant-numeric: tabular-nums`) everywhere clip metrics, index indicators, or memory byte sizes are presented.

## Layout & Spacing

Cliptrace functions as a non-intrusive desktop floating palette (default dimensions: `640px` wide, variable height capped at `520px`). The layout follows a compact vertical stack model:

1. **Omni-Input Bar**: Persistent top anchor containing search tokens, source app filters, and status pins.
2. **Dynamic Scroller Feed**: Single-column indexed virtual list with tight `0.5rem` (`space-sm`) gaps between list items for seamless keyboard arrow navigation.
3. **Inspector Footer**: Compact `0.75rem` (`space-md`) metadata strip indicating keyboard bindings (`↵ Copy`, `⌘⇧P Pin`, `⌥⌫ Delete`).

Compact margins and internal paddings prioritize maximum content density without sacrificing click or tap targets. Content adapts automatically when snapped or resized against Windows desktop boundaries.

## Elevation & Depth

Visual hierarchy uses a refined multi-layer system imitating Windows 11 Acrylic materials without native performance overhead:

- **Level 0 (Desktop Backplate)**: `backdrop-filter: blur(28px) saturate(180%)`. Surface uses `rgba(255, 255, 255, 0.75)` with an inner specular ring `box-shadow: inset 0 1px 1px 0 rgba(255, 255, 255, 0.8), 0 20px 48px -12px rgba(15, 23, 42, 0.16)`.
- **Level 1 (List Row - Idle)**: Neutral flat plane with `rgba(255, 255, 255, 0.45)`, enclosed by a faint sub-pixel boundary `border: 1px solid rgba(15, 23, 42, 0.05)`.
- **Level 2 (List Row - Focused/Selected)**: Surface shifts to solid white `rgba(255, 255, 255, 0.98)` with a distinctive left accent border (3px solid `#d9f99d`) and an ultra-soft drop shadow: `0 4px 14px -2px rgba(15, 23, 42, 0.08)`.
- **Level 3 (Flyout Preview & Context Menu)**: `backdrop-filter: blur(16px)`, `background: rgba(255, 255, 255, 0.96)`, elevated by `0 12px 32px -4px rgba(15, 23, 42, 0.18), 0 0 0 1px rgba(15, 23, 42, 0.08)`.

## Shapes

The design uses balanced, modern curves (Level 2: `0.5rem` / `8px` baseline). 

- **Outer Palette Shell**: `1rem` (`16px`) corner radius to match Windows 11 window curvature.
- **Clipboard Cards**: `0.5rem` (`8px`) corner radius providing comfortable separation between discrete items.
- **Kbd Badges & Shortcut Tokens**: `0.375rem` (`6px`) soft geometric squircle.
- **Privacy & Tag Chips**: Fully pill-shaped (`9999px`) to create an immediate visual distinction between actionable items and categorical tags.

## Components

### 1. Omni Search Input
- **Structure**: Zero-border full-bleed header field with an inset magnifying indicator, clear trigger, and dynamic scope chips.
- **Interaction**: Active typing displays a micro-chartreuse glow ring (`box-shadow: 0 0 0 2px rgba(217, 249, 157, 0.8)`).
- **Filter Tags**: Inline tags with app icons (`Figma`, `VS Code`, `Terminal`) featuring a 1-click dismissal cross.

### 2. Clipboard Stream Row (Standard)
- **Structure**: Left-aligned origin icon + timestamp, central content snippet (up to 2 lines with smart truncation), right-aligned format badge (e.g., `HEX`, `JSON`, `TEXT`, `PNG`).
- **Focus Style**: Moving down the list via arrow keys applies an animated chartreuse cursor bar on the left edge with instant smooth-scrolling lock.

### 3. Masked Secret & Token Row
- **Detection Cue**: Visual amber-gold shield badge (`#f59e0b`) placed next to the timestamp.
- **Obfuscation**: Text displays as heavy disc clusters (`••••••••••••`) accompanied by an inline SVG noise grain pattern.
- **Reveal Action**: Hover or `Spacebar` holds reveal with a smooth opacity unblur accompanied by a temporary timer pill: `Auto-hiding in 4s`.

### 4. Interactive Keyboard Badges (`<kbd>`)
- **Appearance**: Space Grotesk semi-bold uppercase text set in high-contrast slate, bordered with an inset 1px bottom bevel to simulate a physical mechanical keycap: `box-shadow: inset 0 -1px 0 rgba(15, 23, 42, 0.2)`.

### 5. Pin & Action Chips
- **States**:
  - *Unpinned*: Ghost outline with slate fill at 40% opacity.
  - *Pinned*: Warm amber backdrop (`rgba(245, 158, 11, 0.16)`), amber icon, and solid pinned indicator.
  - *Copied Toast Trigger*: Brief chartreuse flash (`#d9f99d`) traversing horizontally across the row item via a swift 200ms cubic-bezier ease.

### 6. Media & Asset Previews
- **Color Swatches**: Rendered as a duo-tone split chip displaying swatch color alongside HEX/RGB text.
- **Image Thumbnails**: Mini rounded previews (`32x32px`) with an aspect-ratio preserving cover and zoom flyout on hover.