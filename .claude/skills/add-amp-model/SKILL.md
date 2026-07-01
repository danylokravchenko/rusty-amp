---
name: add-amp-model
description: Add a new built-in amplifier model to rusty-amp. Use when asked to add, create, or contribute a new amp model/head (e.g. a Vox, Fender, Soldano, Peavey sim) to the amp bank. Covers the DSP code, the AmpModel enum, UI wiring, presets, tests, AND the docs site so nothing drifts.
---

# Add an amp model to rusty-amp

An amp model is a struct implementing the `Amplifier` trait, addressed through the fixed `AmpModel` enum (`Marshall`, `Mesa`, `Randall`, …) and switched live in the TUI. Adding one is **two coordinated bodies of work**: the Rust implementation (code + tests) and the documentation site. Both land in the **same PR** — a model that isn't documented is treated as incomplete.

`CONTRIBUTING.md`'s *"Adding a new amp model"* section is the short pointer for this; this skill is the full operational checklist (the `CONTRIBUTING.md` blurb says a `[C]` cycle key — that's stale copy-paste from the cabinet section, the real key is **`A`**, see step 4 below).

## Before you start

- Decide whether the model is **tube** (passive FMV-style tone stack, power-amp sag, output-transformer saturation, cathode bias, bright cap — like `Marshall`/`Mesa`) or **solid-state** (active shelf/peak tone stack, stiffer power section, no output transformer — like `Randall`). This determines which shared building blocks in `src/dsp/amp/mod.rs` you wire up.
- Reuse the shared helpers in `src/dsp/amp/mod.rs` rather than reimplementing them: `FrontEnd` (DC block + input HP), `Bloom` (preamp dynamics), `CathodeBias` (tube blocking-distortion/touch), `BrightCap` (gain-pot treble bleed, tube only), `OutputTransformer` (LF core saturation + crossover, tube only), `SpeakerLoad` (dynamic speaker-impedance bloom), `VoiceBalance` (post-tone-stack body/tilt), `Cached`/`ToneCache` (dirty-check coefficient recompute), `Oversampler8` (8× oversampling — mandatory for any nonlinear stage, see the aliasing test in step 9). For the tone stack itself, either call `crate::dsp::tonestack::ToneStack` with an existing or new `Components` preset (passive network) or build an active shelf/peak stack directly with `Biquad` (see `Randall`).
- Study `src/dsp/amp/marshall.rs` (tube template) or `src/dsp/amp/randall.rs` (solid-state template) line by line before writing the new one — the doc comments on each explain *why* each stage exists.
- Loudness-match: the new model's output trim must land within the loudness window the other models already sit in (see the `amps_are_loudness_matched` test in step 9) so switching models mid-set doesn't jump the volume.

## Part A — the code

Signal-path code must be **allocation-free and panic-free** at runtime; process samples as `f32`.

1. **`src/dsp/amp/<model>.rs`** — a `struct` implementing `Amplifier::process(sample, gain, bass, mid, treble, presence, master) -> f32`. Mirror the stage order of your chosen template (front end → oversampled nonlinear stages → tone stack → voice balance → power amp/sag → output transformer/speaker load → presence → output trim). Document the "why" for any stage whose corner frequency or depth isn't self-evident, same style as the existing models.
2. **`src/dsp/amp/mod.rs`**:
   - `pub mod <model>;` and `pub use <model>::<Model>;` at the top.
   - Add a field to `AmpBank` and construct it in `AmpBank::new`.
   - Add a match arm in `AmpBank::process`.
3. **`src/dsp/mod.rs`**:
   - Add a variant to the `AmpModel` enum (next `#[repr(u8)]` value).
   - `AmpModel::from_u8` — add the new discriminant.
   - `AmpModel::name()` and `AmpModel::short_name()` — full/short display strings.
   - `AmpModel::next()` / `AmpModel::prev()` — splice the new variant into the cycle ring (every variant must appear exactly once in each direction).
4. **`src/ui/draw.rs`** — add the variant to the amp-selector's literal array (`for m in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall]`, in the amp header row) so it renders as a selectable tile. The hint text there (`"↑/↓  A"`) confirms `A` is the live cycle key — don't add a redundant one.
5. **`src/ui/input.rs`** — no new match arm needed (cycling is generic over `AmpModel::next()`/`prev()`); just update the `cycle_amp_forward_visits_every_model_and_returns` / `cycle_amp_backward_is_the_inverse` tests to walk through the new model in the right position.
6. **`src/preset.rs`**:
   - `AmpSection.model` doc comment — extend the `"marshall" | "mesa" | "randall"` list.
   - `Preset::from_params` — add the new `AmpModel::<Model> => "<model>"` arm to the `amp_model_str` match.
   - `Preset::apply` — add `Some("<model>") => AmpModel::<Model>,` to the model-parsing match.
7. **Tests** — in `src/dsp/amp/mod.rs`'s `#[cfg(test)]` module:
   - Add the new model to `each_amp()` — this feeds it through every structural test (finite/bounded output, DC-free, harmonic-not-aliased, tight low end, tone/presence controls track, loudness-matched).
   - If it's a tube model, add it to `tube_amps()` too, so the cathode-bias/bright-cap/touch-sensitivity integration tests cover it. Solid-state models are deliberately excluded from `tube_amps()` (see the comment on `Randall`).
8. **`src/ui/draw.rs`** test module — `every_amp_model_renders_its_name` iterates a literal `[AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall]` array; add the new variant so the snapshot/assertion test picks it up.
9. **Snapshot** — your `draw.rs` edit (new tile in the selector) changes the golden **snapshot**. Re-render and re-bless it: run `INSTA_UPDATE=always cargo test --lib 'ui::'` (or `cargo insta review` to inspect the diff first), then **commit the updated `src/ui/snapshots/*.snap`** — CI compares against that committed file.

## Part B — the docs (required, same PR)

`site/amps-cabs.md` documents amps in a data-driven HTML block (`<div class="selector" data-tabs>`), not a new page.

1. **`site/amps-cabs.md`**, `## Amplifiers {#amp}` section:
   - Add a `<button class="tile" data-tab="<id>">` to the `.tiles` row: `.tile__name`, `.tile__sub` (one-line character blurb), `.tile__amp` with 6 decorative knob-icon spans (`--r:` rotation degrees — copy the pattern, vary the rotations).
   - Add the matching `<div class="tab-panel" data-panel="<id>">`: a `.specs` block with the 4 rows (`Gain range`, `Tone stack`, `Rectifier & power`, `Gain stages`) and a `.kv` row per knob (`Gain`, `Bass`, `Mid`, `Treble`, `Presence`, `Master`) — short technical sentences (frequencies, dB, component type), no marketing fluff, matching the terse style of the existing entries.
   - If the model changes the tube-vs-solid-state split described in the closing comparison paragraph, update that paragraph too.
2.  **`site/index.md`** - mention new model
3. **`site/how-it-works.md`** - new model in the chain
4. **`site/presets.md`** - new model in the template
5. **`CONTRIBUTING.md`** — while you're there, fix the stale `[C]` cycle-key reference in *"Adding a new amp model"* to `A` (see step 4 above) if it hasn't been fixed yet.
6. **`README.md`** — if it lists the built-in amp models, add the new one.

## Verify

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo nextest run                             # new amp tests + preset round-trip + UI tables
INSTA_UPDATE=always cargo test --lib 'ui::'   # re-bless the UI snapshot, then commit src/ui/snapshots/*.snap
cd site && npm run build                      # or: npm run dev  to preview
```

## Checklist

- [ ] `src/dsp/amp/<model>.rs` implementing `Amplifier`
- [ ] `src/dsp/amp/mod.rs`: module decl/re-export, `AmpBank` field + `new`/`process` arms
- [ ] `src/dsp/mod.rs`: `AmpModel` variant, `from_u8`, `name()`, `short_name()`, `next()`/`prev()`
- [ ] `src/ui/draw.rs`: selector literal array + `every_amp_model_renders_its_name` test array
- [ ] `src/ui/input.rs`: cycle tests updated for the new model
- [ ] `src/preset.rs`: doc comment, `from_params` match, `apply` match
- [ ] `each_amp()` (and `tube_amps()` if applicable) in `dsp/amp/mod.rs` tests
- [ ] UI snapshot re-blessed and committed (`src/ui/snapshots/*.snap`)
- [ ] docs: `site/amps-cabs.md` tile + panel, `site/index.md`, `site/how-it-works.md`, `site/presets.md`, `CONTRIBUTING.md`/`README.md` touch-ups
- [ ] all `cargo`/`npm` checks pass; manual audio pass done (load the model, sweep every knob)
- [ ] code + docs in the same PR
