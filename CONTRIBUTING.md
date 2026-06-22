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
    biquad.rs       — biquad filter building block
    tonestack.rs    — passive FMV tone stack
    oversample.rs   — 8× oversampling scaffolding
    compressor.rs, delay.rs, reverb.rs, fuzz.rs, …
  ui/
    draw.rs         — ratatui rendering
    input.rs        — keyboard handling
    presets.rs      — preset browser overlay
    config.rs       — shared UI state
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

1. Create `src/dsp/<effect>.rs` with a `struct` that holds state and a `process(sample: f32) -> f32` (or stereo equivalent) method.
2. Add it to `src/dsp/mod.rs`.
3. Wire it into the signal chain in `src/audio/` (the processing closure).
4. Expose knobs in `src/ui/config.rs` and render them in `src/ui/draw.rs`.
5. Handle keyboard input in `src/ui/input.rs`.
6. Add preset fields in `src/preset.rs`.

### Adding a new amp model

Amp models live in `src/dsp/amp/`. Each model is a struct that implements the gain stages, tone stack call, sag, and speaker interaction. Use the existing Marshall or Mesa files as a template.

Register the new model in the `AmpModel` enum and match arms in `src/dsp/amp/mod.rs`, then add a `[C]` cycle entry in `src/ui/input.rs` and a label in `src/ui/draw.rs`.

### Adding a new cabinet model

Cabinet models live in `src/dsp/cab/`. Each model synthesizes its own IR (voiced EQ skeleton + comb reflections + modal resonances). Follow the existing Mesa/Marshall/Orange pattern.

Register in the `CabinetModel` enum and cycle logic.

## Adding a bundled preset

Drop a `.toml` file in `presets/`. Follow the schema in the README — all fields are optional except `name`. Test it by running the app and pressing `P`.

Bundled presets cannot be deleted by users, so only add presets that are genuinely useful and well-tuned.

## Testing

There are no automated tests for rendering and controls yet. Manual testing is required:

1. `cargo run --release` with an audio interface connected
2. Verify the effect under test across the full gain range
3. Confirm no audible clicks or artifacts when toggling bypass or switching models
4. Confirm the preset round-trips correctly (save → reload → values match)

If you add a pure DSP utility (filter math, IR generation), unit tests are welcome in a `#[cfg(test)]` module in the same file.

## Submitting a PR

1. Fork the repo and create a branch from `main`
2. Make your change — keep scope tight, one logical change per PR
3. Run `cargo fmt && cargo clippy --all-targets -- -D warnings` and fix any issues
4. Open a PR against `main` with a short title and a description of *why* the change is needed
5. Include a brief note on how you tested it

## License

By contributing you agree that your work will be released under the [Apache 2.0 license](LICENSE).
