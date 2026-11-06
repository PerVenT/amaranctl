# Design: Automated Demo Recording & Split-Screen GIF Generation

**Date:** 2026-11-06  
**Status:** Proposed  
**Scope:** Developer Tooling / Documentation Asset  

---

## 1. Intent & Goal

Showcase `amaranctl` in action inside `README.md` with an engaging, informative visual demonstration.  
The demo must prove:
1. **Sub-50ms local IPC execution speed** of CLI commands.
2. **Bi-directional status polling** (`amaranctl status`).
3. **Adaptive system tray icon** live color rendering (warm gold to cool white daylight).
4. **Global hotkey interaction** (`Ctrl + Alt + L`, `Ctrl + Alt + Up/Down`, `Ctrl + Alt + Left/Right`).

---

## 2. Approaches Considered

1. **Approach A: Terminal-only recording (asciinema)**
   - *Pros:* Easy to capture, text-based.
   - *Cons:* Does not demonstrate the unique graphical tray features or hotkeys; only shows command text.
2. **Approach B: Two separate recordings stitched together**
   - *Pros:* Independent framing.
   - *Cons:* BLE communication jitter and timing variances make frame-accurate synchronization between terminal and tray blinks nearly impossible without tedious manual editing.
3. **Approach C: Single-pass synchronized recording with scripted walkthrough & ffmpeg split-screen crop (Chosen)**
   - *Pros:* 100% frame-perfect synchronization. One script runs the whole show with simulated typing and hotkey commands. An automated ffmpeg script extracts the terminal window and tray region, scales up the tray for readability, and generates an optimized `demo.gif` and `demo.mp4`.

---

## 3. Architecture & Components

### Component 1: `scripts/demo-walkthrough.ps1`
An automated PowerShell walkthrough runner:
* Prints a sleek terminal banner.
* Implements a typewriter effect for commands (`amaranctl status`, `amaranctl on`, `amaranctl cct 5600`, etc.).
* Executes the actual IPC commands so the live lamp and tray icon react in real-time.
* Visualizes hotkey actions on-screen (e.g. `[Hotkey] Ctrl + Alt + Down (-10% Brightness)`).
* Includes configurable delays (1.2–2s) for visual pacing.

### Component 2: `scripts/render-demo.ps1`
An ffmpeg automation script (can also run via `make demo`):
* Accepts input recording (e.g. `raw_demo.mp4` captured via Windows Snipping Tool, Game Bar, or OBS).
* Automatically detects or uses parameters to crop:
  - **Left Half:** Terminal window content.
  - **Right Half:** Taskbar tray notification area (scaled up 1.5x–2x so the adaptive tray icon is distinctly visible).
* Uses ffmpeg `hstack` filter with high-quality palette generation (`palettegen` / `paletteuse`).
* Outputs:
  - `docs/assets/demo.gif` (optimized <2 MB for direct inline GitHub rendering).
  - `docs/assets/demo.mp4` (crisp H.264 video).

### Component 3: `README.md` Integration
* Embeds `docs/assets/demo.gif` right beneath the main badge bar.
* Preserves fast loading and clean layout.
