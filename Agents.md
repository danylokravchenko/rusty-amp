# Agents.md — rusty-amp

## Project overview

rusty-amp is a real-time guitar amplifier emulator that runs in the terminal. It captures audio from an audio interface, processes it through a full signal chain (noise gate → overdrive pedals → amp model → cabinet simulation → EQ → delay → reverb), and writes the processed signal back out. The UI is a ratatui TUI with live VU meters, knob sections, and a preset browser.

**Platform:** multiplatform (via cpal)
**Language:** Rust
**Minimum Rust:** 1.95+

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

---

## Running the project

```bash
cargo run            # debug build
cargo run --release  # release build (use this when testing audio — debug can underrun)
```

Startup prompts for input device → input channel → output device. The processed signal is written to all output channels.

---

## Preset format

Presets are `.toml` files. All sections except `[tube_screamer]`, `[amp]`, and `[reverb]` are optional — omitting a section leaves the current state unchanged.

```toml
name        = "My Preset"
description = "Optional one-line description shown in the preset browser."

# All sections except [tube_screamer], [amp], and [reverb] are optional.
# Omitting a section leaves that effect's current state unchanged.

[noise_gate]
enabled   = true    # optional, defaults to true
threshold = 0.20    # 0.0 – 1.0  (0 = barely open, 1 = always open)
release   = 0.30    # 0.0 – 1.0  (0 = instant close, 1 = very slow)

# Omit [compressor] entirely to leave it off,
# or include it with enabled = false to store values but keep it bypassed.
[compressor]
enabled = false     # optional, defaults to true when the section is present
sustain = 0.40      # 0.0 – 1.0  (compression amount)
attack  = 0.30      # 0.0 – 1.0  (0.5 ms → 50 ms)
level   = 0.50      # 0.0 – 1.0  (output makeup, 0.5 = unity)

# Omit [fuzz] entirely to leave it off (the default for the bundled presets),
# or include it with enabled = false to store values but keep it bypassed.
[fuzz]
enabled = false     # optional, defaults to true when the section is present
fuzz  = 0.70        # 0.0 – 1.0  (sustain/gain)
tone  = 0.50
level = 0.60

[tube_screamer]
enabled = true      # optional, defaults to true
drive = 0.40        # 0.0 – 1.0
tone  = 0.60
level = 0.70

# Omit [distortion] entirely to leave it off,
# or include it with enabled = false to store values but keep it bypassed.
[distortion]
enabled = true
drive = 0.50
tone  = 0.55
level = 0.65

# Pre-amp EQ — shapes the signal before the amp's gain stage.
# Omit [preamp_eq] entirely to leave it off, or include it with enabled = false.
[preamp_eq]
enabled = false       # optional, defaults to true when the section is present
low  = 0.50           # 0.0 = −12 dB, 0.5 = flat, 1.0 = +12 dB
mid  = 0.50
high = 0.50

[amp]
model  = "marshall"   # "marshall" (default), "mesa", or "randall"
gain   = 0.65
bass   = 0.50
mid    = 0.45
treble = 0.65
master = 0.55

[cabinet]
model     = "mesa"    # "mesa" (default), "marshall", or "orange"
mic_pos   = 0.5       # 0.0 = edge/dark, 0.5 = neutral, 1.0 = center/bright (default 0.5)
mic_blend = 0.15      # 0.0 = SM57 dynamic, 1.0 = R121 ribbon (default 0.15)
mic_room  = 0.15      # 0.0 = dry close mic, 1.0 = full room mic (default 0.15)

[eq]
enabled = true        # optional, defaults to true
low  = 0.50           # 0.0 = −15 dB, 0.5 = 0 dB, 1.0 = +15 dB
mid  = 0.50
high = 0.50

[delay]
enabled  = true       # optional, defaults to true
time     = 0.30       # 0.0 = 0 ms, 1.0 = 500 ms
feedback = 0.40       # 0.0 – 1.0 (internally capped at 85%)
mix      = 0.30       # 0.0 = dry, 1.0 = fully wet

[reverb]
enabled = true        # optional, defaults to true
room = 0.55
damp = 0.40
mix  = 0.25
```

Drop new presets in `~/.config/rusty-amp/presets/` — they appear in the browser on next launch.

---

## DSP constraints (important for agents touching audio code)

- **No heap allocations on the audio thread.** All buffers must be pre-allocated. Do not introduce `Vec::push`, `Box::new`, channels that allocate, or any other allocating call inside the real-time callback.
- **No blocking on the audio thread.** No mutexes that can be held by the UI thread while the audio thread waits. Use lock-free primitives (atomics, `Arc`, single-producer channels) for communication between the UI and audio threads.
- **Biquad state must be preserved across buffer boundaries.** Filter state lives outside the processing loop and is passed in by reference each call.
- **Knob values are normalized 0.0–1.0 internally.** The TUI displays them as 0–10; conversion happens at the UI layer.
- **Rectifier sag is stateful.** The sag envelope has attack and release times that differ per amp model; do not reset this state on model switch unless explicitly tested.

---

## Adding a new amp model

1. Define DSP constants (gain range, tone stack frequencies, sag parameters).
2. Implement a processing function matching the signature of the existing models.
3. Add a variant to the amp model enum and wire it into the dispatch in the audio engine.
4. Add a `model = "<name>"` string to the preset parser.
5. Update the selector row in the TUI to include the new label.
6. Add a bundled preset that showcases the model.
7. Document the model in `README.md` under "Amp models".

---

## Adding a new effect

1. Implement the DSP as a standalone struct with `process(&mut self, sample: f32) -> f32` (or buffer-based equivalent). Keep state inside the struct.
2. Decide where in the signal chain it lives and insert it there.
3. Add bypass logic (an `enabled: bool` field; when false, pass input through unchanged).
4. Wire a new knob section into the TUI.
5. Add the section to the preset format (optional section pattern — omitting leaves current state).
6. Document it in `README.md`.
