# Demo Recording & Split-Screen GIF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide an automated walkthrough script and ffmpeg split-screen processing pipeline to generate an engaging, synchronized demo GIF showing terminal commands and live system tray reactions side-by-side in `README.md`.

**Architecture:** A PowerShell script (`scripts/demo-walkthrough.ps1`) runs a choreographed demo with typing effects and on-screen hotkey banners while exercising the live `amaranctl` IPC daemon. A post-processing script (`scripts/render-demo.ps1`) uses ffmpeg to crop the terminal and zoomed tray regions, stack them horizontally (`hstack`), and export a lightweight, palette-optimized `demo.gif` and `demo.mp4`.

**Tech Stack:** PowerShell 7+ (pwsh), FFmpeg (via WSL Debian or host Windows), Markdown.

**Spec:** [docs/superpowers/specs/2026-11-06-demo-recording-design.md](file:///D:/Projects/apps/amaranctl/docs/superpowers/specs/2026-11-06-demo-recording-design.md)

## Global Constraints

- Standalone executable path: `~/.local/bin/amaranctl.exe`.
- Output demo gif size target: < 2 MB for fast GitHub README loading.
- High-contrast visual readability: tray region scaled 2x so the adaptive icon is clearly visible.
- Commits must use `--no-gpg-sign`.

## Review Focus

1. Execution without active daemon: `demo-walkthrough.ps1` should check if `amaranctl run` is active and warn or launch it cleanly if not.
2. FFmpeg availability: `render-demo.ps1` should check host Windows and WSL Debian for ffmpeg, with clear instructions if missing.
3. Aspect ratio / resolution: Ensure cropped terminal and tray regions have even dimensions for H.264 / GIF encoding.
4. Git whitelist: Ensure `docs/assets/**` is whitelisted in `.gitignore` so the demo assets can be committed.
5. Timing: Walkthrough pauses must be long enough for viewer comprehension (1.2–2.0s) without making the loop drag (>15s total).

---

### Task 1: Whitelist Assets Directory & Create Walkthrough Runner

**Files:**
- Modify: `.gitignore`
- Create: `scripts/demo-walkthrough.ps1`

**Interfaces:**
- Produces: `scripts/demo-walkthrough.ps1` accepting `-PaceSeconds` (default 1.5) and `-DryRun` switch.

- [ ] **Step 1: Ensure `docs/assets/**` is allowed in `.gitignore`**

Verify `.gitignore` already un-ignores `!docs/**`. Ensure `docs/assets` is created.

- [ ] **Step 2: Create `scripts/demo-walkthrough.ps1`**

Implement automated script that:
- Prints a clean, styled terminal header.
- Defines a typing simulator function `Write-SimulatedCommand`.
- Executes the choreography:
  1. `amaranctl status`
  2. `amaranctl on`
  3. `amaranctl brightness 50`
  4. `amaranctl cct 5600`
  5. Simulates hotkey banner `[Hotkey] Ctrl + Alt + Down (-10% Brightness)` & calls `amaranctl brightness -- -10`
  6. Simulates hotkey banner `[Hotkey] Ctrl + Alt + Left (-200K Warmer)` & calls `amaranctl cct -- -200`
  7. `amaranctl status`
  8. `amaranctl off`
- Pauses between steps for visual clarity.

- [ ] **Step 3: Test walkthrough runner with `-DryRun`**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/demo-walkthrough.ps1 -DryRun`
Expected: Clean terminal walkthrough execution without errors.

- [ ] **Step 4: Commit**

```bash
git add .gitignore scripts/demo-walkthrough.ps1
git commit --no-gpg-sign -m "feat(demo): add automated demo walkthrough runner script"
```

---

### Task 2: Create FFmpeg Split-Screen Processor & Makefile Target

**Files:**
- Create: `scripts/render-demo.ps1`
- Modify: `Makefile`

**Interfaces:**
- Consumes: Raw screen recording (e.g. `raw_demo.mp4` or a sample synthetic generator).
- Produces: `docs/assets/demo.gif` and `docs/assets/demo.mp4`.

- [ ] **Step 1: Create `scripts/render-demo.ps1`**

Implement ffmpeg script that:
- Detects ffmpeg in Windows PATH or inside WSL Debian (`wsl -d Debian ffmpeg`).
- Accepts `-InputFile <path>` (default: `raw_demo.mp4`).
- Defines crop rectangles for Terminal (left half) and Tray area (right half).
- Applies `scale=2*iw:2*ih:flags=lanczos` on the tray area for crispness.
- Combines using `[0:v]...[left]; [0:v]...[right]; [left][right]hstack`.
- Generates 2-pass palette (`palettegen` / `paletteuse`) to produce `docs/assets/demo.gif` (<2 MB).
- Also outputs `docs/assets/demo.mp4`.
- If no input video file exists, generates an animated placeholder demo showcasing the side-by-side terminal & tray icon states so `README.md` is immediately functional.

- [ ] **Step 2: Add `make demo` target to `Makefile`**

Add `demo` target to `Makefile` that runs `pwsh -File scripts/render-demo.ps1`.

- [ ] **Step 3: Verify generation of demo assets**

Run: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/render-demo.ps1`
Expected: `docs/assets/demo.gif` and `docs/assets/demo.mp4` generated successfully.

- [ ] **Step 4: Commit**

```bash
git add Makefile scripts/render-demo.ps1 docs/assets/
git commit --no-gpg-sign -m "feat(demo): add ffmpeg split-screen rendering script and Makefile target"
```

---

### Task 3: Embed Demo in `README.md` and Document Usage

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: `docs/assets/demo.gif`.

- [ ] **Step 1: Update `README.md`**

Embed `![amaranctl demo](docs/assets/demo.gif)` directly beneath the hero title and badges.
Add a short note explaining what is seen:
"Left: Sub-50ms CLI execution. Right: Live adaptive system tray icon & hotkeys reaction."

- [ ] **Step 2: Verify `make fmt-check`, `make test`, and documentation links**

Run: `wsl -d Debian bash -c "cd /mnt/d/Projects/apps/amaranctl && make test"`
Expected: All 28 tests pass.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit --no-gpg-sign -m "docs: embed live split-screen demo into README.md"
```
