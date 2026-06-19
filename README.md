# rusty-amp

A guitar amplifier emulator that runs in your terminal. Connect an audio interface and play — all controls live in a ratatui TUI with real-time VU meters, two rows of effect sections, and switchable amp and cabinet models. Presets can be loaded at any time without leaving the app.

![Screenshot](/assets/screenshot.png)

## Signal chain

```text
Guitar
  │
  ▼
Noise Gate  [bypassable]
  Envelope follower → gain ramp (smooth open/close to avoid clicks)
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
Amp  [Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead — switchable in real time]
  JCM800:   dual 12AX7 atan soft-clip → passive tone stack → tube rectifier sag
  Mesa DR:  triple gain stage (atan + silicon clip) → tone stack → silicon rectifier sag
  Randall:  FET → BJT → rail-clip → active tone stack → stiff solid-state power section
  │
  ▼
Cabinet  [Mesa 4×12 Vintage 30 | Marshall 4×12 Greenback — switchable in real time]
  Multi-stage biquad EQ chain modelling a close-mic'd 4×12 cabinet response:
  sub-bass HPF → low shelf → mid character peak → presence peak → air shelf → fizz LPF
  │
  ▼
Parametric EQ  [bypassable]
  Low shelf (120 Hz) → Mid peak (800 Hz, Q 1.5) → High shelf (5 kHz) — each ±15 dB
  │
  ▼
Delay  [bypassable]
  Digital delay with feedback — TIME 0–500 ms, FEEDBACK 0–85%, dry/wet MIX
  │
  ▼
Spring Reverb  [bypassable]
  8 parallel comb filters → 4 series allpass diffusers → dry/wet mix
  │
  ▼
Output soft limiter (tanh)
```

## Requirements

- **macOS** (uses CoreAudio via cpal)
- **Rust** 1.95+ (`rustup` recommended)
- An **audio interface** with a high-impedance instrument input (e.g. Focusrite Scarlett)
- macOS **microphone permission** granted to Terminal.app  
  → System Settings › Privacy & Security › Microphone

## Run

```bash
cargo run
# or after building:
./target/release/rusty-amp
```

### Startup flow

1. **Select input device** — your audio interface appears in the list
2. **Select guitar input channel** — Focusrite 2i2 has 2; guitar is usually channel 2 if plugged into Input 2
3. **Select output device** — pick your speakers or headphones
4. **Select output channel** — which speaker/headphone channel to route the processed signal to

The app launches immediately with default values. Press **`P`** at any time to open the preset browser and load a preset while playing.

## Controls

| Key | Action |
| ----- | -------- |
| `Tab` / `Shift-Tab` | Jump between sections |
| `←` / `→` | Move focus one knob at a time within the focused section |
| `↑` / `+` / `=` | Increase focused knob by 5 % — or cycle amp model forward when on the selector row |
| `↓` / `-` | Decrease focused knob by 5 % — or cycle amp model backward when on the selector row |
| `A` | Cycle amp model forward (works from any section) |
| `C` | Toggle cabinet model between Mesa V30 and Marshall Greenback |
| `Space` | Toggle the focused pedal / effect on / off |
| `P` | Open the preset browser overlay |
| `Q` / `Ctrl-C` | Quit |

### Preset browser (`P`)

| Key | Action |
| ----- | -------- |
| `↑` / `↓` | Navigate the preset list |
| `Enter` | Apply the selected preset (takes effect immediately, audio is uninterrupted) |
| `Esc` / `P` | Close without changing anything |

Focus starts on the **selector row** (amp + cabinet). Tab moves into the pedals row and cycles through sections.

## Knob sections

### Pedals row  _(each section is independently bypassable with Space)_

#### Noise Gate

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Thresh | 0–10 | Gate open threshold. 0 = opens at very low levels (−80 dB), 10 = always open. Start around 2–3 for high-gain tones |
| Release | 0–10 | How long the gate stays open after the signal drops below threshold. Higher = slower, more natural decay |

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

#### Spring Reverb

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Room | 0–10 | Decay time (Freeverb room size) |
| Damp | 0–10 | High-frequency absorption in the feedback path |
| Mix | 0–10 | Dry/wet blend (0 = fully dry, 10 = fully wet) |

### Amp / FX row

#### Amp  _(model selected with `↑`/`↓` or `A` on the selector row)_

| Knob | Range | Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead |
| ------ | ------- | ---------------- | -------------------- | --------------- |
| Gain | 0–10 | Preamp gain 1×–40× into dual 12AX7 | Preamp gain 1×–36× into three stages | Preamp gain 1×–46× into FET+BJT stages |
| Bass | 0–10 | Low shelf at 80 Hz (±15 dB) | Low shelf at 100 Hz (±15 dB) | Low shelf at 80 Hz (±15 dB) |
| Mid | 0–10 | Peak EQ at 400 Hz (±12 dB) | Peak EQ at 750 Hz (±12 dB) | Peak EQ at 500 Hz (±12 dB) |
| Treble | 0–10 | High shelf at 2.5 kHz (±15 dB) | High shelf at 3.3 kHz (±15 dB) | High shelf at 4.5 kHz (±15 dB) |
| Master | 0–10 | Post-amp output level | Post-amp output level | Post-amp output level |

#### Parametric EQ  _(bypassable with Space)_

All three bands map 0–10 to −15 dB → 0 dB → +15 dB. Centre (5.0) is unity gain.

| Knob | Frequency | Type |
| ------ | ----------- | ---- |
| Low | 120 Hz | Low shelf |
| Mid | 800 Hz | Peak (Q 1.5) |
| High | 5 kHz | High shelf |

#### Delay  _(bypassable with Space)_

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Time | 0–10 | Delay time 0–500 ms |
| Feedback | 0–10 | Repeat level. Capped at 85% internally to prevent runaway |
| Mix | 0–10 | Dry/wet blend |

## Amp models

| Model | Character | Tone stack centre | Rectifier / power | Gain stages |
| ------- | ----------- | ----------------- | ----------------- | ----------- |
| Marshall JCM800 | Punchy, dynamic, touch-sensitive | Bass 80 Hz / Mid 400 Hz / Treble 2.5 kHz | Tube sag (5 ms attack, 200 ms release) | 2 × 12AX7 atan soft-clip |
| Mesa Dual Rectifier | Compressed, aggressive, modern | Bass 100 Hz / Mid 750 Hz / Treble 3.3 kHz | Silicon sag (0.5 ms attack, 80 ms release) | 3-stage: atan → atan → exponential |
| Randall Warhead | Tight, crushing, solid-state | Bass 80 Hz / Mid 500 Hz / Treble 4.5 kHz + fixed +3 dB presence | No sag — stiff solid-state rails | FET (x/√(1+x²)) → BJT (tanh) → rail-clip |

## Cabinet models

| Model | Character | Key frequencies |
| ------- | ----------- | --------------- |
| Mesa 4×12 (Vintage 30) | Scooped, aggressive, forward-projecting | −5 dB mid scoop at 400 Hz, +5 dB presence at 3.5 kHz, hard rolloff above 6 kHz |
| Marshall 4×12 (Greenback) | Warm, mid-forward, smooth top end | +3 dB body at 800 Hz, +4 dB presence at 2.5 kHz, soft rolloff above 5 kHz |

Toggle between them with `C` at any time. The cabinet state is preserved when switching amp models.

## Presets

Presets are `.toml` files. rusty-amp searches these directories at startup, in order:

1. `./presets/` — relative to your working directory (bundled with the repo)
2. `~/.config/rusty-amp/presets/` — your personal presets

Press **`P`** while playing to open the preset browser and apply a preset without interrupting audio.

### Bundled presets

| File | Amp | Cabinet | Description |
| ------ | --- | ------- | ----------- |
| `metallica.toml` | Marshall JCM800 | Marshall Greenback | Hetfield's rhythm tone — TS clean boost, scooped mids, bone dry |
| `pantera.toml` | Randall Warhead | Mesa V30 | Dimebag's rhythm tone — DS-1, deep mid-scoop, Furman PQ-3 EQ |
| `pantera_floods.toml` | Randall Warhead | Mesa V30 | Floods solo — DS-1 light, open mids, delay + reverb |
| `slipknot.toml` | Mesa Dual Rectifier | Mesa V30 | Mick Thomson / Jim Root — TS boost, modern EQ scoop, full saturation |
| `death.toml` | Mesa Dual Rectifier | Mesa V30 | Chuck Schuldiner — TS boost, mids-up for note clarity |
| `metalcore_shred.toml` | Mesa Dual Rectifier | Mesa V30 | Modern metalcore shred — TS tight boost, djent-adjacent EQ, slapback delay |

### Writing your own preset

```toml
name        = "My Preset"
description = "Optional one-line description shown in the preset browser."

# All sections except [tube_screamer], [amp], and [reverb] are optional.
# Omitting a section leaves that effect's current state unchanged.

[noise_gate]
enabled   = true    # optional, defaults to true
threshold = 0.20    # 0.0 – 1.0  (0 = barely open, 1 = always open)
release   = 0.30    # 0.0 – 1.0  (0 = instant close, 1 = very slow)

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

[amp]
model  = "marshall"   # "marshall" (default), "mesa", or "randall"
gain   = 0.65
bass   = 0.50
mid    = 0.45
treble = 0.65
master = 0.55

[cabinet]
model = "mesa"        # "mesa" (default) or "marshall"

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

Drop the file in `~/.config/rusty-amp/presets/` and it will appear in the preset browser next time you run.

## License

Apache 2.0
