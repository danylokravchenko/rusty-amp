# Agents.md — rusty-amp

## Project overview

rusty-amp is a real-time guitar amplifier emulator that runs in the terminal. It captures audio from an audio interface, processes it through a full signal chain (noise gate → overdrive pedals → amp model → cabinet simulation → EQ → delay → reverb), and writes the processed signal back out. The UI is a ratatui TUI with live VU meters, knob sections, and a preset browser.

**Platform:** macOS only (CoreAudio via cpal)  
**Language:** Rust  
**Minimum Rust:** 1.95+

---

## Architecture

### Signal chain (in order)

```text
Guitar input
  → Noise Gate          (envelope follower + gain ramp)
  → TS-808 Tube Screamer (DC block → HP → asymmetric diode clip → tone LP)
  → DS-1 Distortion     (DC block → HP → asymmetric hard-clip → active tone)
  → Amp model           (switchable: Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead)
  → Cabinet sim         (switchable: Mesa 4×12 V30 | Marshall 4×12 Greenback)
  → Parametric EQ       (low shelf 120 Hz / mid peak 800 Hz / high shelf 5 kHz)
  → Delay               (digital, 0–500 ms, feedback capped at 85%)
  → Spring Reverb       (8 parallel combs → 4 allpass diffusers)
  → Output soft limiter (tanh)
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
description = "Optional description shown in the preset browser."

[noise_gate]
enabled   = true
threshold = 0.20    # 0.0–1.0
release   = 0.30

[tube_screamer]
enabled = true
drive = 0.40
tone  = 0.60
level = 0.70

[distortion]
enabled = true
drive = 0.50
tone  = 0.55
level = 0.65

[amp]
model  = "marshall"   # "marshall" | "mesa" | "randall"
gain   = 0.65
bass   = 0.50
mid    = 0.45
treble = 0.65
master = 0.55

[cabinet]
model = "mesa"        # "mesa" | "marshall"

[eq]
enabled = true
low  = 0.50           # 0.0 = −15 dB, 0.5 = 0 dB, 1.0 = +15 dB
mid  = 0.50
high = 0.50

[delay]
enabled  = true
time     = 0.30       # 0.0 = 0 ms, 1.0 = 500 ms
feedback = 0.40
mix      = 0.30

[reverb]
enabled = true
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
