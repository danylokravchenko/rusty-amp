---
name: add-pedal
description: Add a new pedal / DSP effect to rusty-amp end to end. Use when asked to add, create, or contribute a new pedal, stompbox, or rack effect (e.g. a chorus, phaser, flanger, octaver, EQ). Covers the DSP code, the UI wiring, presets, tests, AND the docs site so nothing drifts.
---

# Add a pedal to rusty-amp

A pedal is a self-contained DSP effect wired into the fixed signal chain, the TUI, and the preset format. Adding one is **two coordinated bodies of work**: the Rust implementation (code + tests) and the documentation site. Both land in the **same PR** — a pedal that isn't documented is treated as incomplete.

`CONTRIBUTING.md` is the source of truth; this skill operationalizes its *"Adding a new DSP effect"* and *"Documenting a new pedal"* sections. Read them alongside this.

## Before you start

- Decide **where in the chain** the pedal sits. The order is fixed:
  `Gate → Comp → Fuzz → TS-808 → DS-1 → Pre-EQ → Amp → Cab → Parametric EQ → Delay → Reverb`.
  Its position defines its character (pre-amp shapes what the gain stage clips; post-cab colours the final mix). Insert it at the right point in every ordered list below.
- Pick a **livery colour** and a short `<id>` (used for the CSS var, the `PEDAL_<NAME>` colour, and the docs deep-link).
- Reuse the shared building blocks in `src/dsp/effects/mod.rs` (`OnePoleLp`, `ThreeBandEq`, `db_to_lin`/`lin_to_db`, `param_changed`) and `Oversampler::process(x, f)` in `oversample.rs` rather than re-implementing tone LPs, EQ trios, dirty-checks, or oversample loops. Study an existing effect (`src/dsp/effects/tube_screamer.rs` or `distortion.rs`) as a template.

## Part A — the code

Signal-path code must be **allocation-free and panic-free** at runtime; process samples as `f32`.

1. **`src/dsp/effects/<effect>.rs`** — a `struct` holding state with a `process(sample: f32) -> f32` (or stereo) method. Reuse the shared helpers above.
2. **`src/dsp/effects/mod.rs`** — declare and re-export the module.
3. **`src/dsp/mod.rs`** — add a field to `DspChain`, a line in `DspChain::process` using the `mono_stage!` / `stereo_stage!` macro (so it bypasses cleanly and reads `Params`), and the `Params` fields + defaults: one `Arc<AtomicF32>` per knob **and** the `<fx>_enabled: Arc<AtomicBool>` bypass flag.
4. **`src/dsp/draw.rs`** - update `render_header` chain
5. **`src/ui/config.rs`** — add the `<FX>_START` / `<FX>_END` knob-range constants and the `KNOBS` entries (in top-to-bottom on-screen order — `←`/`→` walks the array linearly), then a `PEDALS` entry (name, livery colour accessor, knob range, `enabled`-flag accessor). The tile grid, detail editor, `+ ADD` picker, and `D`-to-remove all derive from this table.
6. **`src/ui/styles.rs`** — add the `PEDAL_<NAME>` colour used by the `PEDALS` entry.
7. **`src/ui/input.rs`** — add the pedal's `enabled` flag to the `toggle_pedal` match so `Space` can bypass it on the board.
8. **`src/preset.rs`** — add the preset section/fields (including the on/off state, so a preset can place the pedal on the board — `sync_board` reads the enabled flags after load). Mirror an existing optional section like `FuzzSection`.
9. **Tests** — a `#[cfg(test)]` module in the effect file: finite/bounded output, and that each knob moves the band/level it should.

## Part B — the docs (required, same PR)

Data-driven HTML blocks inside Markdown — no new page. Use the **same livery colour** in all three, and place the pedal at its correct chain position in each ordered list.

1. **`site/assets/site.css`** — if the pedal needs a new colour, add a CSS variable to the `:root` block (web twin of the `PEDAL_<NAME>` colour in `styles.rs`; existing ones like `--green`, `--teal`).
2. **`site/pedals.md`** — in the *All pedals* selector (`<div class="selector" data-tabs data-tabs-hash>`), add a `<button class="tile" data-tab="<id>" style="--c:var(--<colour>)">` **and** a matching `<div class="tab-panel" data-panel="<id>" style="--c:var(--<colour>)">` with an intro `<p class="muted">` and one `.kv` row per knob. Keep the tile in signal-chain order; `<id>` becomes the `pedals.html#<id>` deep link. If you change the chain wording, update the header count in the frontmatter `description`/`lead` and the chain-order `<div class="note">`.
3. **`site/index.md`** — add a `<div class="pedal" style="--c:var(--<colour>)">` card to the board grid (landing-page overview).
4. **`site/how-it-works.md`** — add a `<div class="flow__stage" style="--c:var(--<colour>)">` at the correct point in the signal-chain flow, with a one-line DSP summary.
5. **`site/presets.md`** — add the new pedal to the bundled preset template.

## Verify

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test                      # new effect tests + preset round-trip
cd site && npm run build        # or: npm run dev  to preview
```

## Checklist

- [ ] `src/dsp/effects/<effect>.rs` + re-export in `effects/mod.rs`
- [ ] `DspChain` field, `process` macro stage, `Params` knobs + `_enabled` flag
- [ ] `config.rs` knob-range consts, `KNOBS` rows, `PEDALS` entry
- [ ] `styles.rs` `PEDAL_<NAME>` colour + `site.css` `--<colour>` var
- [ ] `toggle_pedal` match arm in `input.rs`
- [ ] preset section in `preset.rs` (with enabled state)
- [ ] `#[cfg(test)]` tests in the effect file
- [ ] docs: `pedals.md` tile + panel, `index.md` card, `how-it-works.md`, `presets.md` stage
- [ ] all four `cargo`/`npm` checks pass; manual audio pass done
- [ ] code + docs in the same PR
