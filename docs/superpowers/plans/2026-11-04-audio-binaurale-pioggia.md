# Audio Binaurale and Rain Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the binaural tone clearer by removing the masking brown-noise layer while preserving its correct 200/240 Hz carriers (40 Hz difference), and smooth MP3 rain loop boundaries without buffering the whole recording.

**Architecture:** Preserve the existing `rodio::Source` interfaces, per-channel phase progression, and independent sinks. Remove the unrelated brown-noise overlay from the binaural source so the interaural tone difference is not masked, then improve the streaming file loop with a bounded overlap buffer and tests that exercise boundaries without requiring a user-provided recording. Keep effect claims conservative: this changes listening quality, not proven cognitive efficacy.

**Tech Stack:** Rust stable, rodio 0.19, inline Rust unit tests.

## Global Constraints

- Keep the audio generators procedural and local; do not add network access or new dependencies.
- Preserve stereo binaural separation and unlimited playback.
- Never buffer the entire MP3; loop-boundary buffering must remain bounded to a short crossfade window.
- Keep user audio data and existing `pioggia.mp3` path unchanged.

---

### Task 1: Correct binaural carrier frequencies

**Files:**
- Modify: `src/suono.rs`
- Test: `src/suono.rs` inline `#[cfg(test)]` module

**Interfaces:**
- Consumes: existing `Binaurale40` iterator and constants `FREQUENZA`, `TONO_SINISTRO`, `TONO_DESTRO`.
- Produces: clean interleaved 200/240 Hz stereo carriers whose 40 Hz difference is not masked by a noise layer.

- [x] **Step 1: Write the failing signal test**
  Update the binaural tests to inspect one second of interleaved samples and assert per-ear carrier rates of 200 Hz and 240 Hz, a 40 Hz difference, and exact agreement with the pure sine values (no added stochastic noise).
- [x] **Step 2: Run the focused test and confirm it fails**
  Run: `cargo test suono::test -- --nocapture`
  Expected: FAIL because the binaural source currently adds brown noise to both channels.
- [x] **Step 3: Remove only the masking noise layer**
  Keep phase increments based on `FREQUENZA` (Rodio sample rate is per channel), preserve tone constants, amplitude, stereo alternation, and source metadata, and return only each channel’s sine carrier.
- [x] **Step 4: Run binaural tests**
  Run: `cargo test suono::test -- --nocapture`
  Expected: all tests pass, including assertions for per-ear carriers and the 40 Hz difference.

### Task 2: Smooth MP3 rain loop boundaries

**Files:**
- Modify: `src/suono_file.rs`
- Modify: `src/ui_audio.rs`
- Test: `src/suono_file.rs` inline `#[cfg(test)]` module

**Interfaces:**
- Consumes: `SuonoFile::apri`, `Iterator<Item = i16>`, `Source`, the existing application-data `pioggia.mp3` path.
- Produces: continuous playback with a short, bounded crossfade at each loop and UI text that names the actual MP3 location.

- [x] **Step 1: Add a deterministic loop-boundary regression test**
  Extract or introduce a small internal loop/crossfade helper that can be tested with synthetic interleaved samples. Assert the emitted boundary transitions remain bounded relative to the abrupt-loop baseline, preserve sample count/order outside the overlap, and do not allocate proportional to the recording duration.
- [x] **Step 2: Run the focused regression test and confirm it fails**
  Run: `cargo test suono_file::test::il_loop_sfuma_senza_salti_bruschi -- --exact --nocapture`
  Expected: FAIL against the current immediate decoder reopen.
- [x] **Step 3: Implement bounded overlap at EOF**
  Use a short fixed-duration tail/head buffer, aligned to complete channel frames. Crossfade the saved tail into the beginning of the reopened decoder, then resume streaming the remainder. Handle recordings shorter than the overlap and decoder reopen failure without panicking or looping forever.
- [x] **Step 4: Correct rain-file UI copy**
  Update the rain-mode help text in `src/ui_audio.rs` to say that `pioggia.mp3` is read from the application's data folder, matching `src/audio.rs`.
- [x] **Step 5: Run focused tests and formatting**
  Run: `cargo test suono_file::test -- --nocapture`
  Run: `rustfmt --edition 2021 --check src/suono.rs src/suono_file.rs src/ui_audio.rs`
  Expected: focused file-source tests pass and the changed Rust files are formatted. Repository-wide `cargo fmt --check` has unrelated pre-existing differences.

### Task 3: Verify the integrated audio behavior

**Files:**
- Test: existing source-module unit tests

- [x] **Step 1: Run the complete test suite**
  Run: `cargo test`
  Expected: all non-ignored tests pass.
- [x] **Step 2: Inspect final changes**
  Run: `git diff --check` and `git status --short`
  Expected: only the binaural, MP3 loop, UI-copy, and plan files are changed; no generated audio artifacts are added.
