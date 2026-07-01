# Agents.md — rusty-amp

## Project overview

rusty-amp is a real-time guitar amplifier emulator that runs in the terminal. It captures audio from an audio interface, processes it through a full signal chain (noise gate → overdrive pedals → amp model → cabinet simulation → EQ → flanger → delay → reverb), and writes the processed signal back out. The UI is a ratatui TUI with live VU meters, knob sections, and a preset browser.

**Platform:** multiplatform (via cpal)
**Language:** Rust
**Minimum Rust:** 1.96+

---

## Architecture

### Signal chain (in order)

```text
Guitar input
  → Noise Gate          (envelope follower + gain ramp)
  → Compressor          (peak-follower detector → hard-knee gain computer)
  → Fuzz                (Big Muff style: DC block → 70 Hz HP → two cascaded soft-clips → mid scoop → variable tone LP)
  → TS-808 Tube Screamer (DC block → 340 Hz HP → asymmetric diode soft-clip → variable tone LP)
  → DS-1 Distortion     (DC block → 80 Hz HP → silicon diode hard-clip → active tone LP/HP blend)
  → Pre-amp EQ          (low shelf 100 Hz / mid peak 650 Hz / high shelf 3 kHz — shapes what the amp clips)
  → Amp model           (switchable: Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead — 8× oversampled)
  → Cabinet sim         (switchable: Mesa 4×12 Vintage 30 | Marshall 4×12 Greenback | Orange PPC412 Vintage 30 — multi-mic IR)
  → Parametric EQ       (low shelf 120 Hz / mid peak 800 Hz Q 1.5 / high shelf 5 kHz)
  → Flanger             (stereo LFO-swept comb: 0.5–5 ms delay, 0.05–5 Hz rate, feedback capped at 90%, L/R a quarter-cycle apart)
  → Delay               (stereo ping-pong, 0–500 ms, feedback capped at 85%)
  → Stereo Reverb       (dual decorrelated Freeverb cores: 8 parallel combs → 4 series allpasses each)
  → Master-bus widener  (stereo mid/side enhancement)
  → Output limiter      (per-channel soft-clip)
```

Every bypassable stage can be toggled independently with `Space`.

### Key modules to know

| Area | What it does |
| ------ | ------------- |
| Audio engine | Real-time processing loop — latency-sensitive, no allocations on the hot path |
| Amp models | Three distinct DSP paths (tube soft-clip, silicon clip, solid-state rail-clip) with per-model tone stacks and rectifier sag simulation |
| Cabinet sim | Multi-stage biquad EQ chains that model close-mic'd 4×12 responses |
| TUI | ratatui-based UI: selector row (amp + cabinet), pedals row, amp/FX row, VU meters, preset browser overlay |
| Preset system | TOML files loaded from `./presets/` and `~/.config/rusty-amp/presets/` |
| Docs website | `site/` — Markdown + inline HTML rendered by Eleventy, published to GitHub Pages |

---

## Running the project

```bash
cargo run            # debug build
cargo run --release  # release build (use this when testing audio — debug can underrun)
```

Startup prompts for input device → input channel → output device. The processed signal is written to all output channels.

---

## Documentation website

The user-facing docs live in [`site/`](site/) and publish to GitHub Pages at
<https://danylokravchenko.github.io/rusty-amp/>. Pages are **Markdown with inline
HTML** for the interactive components, rendered through a shared layout by
[Eleventy](https://www.11ty.dev/) (11ty). The pedal, amp, and cabinet docs are
data-driven HTML blocks inside Markdown — no per-item pages.

```bash
cd site
npm install
npm run dev     # local preview with live reload
npm run build   # render to ./_site (what CI publishes)
```

Pages: `index` (landing + board grid), `getting-started`, `pedals`, `amps-cabs`,
`presets`, `plugins`, `how-it-works`. Each pedal has a **livery colour** whose web
twin is a CSS variable in the `:root` block of `site/assets/site.css`, matching a
`PEDAL_*` colour in `src/ui/styles.rs`.

**Anything that changes a pedal, amp, cabinet, control, or preset must update the
docs in the same PR.** See [`CONTRIBUTING.md`](CONTRIBUTING.md) ("Documentation
site" / "Documenting a new pedal") and [`site/README.md`](site/README.md) for the
authoring conventions.

---

## DSP constraints (important for agents touching audio code)

- **No heap allocations on the audio thread.** All buffers must be pre-allocated. Do not introduce `Vec::push`, `Box::new`, channels that allocate, or any other allocating call inside the real-time callback.
- **No blocking on the audio thread.** No mutexes that can be held by the UI thread while the audio thread waits. Use lock-free primitives (atomics, `Arc`, single-producer channels) for communication between the UI and audio threads.
- **Biquad state must be preserved across buffer boundaries.** Filter state lives outside the processing loop and is passed in by reference each call.
- **Knob values are normalized 0.0–1.0 internally.** The TUI displays them as 0–10; conversion happens at the UI layer.
- **Rectifier sag is stateful.** The sag envelope has attack and release times that differ per amp model; do not reset this state on model switch unless explicitly tested.
