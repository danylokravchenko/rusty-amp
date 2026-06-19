# rusty-amp

A guitar amplifier emulator that runs in your terminal. Connect an audio interface, pick a preset, and play — all controls live in a ratatui TUI with real-time VU meters, two rows of effect pedals, and switchable amp models.

```text
╔══════════════════════════════════════════════════════════════════════════════╗
║  R U S T Y  A M P  ▐  MARSHALL JCM800  ▐  TS-808 → Marshall JCM800 → ...  ║
║  TUBE SCREAMER ──▶ PREAMP ──▶ TONE STACK ──▶ POWER AMP ──▶ REVERB ──▶ OUT ║
╠════════════════════════════════════════════════════════════════════════════ ═╣
║  INPUT   ▐████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▌ ║
║  OUTPUT  ▐████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▌ ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  AMP MODEL  ◀ Marshall JCM800 ▶  [ Mesa Dual Rectifier ]                   ║
╠═════════════════════════════╦════════════════════════════════════════════════╣
║ ⚡ TS-808 ●                 ║ ⚡ DS-1 DISTORTION ○                          ║
║  DRIVE  TONE  LEVEL         ║  DRIVE  TONE  LEVEL                           ║
╠═════════════════════════════╩════════════════════════════════════════════════╣
║ ⚡ Marshall JCM800           ║ ⚡ SPRING REVERB                              ║
║  GAIN  BASS  MID  TREBLE  MASTER  ║  ROOM  DAMP  MIX                       ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

## Signal chain

```text
Guitar
  │
  ▼
TS-808 Tube Screamer  [bypassable]
  DC block → 720 Hz HP → asymmetric diode clip → variable tone LP
  │
  ▼
DS-1 Distortion  [bypassable]
  DC block → 100 Hz HP → asymmetric hard-clip (op-amp + diodes) → active tone (LP+HP blend)
  │
  ▼
Amp  [Marshall JCM800 | Mesa Dual Rectifier — switchable in real time]
  JCM800:  dual 12AX7 atan soft-clip → passive tone stack → tube rectifier sag
  Mesa DR: triple gain stage (atan + silicon clip) → tone stack → silicon rectifier sag
  │
  ▼
Freeverb reverb
  8 parallel comb filters → 4 series allpass diffusers → dry/wet mix
  │
  ▼
Output soft limiter (tanh)
```

## Requirements

- **macOS** (uses CoreAudio via cpal)
- **Rust** 1.80+ (`rustup` recommended)
- An **audio interface** with a high-impedance instrument input (e.g. Focusrite Scarlett)
- macOS **microphone permission** granted to Terminal.app  
  → System Settings › Privacy & Security › Microphone

## Build

```bash
git clone https://github.com/you/rusty-amp
cd rusty-amp
cargo build --release
```

## Run

```bash
cargo run --release
# or after building:
./target/release/rusty-amp
```

### Startup flow

1. **Select a preset** — built-in starting values or any `.toml` file from `./presets/`
2. **Select input device** — your audio interface appears in the list
3. **Select guitar input channel** — Focusrite 2i2 has 2; guitar is usually channel 2 if plugged into Input 2
4. **Select output device** — pick your speakers or headphones
5. **Select output channel** — which speaker/headphone channel to route the processed signal to

## Controls

| Key | Action |
| ----- | -------- |
| `Tab` / `←` / `→` | Move focus between the amp selector, pedal knobs, and amp knobs |
| `↑` / `+` / `=` | Increase focused knob by 5 % — or switch amp model when on the selector |
| `↓` / `-` | Decrease focused knob by 5 % — or switch amp model when on the selector |
| `Space` | Toggle the focused pedal (TS-808 or DS-1) on / off |
| `Q` / `Ctrl-C` | Quit |

Focus starts on the **AMP MODEL** selector row. Tab once to enter the pedal knobs, Tab again to reach amp/reverb knobs. BackTab goes the other direction.

## Knob sections

### Pedals row  _(each pedal is independently bypassable with Space)_

#### TS-808 Tube Screamer

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Drive | 0–10 | Pre-clip gain (1×–51×). High values push the asymmetric diode clippers into saturation |
| Tone | 0–10 | Low-pass cutoff after clipping. 0 = dark (~500 Hz), 10 = bright (~7 kHz) |
| Level | 0–10 | Output volume of the pedal into the next stage |

#### DS-1 Distortion

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Drive | 0–10 | Gain into the hard-clip stage (1×–61×). More aggressive than the TS |
| Tone | 0–10 | Active LP/HP blend. 0 = dark & full, 5 = mid-scooped, 10 = bright & cutting |
| Level | 0–10 | Output volume of the pedal into the next stage |

### Amp row

#### Amp  _(model switchable on the selector row above the pedals)_

| Knob | Range | Marshall JCM800 | Mesa Dual Rectifier |
| ------ | ------- | ---------------- | -------------------- |
| Gain | 0–10 | Preamp gain 1×–40× into dual 12AX7 | Preamp gain 1×–36× into three stages |
| Bass | 0–10 | Low shelf at 80 Hz (±15 dB) | Low shelf at 100 Hz (±15 dB) |
| Mid | 0–10 | Peak EQ at 400 Hz (±12 dB) | Peak EQ at 750 Hz (±12 dB) |
| Treble | 0–10 | High shelf at 2.5 kHz (±15 dB) | High shelf at 3.3 kHz (±15 dB) |
| Master | 0–10 | Post-amp output level | Post-amp output level |

#### Spring Reverb

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Room | 0–10 | Decay time (Freeverb room size) |
| Damp | 0–10 | High-frequency absorption in the feedback path |
| Mix | 0–10 | Dry/wet blend (0 = fully dry, 10 = fully wet) |

## Amp models

| Model | Character | Tone stack centre | Rectifier | Gain stages |
| ------- | ----------- | ----------------- | ---------- | ----------- |
| Marshall JCM800 | Punchy, dynamic, touch-sensitive | Bass 80 Hz / Mid 400 Hz / Treble 2.5 kHz | Tube (5 ms attack, 200 ms release) | 2 × 12AX7 atan soft-clip |
| Mesa Dual Rectifier | Compressed, aggressive, modern | Bass 100 Hz / Mid 750 Hz / Treble 3.3 kHz | Silicon (0.5 ms attack, 80 ms release) | 3-stage: atan → atan → exponential |

## Presets

Presets are `.toml` files. rusty-amp searches these directories at startup, in order:

1. `./presets/` — relative to your working directory (bundled with the repo)
2. `~/.config/rusty-amp/presets/` — your personal presets

### Bundled presets

| File | Description |
| ------ | ------------- |
| `metallica.toml` | James Hetfield's JCM800 rhythm tone — TS as clean boost, scooped mids, bone dry |
| `pantera.toml` | Dimebag Darrell's crushing tone — DS-1 + extreme JCM800 gain, deep mid-scoop |

### Writing your own preset

```toml
name        = "My Preset"
description = "Optional one-line description shown in the menu."

[tube_screamer]
enabled = true   # optional, defaults to true
drive = 0.40     # 0.0 – 1.0
tone  = 0.60
level = 0.70

# Omit the [distortion] block entirely to leave it off,
# or include it with enabled = false to store the knob values but keep it bypassed.
[distortion]
enabled = true
drive = 0.50
tone  = 0.55
level = 0.65

[amp]
model  = "marshall"   # "marshall" (default) or "mesa"
gain   = 0.65
bass   = 0.50
mid    = 0.45
treble = 0.65
master = 0.55

[reverb]
room = 0.55
damp = 0.40
mix  = 0.25
```

Drop the file in `~/.config/rusty-amp/presets/` and it will appear in the menu next time you run.

## Project structure

```text
src/
├── main.rs               Entry point: preset selection → audio → TUI
├── audio/
│   └── mod.rs            Device enumeration, sample-rate negotiation, stream construction
├── dsp/
│   ├── mod.rs            Params (AtomicF32/AtomicBool/AtomicU8), Levels, DspChain
│   ├── biquad.rs         Second-order IIR filter (Audio EQ Cookbook coefficients)
│   ├── tube_screamer.rs  TS-808: DC block, 720 Hz HP, asymmetric diode clip, tone LP
│   ├── distortion.rs     DS-1: DC block, 100 Hz HP, asymmetric hard-clip, active tone stack
│   ├── marshall.rs       JCM800: dual 12AX7 stages, passive tone stack, tube rectifier sag
│   ├── mesa.rs           Dual Rectifier: 3-stage gain, higher tone centres, silicon rectifier sag
│   └── reverb.rs         Freeverb: 8 comb + 4 allpass, sample-rate scaled
├── preset.rs             TOML schema, file discovery, apply-to-params
└── ui.rs                 ratatui TUI: header, VU meters, amp selector, pedals row, amp row
presets/
├── metallica.toml
└── pantera.toml
```

## Audio thread safety

Parameters are `Arc<AtomicF32>` / `Arc<AtomicBool>` / `Arc<AtomicU8>` — the UI thread writes with `Relaxed` stores, the audio callback reads with `Relaxed` loads. No locks in the hot path. The ring buffer between input and output callbacks is `rtrb` (lock-free SPSC).

The input and output streams are forced to the same sample rate (the interface's native rate, typically 48 000 Hz) to prevent the ring buffer from overflowing or underflowing.

Pedal bypass is implemented as a true bypass in the DSP chain — when a pedal is disabled its entire processing block is skipped and the unmodified sample passes straight through to the next stage.

## License

MIT
