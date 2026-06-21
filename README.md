# rusty-amp

A guitar amplifier emulator that runs in your terminal. Connect an audio interface and play — all controls live in a ratatui TUI with real-time VU meters, two rows of effect sections, and switchable amp and cabinet models. The signal becomes **stereo** at the cabinet stage (a blended multi-mic impulse-response convolution) and stays stereo through a ping-pong delay, stereo reverb, and a master-bus widener for a wide, studio-grade image. The amp's nonlinear stages run at **8× oversampling** for creamy, alias-free high-gain saturation; the tube amps use a real **passive FMV tone stack** with a modelled **power-amp ↔ speaker interaction**; and the cabinets blend three mics (close SM57 + ribbon + room) over **~23 ms impulse responses** with room reflections and cone resonance for three-dimensional depth. Presets can be loaded at any time without leaving the app.

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
Fuzz  [bypassable · Big Muff style]
  DC block → 70 Hz HP → [4× OS: two cascaded asymmetric soft-clip stages] → DC block → 700 Hz mid scoop → variable tone LP
  │
  ▼
TS-808 Tube Screamer  [bypassable]
  DC block → 340 Hz HP → 720 Hz mid-peak → [4× OS: asymmetric diode soft-clip] → variable tone LP
  │
  ▼
DS-1 Distortion  [bypassable]
  DC block → 80 Hz HP → [4× OS: pre-clip HP → symmetric silicon diode clip] → active tone (LP+HP blend)
  │
  ▼
Amp  [Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead — switchable in real time]
  8× oversampled nonlinear stages (8th-order Butterworth anti-alias) + dynamic grid-bias "bloom" for touch sensitivity
  JCM800:   dual 12AX7 atan soft-clip → passive FMV tone stack → tube sag → speaker-load bloom
  Mesa DR:  triple gain stage (atan + silicon clip) → passive FMV tone stack → silicon sag → speaker-load bloom
  Randall:  FET → BJT → rail-clip → active tone stack → stiff solid-state power section → static speaker load
  │
  ▼  (mono → STEREO)
Cabinet  [Mesa 4×12 Vintage 30 | Marshall 4×12 Greenback | Orange PPC412 Vintage 30 — switchable in real time]
  Blended multi-mic impulse-response convolution of a 4×12 cabinet:
  close SM57 dynamic + R121 ribbon + room mic, each a ~23 ms voiced EQ
  skeleton + early-reflection comb + late room reflections + deep cone-resonance
  ring, decorrelated L/R → natural stereo width and depth · mic-position shelf
  │
  ▼  (stereo from here on)
Parametric EQ  [bypassable]
  Low shelf (120 Hz) → Mid peak (800 Hz, Q 1.5) → High shelf (5 kHz) — each ±15 dB
  │
  ▼
Delay  [bypassable]
  Stereo ping-pong delay — repeats bounce L↔R — TIME 0–500 ms, FEEDBACK 0–85%, dry/wet MIX
  │
  ▼
Stereo Reverb  [bypassable]
  Dual decorrelated Freeverb cores (8 parallel combs → 4 series allpasses each) → dry/wet mix
  │
  ▼
Master-bus mid/side stereo widener (mono center preserved)
  │
  ▼
Per-channel output soft limiter → stereo (L, R)
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

The processed signal is **true stereo**: the left channel goes to output 0, the right to output 1 (a mono output device receives the summed mix). On a stereo interface or headphones you hear the full multi-mic cab spread, ping-pong delay, and stereo reverb.

The app launches immediately with default values. Press **`P`** at any time to open the preset browser and load a preset while playing. Press **`S`** at any time to save the current state as a new preset. Press **`R`** to start or stop recording the processed output to a WAV file.

## Controls

| Key | Action |
| ----- | -------- |
| `Tab` / `Shift-Tab` | Jump between sections |
| `←` / `→` | Move focus one knob at a time within the focused section |
| `↑` / `+` / `=` | Increase focused knob by 5 % — or cycle amp model forward when on the selector row |
| `↓` / `-` | Decrease focused knob by 5 % — or cycle amp model backward when on the selector row |
| `A` | Cycle amp model forward (works from any section) |
| `C` | Cycle cabinet model (Mesa V30 → Marshall Greenback → Orange PPC412) |
| `Space` | Toggle the focused pedal / effect on / off |
| `P` | Open the preset browser overlay |
| `S` | Save the current state as a new user preset |
| `R` | Start / stop recording — saves a WAV file to your home directory when stopped |
| `Q` / `Ctrl-C` | Quit |

### Preset browser (`P`)

| Key | Action |
| ----- | -------- |
| `↑` / `↓` | Navigate the preset list |
| `Enter` | Apply the selected preset (takes effect immediately, audio is uninterrupted) |
| `S` | Open the save dialog to capture the current state as a new preset |
| `D` | Delete the selected preset (user presets only — bundled presets cannot be deleted) |
| `Esc` / `P` | Close without changing anything |

User presets are marked with a `[user]` tag in the list. The `D` hint appears in the footer only when the cursor is on a deletable preset.

### Save dialog (`S`)

| Key | Action |
| ----- | -------- |
| `Tab` | Switch between Name and Description fields |
| `Enter` | Save the preset and return to the browser |
| `Esc` | Cancel without saving |

The preset is written to `~/.config/rusty-amp/presets/<name>.toml` and appears in the browser immediately — no restart required.

Focus starts on the **selector row** (amp + cabinet). Tab moves into the pedals row and cycles through sections.

## Knob sections

### Pedals row  _(Fuzz | TS-808 | DS-1 | Stereo Reverb | Delay | Noise Gate — each independently bypassable with Space)_

#### Noise Gate

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Thresh | 0–10 | Gate open threshold. 0 = opens at very low levels (−80 dB), 10 = always open. Start around 2–3 for high-gain tones |
| Release | 0–10 | How long the gate stays open after the signal drops below threshold. Higher = slower, more natural decay |

#### Fuzz  _(Big Muff style)_

Runs first in the chain so it sees the rawest pickup signal. Two cascaded clipping stages give the long, singing sustain and near-square saturation of a vintage fuzz — much heavier than the TS or DS-1. The voice is mid-scooped at 700 Hz for the classic "wall of sound" timbre.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Fuzz | 0–10 | Sustain/gain into the two cascaded soft-clip stages. High values drive the waveform toward a gated square wave |
| Tone | 0–10 | Low-pass after the scoop. 0 = dark/woolly (~400 Hz), 10 = bright/buzzy (~6 kHz) |
| Level | 0–10 | Output volume of the pedal into the next stage |

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

#### Stereo Reverb  _(bypassable with Space)_

Two decorrelated Freeverb cores (the right channel's delay lines are offset) produce a wide, deep stereo tail.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Room | 0–10 | Decay time (Freeverb room size) |
| Damp | 0–10 | High-frequency absorption in the feedback path |
| Mix | 0–10 | Dry/wet blend (0 = fully dry, 10 = fully wet) |

#### Delay  _(bypassable with Space)_

Stereo ping-pong: feedback cross-feeds the two channels so repeats bounce left ↔ right.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Time | 0–10 | Delay time 0–500 ms |
| Feedback | 0–10 | Repeat level. Capped at 85% internally to prevent runaway |
| Mix | 0–10 | Dry/wet blend |

### Amp / FX row  _(Amp | Parametric EQ | Cabinet mics)_

#### Amp  _(model selected with `↑`/`↓` or `A` on the selector row)_

| Knob | Range | Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead |
| ------ | ------- | ---------------- | -------------------- | --------------- |
| Gain | 0–10 | Preamp gain 1×–40× into dual 12AX7 | Preamp gain 1×–36× into three stages | Preamp gain 1×–46× into FET+BJT stages |
| Bass | 0–10 | Passive FMV tone stack — bass/mid/treble interact like the real network (Marshall component values) | Passive FMV tone stack (Fender-type values: fuller lows, gentler scoop) | Active tone stack — low shelf at 80 Hz |
| Mid | 0–10 | …the mid pot sets the depth of the stack's inherent scoop | …gentler scoop than the Marshall | Peak EQ at 500 Hz |
| Treble | 0–10 | …treble interacts with mid/bass, lossy & peak-normalised | …same interacting network | High shelf at 4.5 kHz |
| Presence | 0–10 | High shelf at 3.5 kHz (±6 dB) | High shelf at 4 kHz (±6 dB) | High shelf at 5 kHz (+3 dB fixed offset, ±6 dB) |
| Master | 0–10 | Post-amp output level | Post-amp output level | Post-amp output level |

The tube amps (Marshall, Mesa) drive a **passive FMV tone stack** — a single RC
network where the three controls interact and the mids inherently scoop, exactly
like a real amp — followed by a **power-amp ↔ speaker interaction** model: the
speaker's impedance resonance blooms the low end dynamically as the supply sags
under hard playing. The Randall keeps an active (independent-band) stack and a
small static speaker resonance, true to its stiff solid-state design.

#### Parametric EQ  _(bypassable with Space)_

All three bands map 0–10 to −15 dB → 0 dB → +15 dB. Centre (5.0) is unity gain.

| Knob | Frequency | Type |
| ------ | ----------- | ---- |
| Low | 120 Hz | Low shelf |
| Mid | 800 Hz | Peak (Q 1.5) |
| High | 5 kHz | High shelf |

#### Cabinet mics  _(labeled with the active cabinet model)_

Three controls model a multi-mic'd 4×12 — the close mic's position, a blend from a
dynamic to a ribbon, and a room mic for depth. The blend is a weighted sum of the
three mics' impulse responses, so it costs no extra per-sample CPU.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Mic | 0–10 | Close-mic position: 0 = edge (off-axis, dark, −6 dB at 5 kHz) · 5 = centre neutral · 10 = on-axis (bright, +6 dB at 5 kHz) |
| Blend | 0–10 | Close-mic capsule: 0 = SM57 dynamic (bright, present) · 10 = R121 ribbon (darker, fuller low-mids, silky top) |
| Room | 0–10 | Amount of a distant room mic mixed in — adds air and three-dimensional depth (0 = dry close mic only) |

## Amp models

| Model | Character | Tone stack | Rectifier / power | Gain stages |
| ------- | ----------- | ----------------- | ----------------- | ----------- |
| Marshall JCM800 | Punchy, dynamic, touch-sensitive | Passive FMV (Marshall values) | Tube sag (5 ms attack, 200 ms release) + dynamic speaker-load bloom | 2 × 12AX7 atan soft-clip |
| Mesa Dual Rectifier | Compressed, aggressive, modern | Passive FMV (Fender values) | Silicon sag (0.5 ms attack, 80 ms release) + dynamic speaker-load bloom | 3-stage: atan → atan → exponential |
| Randall Warhead | Tight, crushing, solid-state | Active, independent bands + fixed +3 dB presence | No sag — stiff solid-state rails + static speaker resonance | FET (x/√(1+x²)) → BJT (tanh) → rail-clip |

## Cabinet models

| Model | Character | Key frequencies |
| ------- | ----------- | --------------- |
| Mesa 4×12 (Vintage 30) | Scooped, aggressive, forward-projecting | −5 dB mid scoop at 400 Hz, +7 dB presence at 3.5 kHz, hard rolloff above 6 kHz |
| Marshall 4×12 (Greenback) | Warm, mid-forward, smooth top end | +4 dB body at 800 Hz, +5 dB presence at 2.5 kHz, soft rolloff above 5 kHz |
| Orange PPC412 (Vintage 30) | Thick, chunky, mid-forward (closed-back birch) | +5 dB low-mid "wall" at 600 Hz, +3 dB grind at 1.2 kHz, +5 dB presence at 3.2 kHz |

Each cabinet is rendered by **impulse-response convolution** rather than a plain EQ. The IRs are synthesized in-code (no external `.wav` files): the model's voiced EQ provides the magnitude skeleton, then early reflections (comb filtering), late cabinet/room reflections, and speaker modal resonances — including a deep, long-decaying cone "thump" — add the time-domain depth of a real miked cab. Each IR runs ~23 ms (~1100 taps at 48 kHz), long enough for the late room reflections and the low cone resonance to ring out. Two slightly different left/right IRs decorrelate the stereo image for natural width.

Each cabinet is captured by **three mics** — a close SM57 dynamic, a close R121 ribbon, and a room mic — each with its own voicing and reflection texture (the room mic carries extra pre-delay and denser late reflections for air). The **Blend** and **Room** knobs mix these captures. Because convolution is linear, the blend is just a weighted **sum of the three IRs**, recombined into the live convolver only when a knob moves — so any mic mix costs exactly two convolutions per sample, no more.

Cycle between cabinet models with `C` at any time. The cabinet state is preserved when switching amp models.

The **Mic** knob applies a high-shelf filter (±6 dB at 5 kHz) per channel after convolution, modelling the tonal difference between an on-axis and off-axis close-mic placement.

## Presets

Presets are `.toml` files. rusty-amp searches these directories, in order:

1. `./presets/` — bundled presets (read-only, shipped with the repo)
2. `~/.config/rusty-amp/presets/` — your personal presets

Press **`P`** while playing to open the preset browser. Press **`S`** (from anywhere) to save the current state as a new user preset. The browser updates instantly — no restart required.

Bundled presets are marked as system presets and cannot be deleted from within the app. User presets show a `[user]` tag and can be deleted with **`D`**.

### Bundled presets

| File | Amp | Cabinet | Description |
| ------ | --- | ------- | ----------- |
| `metallica.toml` | Marshall JCM800 | Marshall Greenback | Hetfield's rhythm tone — TS clean boost, scooped mids, bone dry |
| `pantera.toml` | Randall Warhead | Mesa V30 | Dimebag's rhythm tone — DS-1, deep mid-scoop, Furman PQ-3 EQ |
| `pantera_floods.toml` | Randall Warhead | Mesa V30 | Floods solo — DS-1 light, open mids, delay + reverb |
| `slipknot.toml` | Mesa Dual Rectifier | Mesa V30 | Mick Thomson / Jim Root — TS boost, modern EQ scoop, full saturation |
| `death.toml` | Mesa Dual Rectifier | Mesa V30 | Chuck Schuldiner — TS boost, mids-up for note clarity |
| `slayer.toml` | Marshall JCM800 | Marshall Greenback | Hanneman & King's thrash assault — straight into a cranked JCM800, extreme mid-scoop, zero mercy |
| `metalcore_shred.toml` | Mesa Dual Rectifier | Mesa V30 | Modern metalcore shred — TS tight boost, djent-adjacent EQ, slapback delay |
| `solo_seeker.toml` | Mesa Dual Rectifier | Mesa V30 | Lead tone — sustain-focused, delay + reverb, on-axis mic for pick-attack clarity |

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

Drop the file in `~/.config/rusty-amp/presets/` and it will appear in the preset browser the next time you open it (or save another preset to trigger a reload).

## Recording

Press **`R`** to start recording. The header switches from `○ OFF AIR` to a blinking `● ON AIR` indicator next to `POWER ON`. Press **`R`** again to stop — the file is written immediately and the saved path is shown briefly in the footer.

Recordings capture the fully-processed signal (after the entire effects chain and output limiter) as a 32-bit float **stereo** WAV at the same sample rate as your audio interface — the full multi-mic cab spread and stereo effects are preserved. Files are named `rusty-amp-<unix-timestamp>.wav` and saved to your home directory (`~/`).

## License

Apache 2.0
