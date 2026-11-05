# Core Study Flow UX Implementation Plan

> **For agentic workers:** Use the executing-plans workflow task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the path from setting up a study session through focused work to session completion clear and low-friction, and give the user an easy way to choose and preview a small, licensed set of background sounds.

**Architecture:** Keep the existing egui views and session/state model. Restructure setup around required session inputs, keep the primary start action visible with a clear disabled reason, reduce in-session visual competition without deleting features, and show a completion summary from the actual session result. Add two small CC0 rain recordings and a procedurally generated 4 Hz binaural option to the direct sound selector; keep 40 Hz available and describe options neutrally.

**Tech Stack:** Rust stable, eframe/egui, existing inline Rust UI/session tests.

## Global Constraints

- Preserve existing persisted configuration and study data formats.
- Preserve current session, Zotero, TTS, XP, review, emergency-stop, and accessibility behavior.
- Keep the primary setup-to-study route visible at the default 620×860 window size and validate narrow and large windows.
- Bundle only verified CC0 audio; record each source URL, author, license, duration, and format in `assets/audio/README.md`.
- Do not add more than 10 MB of new audio assets in this slice; never package Pixabay audio under its current standalone-redistribution restriction.
- Do not make medical or cognitive-effect guarantees about binaural audio.

---

### Task 1: Simplify session setup and keep Start discoverable

**Files:**
- Modify: `src/ui_setup.rs`
- Modify: `src/ui_zotero.rs` only if required to separate essential exam/material selection from secondary exam metadata
- Test: `src/test_interfaccia.rs`

**Interfaces:**
- Consumes: existing `StatoApp`, `MotoreAudio`, and setup actions.
- Produces: an uncluttered primary setup path with the session-start control visible and a clear explanation when required inputs are missing.

- [ ] **Step 1: Add failing UI coverage**
  Extend existing egui tests to cover the default viewport, a narrow viewport, missing-input explanation, and the presence of the primary Start action without requiring scroll to the bottom.
- [ ] **Step 2: Run the focused UI test and confirm it fails**
  Run: `cargo test test_interfaccia -- --nocapture`
  Expected: the regression assertion for the visible primary action or status explanation fails on the current long setup page.
- [ ] **Step 3: Restructure setup around the next study session**
  Keep goal, Zotero exam/material, duration, and sound as primary inputs. Move files, review, history, and less-used configuration into compact secondary sections. Keep Start in a persistent bottom action area and show why it is disabled.
- [ ] **Step 4: Remove misleading setup tagline**
  Replace the hard-coded “five exams / December” tagline with context-neutral text or current session state.
- [ ] **Step 5: Run focused setup tests**
  Run: `cargo test test_interfaccia -- --nocapture`
  Expected: UI tests pass at default, narrow, and large dimensions.

### Task 2: Make the focus overlay easier to scan

**Files:**
- Modify: `src/ui_focus.rs`
- Modify: `src/ui_azioni.rs` only if a focused action needs to move into a secondary group
- Test: `src/test_interfaccia.rs`

**Interfaces:**
- Consumes: current session state and existing action handlers.
- Produces: timer, pause/stop, and immediate study actions remain easy to find; secondary tools stay available without competing with the current task.

- [ ] **Step 1: Add a failing layout test for the primary focus controls**
  Assert that the timer and pause control remain visible at 620×860 and 360×640, and that secondary tools remain reachable through an explicit collapsed group.
- [ ] **Step 2: Run the focused test and confirm the expected failure**
  Run: `cargo test test_interfaccia -- --nocapture`
- [ ] **Step 3: Group the focus overlay by priority**
  Preserve timer and pause at the top, keep the current study task and SOS action prominent, and group less-used file/AI/NotebookLM utilities under a clearly named secondary section. Keep emergency stop available and visibly distinct.
- [ ] **Step 4: Run focused overlay tests**
  Run: `cargo test test_interfaccia -- --nocapture`

### Task 3: Add an explicit session-completion summary

**Files:**
- Modify: `src/sessione.rs`
- Modify: `src/state.rs`
- Modify: `src/ui_setup.rs` or a focused completion view module, following the existing UI boundaries
- Test: `src/sessione.rs` and `src/test_interfaccia.rs`

**Interfaces:**
- Consumes: the real completed-session data already recorded by `Sessione`/`Historico` and XP state.
- Produces: a completion view showing actual duration, exam/material when available, and XP, with a clear next action back to setup; do not synthesize or seed data.

- [ ] **Step 1: Write a failing session-summary state test**
  Add a test proving that a completed session exposes its actual final duration and XP exactly once, while an aborted session does not masquerade as a completed one.
- [ ] **Step 2: Run the focused state test and confirm failure**
  Run: `cargo test sessione -- --nocapture`
- [ ] **Step 3: Preserve completion information across the setup transition**
  Keep only the real just-finished session summary in application state; preserve existing history persistence and avoid changing serialized config/history schemas unless strictly necessary.
- [ ] **Step 4: Render the completion summary and next action**
  Show completion feedback before returning to the normal setup flow, with a single clear action to prepare the next session.
- [ ] **Step 5: Run focused completion tests**
  Run: `cargo test sessione -- --nocapture`
  Run: `cargo test test_interfaccia -- --nocapture`

### Task 4: Make background sound choices understandable and previewable

**Files:**
- Modify: `src/ui_audio.rs`
- Modify: `src/audio.rs`
- Modify: `src/suono.rs`
- Modify: `src/suono_file.rs` only if needed to stream included audio bytes through the existing bounded loop
- Modify: `src/config.rs` only if a selected sound must persist
- Create: `assets/audio/rain-city.mp3` (CC0 source: `https://freesound.org/people/Legnalegna55/sounds/545250/`)
- Create: `assets/audio/gentle-rain.mp3` (CC0 source: `https://freesound.org/people/Legnalegna55/sounds/551989/`)
- Create: `assets/audio/README.md` (asset provenance and license records)
- Test: `src/suono.rs`, `src/audio.rs`, and/or `src/test_interfaccia.rs`

**Interfaces:**
- Consumes: existing audio mode enum, local MP3 source, and persisted sound-mode setting.
- Produces: explicit sound choices with a preview/switch action; two CC0 rain MP3 options and 4 Hz/40 Hz binaural options; no claims of guaranteed focus effects.

- [ ] **Step 1: Add failing tests for new audio options**
  Test that the new audio modes round-trip through configuration and the 4 Hz source emits the requested left/right carrier difference. Test that each bundled MP3 can be opened and emits non-silent samples.
- [ ] **Step 2: Run the focused audio tests and confirm failure**
  Run: `cargo test audio -- --nocapture`
- [ ] **Step 3: Add only verified, small CC0 MP3 assets**
  Verify the asset page names CC0 and ensure the packaged copies are at most 10 MB total. If an authorized direct download is unavailable, stop this asset substep rather than bypassing a login; retain an existing custom-file mode instead of substituting Pixabay material.
- [ ] **Step 4: Add the 4 Hz binaural option without hardcoding health claims**
  Reuse the existing stereo phase generator with a configurable beat difference. Preserve the current 40 Hz mode and headphone note.
- [ ] **Step 5: Replace cyclic audio controls with direct sound choices**
  Make the current choice obvious and add a preview action that switches playback immediately; show a short neutral description and the headphone requirement for binaural options.
- [ ] **Step 6: Run focused sound tests and format changed files**
  Run: `cargo test audio -- --nocapture`
  Run: `rustfmt --edition 2021 --check` on every changed Rust file.

### Task 5: Verify the core study journey

**Files:**
- Test: existing unit and egui UI tests

- [ ] **Step 1: Run full tests and formatting**
  Run: `cargo test`
  Run: `rustfmt --edition 2021 --check` on every changed Rust file.
  Expected: all non-ignored tests pass and changed files are formatted.
- [ ] **Step 2: Validate the rendered application**
  Build and open FocusAnchor. Inspect setup, focus overlay, sound selector, and completion summary at 620×860, a narrow window, and a large window. Confirm the primary action stays visible and secondary actions remain reachable.
- [ ] **Step 3: Inspect the final diff**
  Run: `git diff --check` and `git status --short`.
  Expected: only files required for the approved core flow are changed; preserve pre-existing uncommitted audio changes and generated review artifacts.

### Follow-up: Unify Zotero and NotebookLM in the selected-exam material flow

- [x] Add an egui regression test confirming the selected-exam flow exposes Zotero refresh and NotebookLM package actions together.
- [x] Consolidate selected exam, chosen texts, Zotero refresh feedback, per-exam NotebookLM link, Markdown package generation, preview, and manual upload guidance in that flow.
- [x] Keep NotebookLM transfer explicit and user-driven; do not claim direct API upload or alter persisted configuration formats.
- [x] Verify existing archive action remains available and both old and new flows share one package-generation handler.
- [x] Run focused integration tests, full `cargo test`, and `git diff --check`.

Completion note: the selected-exam materials follow-up above is implemented and covered by `test_interfaccia::test_materiali::il_flusso_materiali_unisce_azioni_zotero_e_notebooklm`; full test suite and `git diff --check` pass.
