# Contributing to rusty-amp

Thanks for your interest in contributing. This is a real-time audio DSP project written in Rust — contributions are welcome in all areas: sound design, new effects, amp/cabinet models, UI improvements, bug fixes, and preset additions.

## Getting started

```bash
git clone https://github.com/danylokravchenko/rusty-amp
cd rusty-amp
cargo build
cargo run
```

Run with `--release` for realistic audio performance:

```bash
cargo run --release
```

## Project layout

```txt
src/
  main.rs           — entry point, audio thread setup, event loop
  preset.rs         — preset load/save/serialization
  recording.rs      — WAV recording
  audio/            — cpal device selection and audio callback
  dsp/              — all signal processing
    amp/            — amp models (marshall, mesa, randall)
    cab/            — cabinet IR convolution and mic models
    effects/        — pedals & rack effects + their shared building blocks
      mod.rs        — OnePoleLp, ThreeBandEq, dB/param helpers (shared logic)
      compressor.rs, fuzz.rs, tube_screamer.rs, distortion.rs,
      preamp_eq.rs, parametric_eq.rs, delay.rs, reverb.rs, noise_gate.rs
    biquad.rs       — biquad filter building block
    tonestack.rs    — passive FMV tone stack
    oversample.rs   — polyphase N× oversampling (with a `process(x, f)` clip helper)
    mod.rs          — Params, DspChain, and the bypass-stage macros that wire it
  ui/
    mod.rs          — UI loop, board (on-board pedal) state, modal wiring
    draw.rs         — ratatui rendering (rig tile grid + detail editor)
    input.rs        — keyboard handling and board-aware navigation
    setup.rs        — device-selection modals shown before audio starts
    presets.rs      — preset browser overlay
    config.rs       — KNOBS + PEDALS tables and knob-range constants
    styles.rs       — colour palette and styles
presets/            — bundled read-only presets (TOML)
```

## Code style

The project enforces strict Clippy lints. Before opening a PR:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Comments should only explain *why*, not *what*. Well-named identifiers carry the what.

## DSP conventions

The audio callback runs on a dedicated real-time thread. Code that touches the signal path must be **allocation-free and panic-free** at runtime.

- Process samples as `f32`
- Oversampled stages live inside `oversample.rs`; add new ones there
- Biquad filters are built with `biquad.rs` helpers; do not implement raw filter math inline
- The FMV tone stack is in `tonestack.rs` — reuse it for tube amp models
- Cabinet IRs are synthesized in-code (no external `.wav` files); see `cab/` for the pattern

### Adding a new DSP effect

Effects live in `src/dsp/effects/`. Each is a self-contained `struct` with a
`process` method; logic shared between several effects lives in
`src/dsp/effects/mod.rs` so the individual files stay focused on their *voicing*:

- `OnePoleLp` — the variable low-pass behind passive tone controls (TS, fuzz).
- `ThreeBandEq` — low-shelf / mid-peak / high-shelf trio shared by the pre-amp and
  parametric EQs.
- `db_to_lin` / `lin_to_db` — decibel conversions for dynamics stages.
- `param_changed` — the "did this knob move enough to rebuild coefficients?" test.
- `Oversampler::process(x, f)` (in `oversample.rs`) — runs a clipper closure at the
  oversampled rate; reuse it instead of re-writing the up/map/down loop.

To add one:

1. Create `src/dsp/effects/<effect>.rs` with a `struct` that holds state and a
   `process(sample: f32) -> f32` (or stereo equivalent) method. Reuse the shared
   helpers above rather than re-implementing tone LPs, EQ trios or dirty-checks.
2. Declare and re-export it in `src/dsp/effects/mod.rs`.
3. Add a field to `DspChain` and a line in `DspChain::process` using the
   `mono_stage!` / `stereo_stage!` macro (in `src/dsp/mod.rs`) so it bypasses
   cleanly and reads its knobs from the shared `Params`.
4. Add the `Params` fields + defaults in `src/dsp/mod.rs` — the per-knob
   `Arc<AtomicF32>` values **and** the `<fx>_enabled: Arc<AtomicBool>` bypass flag
   (the macro stage and the UI both read it).
5. In `src/ui/config.rs`, add the knob-range constants (`<FX>_START` / `<FX>_END`)
   and the matching `KNOBS` entries. Keep them in the same top-to-bottom order as
   the rest — `←`/`→` navigation walks the `KNOBS` array linearly, so the order
   *is* the on-screen layout.
6. Register the pedal in the `PEDALS` table (also `src/ui/config.rs`): one entry
   with its name, livery colour (add a `PEDAL_<NAME>` colour to `src/ui/styles.rs`),
   knob range, and `enabled`-flag accessor. The rig tile grid, the detail editor,
   the `+ ADD` picker, and `D`-to-remove all derive from this table — no `draw.rs`
   layout code is needed.
7. Update chain of effects in the header in `src/ui/draw.rs` - `render_header`
8. Add the pedal's `enabled` flag to the `toggle_pedal` match in `src/ui/input.rs`
   so `Space` can bypass it while it's on the board.
9. Add preset fields in `src/preset.rs` (including the on/off state so a preset can
   place the pedal on the board — `sync_board` reads the enabled flags after load).
10. Add a `#[cfg(test)]` module in the effect file — every effect carries unit tests
   (finite/bounded output, and that each knob moves the band/level it should).
11. Update the UI tests your `config.rs` edit touches: bump the counts in
   `table_sizes_are_stable` (`src/ui/config.rs`), then re-bless the golden
   snapshots (`INSTA_UPDATE=always cargo nextest run`) and commit the updated
   `src/ui/snapshots/*.snap` — the new pedal appears in the default board and the
   add-pedal modal. See [Testing](#testing).
12. Document the pedal on the docs site — see [Documenting a new pedal](#documenting-a-new-pedal).

### Adding a new amp model

Amp models live in `src/dsp/amp/`. Each model is a struct implementing the
`Amplifier` trait (gain stages, tone stack call, sag, and speaker interaction).
Use the existing `marshall.rs`/`mesa.rs` (tube) or `randall.rs` (solid-state)
files as a template — the shared building blocks (`FrontEnd`, `Bloom`,
`CathodeBias`, `BrightCap`, `OutputTransformer`, `SpeakerLoad`, `VoiceBalance`,
`Cached`/`ToneCache`) live in `src/dsp/amp/mod.rs`.

To add one:

1. Create `src/dsp/amp/<model>.rs` and declare/re-export it in
   `src/dsp/amp/mod.rs`; add a field to `AmpBank` and a match arm in each of
   `AmpBank::new`/`AmpBank::process`.
2. Register the model in the `AmpModel` enum in `src/dsp/mod.rs`: add the
   variant, the `from_u8` discriminant, `name()`/`short_name()` display
   strings, and splice it into the `next()`/`prev()` cycle ring.
3. Add it to the amp-selector's literal array in `src/ui/draw.rs` (the amp
   header row — cycling itself is generic over `AmpModel::next()`/`prev()`, live
   on the **`A`** key, no `input.rs` match arm needed).
4. Add the model's string in `src/preset.rs` — the `AmpSection.model` match arms
   in both `Preset::from_params` and `Preset::apply`.
5. Add the new model to `each_amp()` (and `tube_amps()`, if it's a tube model)
   in `src/dsp/amp/mod.rs`'s test module, and to the `every_amp_model_renders_its_name`
   literal array and the `cycle_amp_*` tests in `src/ui/draw.rs`/`src/ui/input.rs`.
6. Re-bless the UI golden snapshot (see [Working with snapshots](#working-with-snapshots)).
7. Document the model on the docs site — see [Documenting a new amp
   model](#documenting-a-new-amp-model).

The `add-amp-model` Claude Code skill (`.claude/skills/add-amp-model/`)
operationalizes this whole checklist end to end.

### Adding a new cabinet model

Cabinet models live in `src/dsp/cab/`. Each model synthesizes its own IR (voiced EQ skeleton + comb reflections + modal resonances). Follow the existing Mesa/Marshall/Orange pattern.

Register in the `CabinetModel` enum and cycle logic.

## Adding a bundled preset

Drop a `.toml` file in `presets/`. Follow the schema in the README — all fields are optional except `name`. Test it by running the app and pressing `P`.

Bundled presets cannot be deleted by users, so only add presets that are genuinely useful and well-tuned.

**Update the docs site in the same PR.** Add a row for the new preset to the
bundled presets table in [`site/presets.md`](site/presets.md) (`## Bundled
presets`) — file name, amp, cabinet, and a one-line description — then run
`npm run build` in `site/` to confirm it renders. A bundled preset that isn't
listed on the site is considered incomplete.

## Documentation site

The user-facing docs live in `site/` and publish to GitHub Pages
(<https://danylokravchenko.github.io/rusty-amp/>). Pages are **Markdown** with a
little inline HTML for the interactive components, rendered through a shared layout
by [Eleventy](https://www.11ty.dev/). See [`site/README.md`](site/README.md) for the
build, local preview (`npm run dev`), and authoring conventions.

Anything that changes a pedal, amp, cabinet, control, or preset should update the
docs in the same PR.

### Documenting a new pedal

The pedal docs are data-driven HTML blocks inside Markdown — no new page is needed.
Add the effect in three places, all reusing classes already defined in
`site/assets/site.css`:

1. **`site/pedals.md`** — in the *All pedals* selector (`<div class="selector"
   data-tabs data-tabs-hash>`), add a `<button class="tile" data-tab="<id>"
   style="--c:var(--<colour>)">` and a matching `<div class="tab-panel"
   data-panel="<id>" style="--c:var(--<colour>)">` with the intro paragraph and one
   `.kv` row per knob. Keep the tile in signal-chain order; the `data-tab` id
   becomes a shareable deep link (`pedals.html#<id>`).
2. **`site/index.md`** — add a `<div class="pedal" style="--c:var(--<colour>)">`
   card to the board grid so the pedal appears in the landing-page overview.
3. **`site/how-it-works.md`** — add a `<div class="flow__stage"
   style="--c:var(--<colour>)">` to the signal-chain flow at the correct point in
   the chain, with a one-line DSP summary.
4. **`site/presets.md`** — add the new pedal to the bundled preset template.

Use the **same livery colour** in all three. Each pedal colour in
`src/ui/styles.rs` has a web twin defined as a CSS variable in the `:root` block of
`site/assets/site.css` (e.g. `--green`, `--teal`); add a matching one there if the
pedal needs a new colour. Then run `npm run build` (or `npm run dev`) and confirm
the pedal shows up in the selector, the landing grid, and the flow diagram.

### Documenting a new amp model

The amp docs are a data-driven HTML block inside
[`site/amps-cabs.md`](site/amps-cabs.md) (`## Amplifiers {#amp}`) — no new page
is needed. Add the model in two places, reusing classes already defined in
`site/assets/site.css`:

1. A `<button class="tile" data-tab="<id>">` in the `.tiles` selector row:
   `.tile__name`, `.tile__sub` (one-line character blurb), and `.tile__amp`
   with 6 decorative knob-icon spans (`--r:` rotation degrees).
2. A matching `<div class="tab-panel" data-panel="<id>">`: a `.specs` block
   (`Gain range`, `Tone stack`, `Rectifier & power`, `Gain stages`) and one
   `.kv` row per knob (`Gain`, `Bass`, `Mid`, `Treble`, `Presence`, `Master`) —
   short, technical sentences (frequencies, dB, component type), no marketing
   fluff.

If the model changes the tube-vs-solid-state split described in the section's
closing paragraph, update that paragraph too. Then run `npm run build` (or
`npm run dev`) in `site/` and confirm the new tile and panel render. Don't forget about `site/index.md`, `site/presets.md` and `site/how-it-works.md` chain to include the new model.

## Testing

Run the whole suite with:

```bash
cargo nextest run
```

It runs on every commit in CI (`cargo clippy --all-targets --all-features -D
warnings` then `cargo nextest run`).

### DSP tests

Every effect, amp and cabinet model carries `#[cfg(test)]` tests covering
stability (finite, bounded output) and behaviour (each control moves the
band/level it should). New DSP code must come with tests in the same file.

### UI tests

The terminal UI is tested at three levels, all as `#[cfg(test)]` modules inside
`src/ui/` (they need the modules' `pub(super)` internals):

- **Table invariants** (`config.rs`) — the `KNOBS` ↔ `PEDALS` contract: pedal
  knob-ranges tile contiguously, `pedal_of` round-trips, and a deliberate
  `table_sizes_are_stable` tripwire pins the pedal/knob counts so a table change
  is always a conscious edit.
- **Navigation & state** (`input.rs`) — Tab/`←→` cycling, off-board pedals being
  skipped, knob clamping, amp/cab cycling, and add/remove/toggle board logic.
- **Rendering** (`draw.rs`) — the screen is drawn to an in-memory
  [ratatui](https://ratatui.rs) `TestBackend`. Assertion tests confirm every
  pedal/amp/cabinet renders its name and controls; **golden snapshots**
  ([insta](https://insta.rs)) lock the exact glyphs of the default screen and each
  modal (add-pedal, presets, save, plugin browser, external-IR browser).

#### Working with snapshots

Snapshots are committed under `src/ui/snapshots/*.snap` — that file *is* the
expected output, and CI diffs the freshly rendered screen against it. When a change
alters the UI on purpose, regenerate and review the golden, then commit it:

```bash
cargo insta test --review     # or: INSTA_UPDATE=always cargo nextest run
```

Two things to know:

- **Never set `INSTA_UPDATE` in CI** — that would auto-accept and mask regressions.
  A stray pending `*.snap.new` also fails CI, so finish the re-bless and commit the
  `.snap`.
- The golden tests are gated on the default `clap` feature, because the help footer
  renders a `clap`-only key. Regenerate with default features on.

### Manual audio pass

The automated tests don't listen, so an audio change still needs an ear:

1. `cargo run --release` with an audio interface connected
2. Verify the effect under test across the full gain range
3. Confirm no audible clicks or artifacts when toggling bypass or switching models
4. Confirm the preset round-trips correctly (save → reload → values match)

## Submitting a PR

1. Fork the repo and create a branch from `main`
2. Make your change — keep scope tight, one logical change per PR
3. Run `cargo fmt && cargo clippy --all-targets -- -D warnings` and fix any issues
4. Open a PR against `main` with a short title and a description of *why* the change is needed
5. Include a brief note on how you tested it

## License

By contributing you agree that your work will be released under the [Apache 2.0 license](LICENSE).
