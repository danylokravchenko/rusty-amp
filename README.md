# 🎸 rusty-amp

**A complete guitar amp and pedalboard rig that runs right in your terminal.**

Plug in your guitar, pick an amp, and play. rusty-amp recreates classic tube and solid-state amplifiers, a full board of stompbox effects, and multi-mic'd 4×12 cabinets — all driven from a fast, keyboard-only interface with live metering. It ships with artist-inspired presets, so you can dial in a great tone in seconds and tweak from there.

![Screenshot](/assets/screenshot.png)

## Highlights

- 🔊 **3 amplifiers** — Marshall JCM800, Mesa Dual Rectifier, and Randall Warhead, switchable while you play
- 📦 **3 cabinets** — Mesa, Marshall, and Orange 4×12s, each captured with three mics you can blend, plus **load your own `.wav` IR** and A/B it live against the built-ins
- 🎛️ **A full pedalboard** — noise gate, compressor, fuzz, Tube Screamer, DS-1, EQ, ping-pong delay, and stereo reverb. Add, remove, and bypass pedals on the fly; the board shows only what you're using
- 🎧 **True studio-grade stereo** — wide, three-dimensional sound from the cab, delay, and reverb
- 💾 **Ready-made presets** — instant tones inspired by Metallica, Pantera, Slayer, Death, and more
- 🔌 **CLAP plugin host** — drop a third-party CLAP effect into the chain and tweak its parameters from the TUI
- 🎵 **Built-in tuner** — press `T` for a chromatic tuner that mutes the rig to a clean signal, shows the played note, a ±cents needle, and a live note spectrum
- ⏺️ **One-key recording** straight to a stereo WAV file
- 🖥️ **Cross-platform** — runs on macOS, Windows, and Linux

## What you need

- **A guitar and an audio interface** with a high-impedance instrument input (e.g. Focusrite Scarlett)
- Speakers or headphones — for the full stereo image, use stereo output

## Install

The fastest way to start is a pre-built binary — presets are baked in, so there's nothing else to download. Grab the latest from [Releases](https://github.com/danylokravchenko/rusty-amp/releases/latest), or [build from source](#build-from-source).

**macOS (Apple Silicon):**

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-macos-aarch64 -o rusty-amp
chmod +x rusty-amp

# Remove the macOS quarantine flag (required for unsigned binaries)
xattr -d com.apple.quarantine rusty-amp

./rusty-amp
```

**Linux (x86_64):**

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-linux-x86_64 -o rusty-amp
chmod +x rusty-amp

./rusty-amp
```

> On **Windows**, no pre-built binary is published yet — [build from source](#build-from-source) instead. rusty-amp runs natively on all three platforms via [cpal](https://github.com/RustAudio/cpal), which selects the OS audio backend automatically (CoreAudio, WASAPI/ASIO, or ALSA/JACK).

## Build from source

Runs on **macOS, Windows, and Linux**. Requires **Rust 1.95+** (`rustup` recommended).

```bash
cargo run --release
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
| `I` | Open the cabinet-IR browser to load/clear an external `.wav` IR ([see below](#cabinet-irs)) |
| `X` | A/B between a loaded external IR and the built-in cab (only once an IR is loaded) |
| `Space` | Toggle (bypass) the focused pedal — or open the **Add pedal** picker when on the `+ ADD` tile |
| `Enter` | Open the **Add pedal** picker when the `+ ADD` tile is focused |
| `D` | Remove the focused pedal from the board (it is bypassed and hidden — re-add it any time from `+ ADD`) |
| `P` | Open the preset browser overlay |
| `T` | Open the **tuner** ([see below](#tuner-t)) — bypasses the rig for a clean signal |
| `V` | Open the CLAP plugin browser ([see below](#clap-plugins)) |
| `S` | Save the current state as a new user preset |
| `R` | Start / stop recording — saves a WAV file to your home directory when stopped |
| `Q` / `Ctrl-C` | Quit |

### Pedalboard (`Add` / `D`)

The **Guitar Rig** shows one compact tile per pedal that's on the board, followed by a `+ ADD` tile. Tab or `←`/`→` to a tile to load it into the full-size editor below — the editor takes on the pedal's livery colour. Only **enabled** pedals are on the board at startup; everything else lives in the picker.

| Key | Action |
| ----- | -------- |
| `Enter` / `Space` (on `+ ADD`) | Open the picker listing pedals not on the board |
| `↑` / `↓` | Navigate the picker |
| `Enter` (in picker) | Add the selected pedal and jump focus to it |
| `Esc` | Close the picker |
| `D` (on a pedal) | Remove it from the board |

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

Focus starts on the **selector row** (amp + cabinet). Tab moves down through the amp, cabinet mics, each pedal on the board, and finally the `+ ADD` tile.

## Tuner (`T`)

Press **`T`** to open the chromatic tuner. While it is open the **entire rig is bypassed** — every pedal, the amp, and the cabinet are taken out of the path and the dry guitar passes straight to the output, so you hear (and tune against) a clean signal. Pitch is estimated with the McLeod normalised square-difference function (NSDF), accurate to a few cents.

The tuner shows:

- **The detected note** — large, with its octave (e.g. `E2`), colour-coded green when in tune.
- **A ±cents needle** — `♭ ◄ … centre … ► ♯`. The marker sits left when flat and right when sharp; it turns green within ±5 cents (in tune), amber within ±15, and red beyond.
- **A verdict** — `IN TUNE`, `TUNE UP ▲` (flat), or `TUNE DOWN ▼` (sharp), plus the raw frequency in Hz.
- **A live note spectrum** — a log-spaced magnitude display from ~60 Hz to ~1.2 kHz, with the bar at the played fundamental highlighted.

| Key | Action |
| ----- | -------- |
| `Esc` / `T` | Close the tuner and restore the full rig |

Standard tuning reference: **E2 (82.41 Hz) · A2 (110.00) · D3 (146.83) · G3 (196.00) · B3 (246.94) · E4 (329.63)**.

## Knob sections

### Pedals  _(Compressor | Fuzz | TS-808 | DS-1 | Stereo Reverb | Delay | Noise Gate | Pre-amp EQ | Parametric EQ — each can be added, removed, and bypassed independently)_

#### Noise Gate

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Thresh | 0–10 | Gate open threshold. 0 = opens at very low levels (−80 dB), 10 = always open. Start around 2–3 for high-gain tones |
| Release | 0–10 | How long the gate stays open after the signal drops below threshold. Higher = slower, more natural decay |

#### Compressor

Sits right after the gate, before the drive stages, so it evens out picking dynamics and adds sustain going into the amp — the classic "studio" upgrade for clean and edge-of-breakup tones. A peak-follower detector drives a hard-knee gain computer; auto makeup keeps the level steady as you add compression.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Sustain | 0–10 | Compression amount — lowers the threshold (−6 dB → −40 dB) and raises the ratio (2:1 → 10:1). Higher = more squash and sustain |
| Attack | 0–10 | How fast the compressor clamps a transient (0.5 ms → 50 ms). Low = snappy/tight, high = lets the pick attack through |
| Level | 0–10 | Output makeup gain (≈0–2×). 5 = unity with auto makeup |

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
| Drive | 0–10 | Gain into the cubic clip stage (1×–61×). More aggressive than the TS |
| Tone | 0–10 | Tilt control (bass↔treble seesaw around ~1 kHz). 0 = dark & full, 5 = flat, 10 = bright & cutting |
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

#### Pre-amp EQ  _(bypassable with Space)_

Sits **before the amp**, so it shapes the signal that the gain stage actually clips — a different job from the post-cab Parametric EQ below, which colours the final mix. Scoop the mids going in for a tighter chug, or push them for lead sustain. All three bands map 0–10 to −12 dB → 0 dB → +12 dB. Centre (5.0) is flat.

| Knob | Frequency | Type |
| ------ | ----------- | ---- |
| Low | 100 Hz | Low shelf |
| Mid | 650 Hz | Peak (Q 1.0) |
| High | 3 kHz | High shelf |

#### Parametric EQ  _(bypassable with Space)_

Post-cabinet — shapes the final stereo tone after distortion. All three bands map 0–10 to −15 dB → 0 dB → +15 dB. Centre (5.0) is unity gain.

| Knob | Frequency | Type |
| ------ | ----------- | ---- |
| Low | 120 Hz | Low shelf |
| Mid | 800 Hz | Peak (Q 1.5) |
| High | 5 kHz | High shelf |

### Amp &amp; cabinet  _(Amp | Cabinet mics)_

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

#### Cabinet mics  _(labeled with the active cabinet model)_

Three controls model a multi-mic'd 4×12 — the close mic's position, a blend from a
dynamic to a ribbon, and a room mic for depth. The blend is a weighted sum of the
three mics' impulse responses, so it costs no extra per-sample CPU.

| Knob | Range | Effect |
| ------ | ------- | -------- |
| Mic | 0–10 | Close-mic position: 0 = edge (off-axis, dark, −6 dB at 5 kHz) · 5 = centre neutral · 10 = on-axis (bright, +6 dB at 5 kHz) |
| Blend | 0–10 | Close-mic capsule: 0 = SM57 dynamic (bright, present) · 10 = R121 ribbon (darker, fuller low-mids, silky top) |
| Room | 0–10 | Amount of a distant room mic mixed in — adds air and three-dimensional depth (0 = dry close mic only) |

## How it works

Under the hood, rusty-amp runs your guitar through a full signal chain — pedals, amp, cabinet, and a stereo effects rack — sample by sample. The amp's distortion stages run at **8× oversampling** (linear-phase polyphase-FIR) for smooth, alias-free high-gain saturation, the tube amps use a real **passive FMV tone stack** with modelled **power-amp ↔ speaker interaction**, and the cabinets are rendered by **multi-mic impulse-response convolution** for three-dimensional depth.

```text
Guitar
  │
  ▼
Noise Gate  [bypassable]
  Envelope follower → gain ramp (smooth open/close to avoid clicks)
  │
  ▼
Compressor  [bypassable]
  Peak-follower detector → hard-knee gain computer (2:1–10:1) → smoothed gain + auto makeup
  │
  ▼
Fuzz  [bypassable · Big Muff style]
  DC block → 70 Hz HP → [4× OS: two cascaded asymmetric soft-clip stages] → DC block → 700 Hz mid scoop → variable tone LP
  │
  ▼
TS-808 Tube Screamer  [bypassable]
  DC block → 340 Hz HP → 720 Hz mid-peak → [4× OS: asymmetric diode soft-clip] → output coupling cap (DC block) → variable tone LP
  │
  ▼
DS-1 Distortion  [bypassable]
  DC block → 80 Hz HP → 800 Hz mid-emphasis → [4× OS: pre-clip HP → near-symmetric cubic diode clip] → post-clip HP → tilt tone → 6.5 kHz post-clip LP
  │
  ▼
Pre-amp EQ  [bypassable]
  Low shelf (100 Hz) → Mid peak (650 Hz) → High shelf (3 kHz) — each ±12 dB — shapes what the amp clips
  │
  ▼
Amp  [Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead — switchable in real time]
  8× oversampled nonlinear stages (linear-phase polyphase-FIR anti-alias) + dynamic grid-bias "bloom" for touch sensitivity
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

Each cabinet is rendered by **impulse-response convolution** rather than a plain EQ. The built-in IRs are synthesized in-code (nothing to ship or download — though you can also [load your own `.wav` IR](#cabinet-irs)): the model's voiced EQ provides the magnitude skeleton, then early reflections (comb filtering), late cabinet/room reflections, and speaker modal resonances — including a deep, long-decaying cone "thump" — add the time-domain depth of a real miked cab. Each IR runs ~23 ms (~1100 taps at 48 kHz), long enough for the late room reflections and the low cone resonance to ring out. Two slightly different left/right IRs decorrelate the stereo image for natural width.

The convolution is computed with a **partitioned-FFT (uniformly-partitioned overlap-save)** engine rather than a direct tap-by-tap loop. At ~1100 taps per channel this is the single heaviest DSP stage, and the frequency-domain approach cuts its cost several-fold while producing the exact same linear convolution — so the tone is unchanged. The only trade-off is a fixed ~2.7 ms of latency (128 samples at 48 kHz), shared equally by both channels so the stereo image stays aligned.

Each cabinet is captured by **three mics** — a close SM57 dynamic, a close R121 ribbon, and a room mic — each with its own voicing and reflection texture (the room mic carries extra pre-delay and denser late reflections for air). The **Blend** and **Room** knobs mix these captures. Because convolution is linear, the blend is just a weighted **sum of the three IRs**, recombined into the live convolver only when a knob moves — so any mic mix costs exactly two convolutions per sample, no more.

Cycle between cabinet models with `C` at any time. The cabinet state is preserved when switching amp models.

The **Mic** knob applies a high-shelf filter (±6 dB at 5 kHz) per channel after convolution, modelling the tonal difference between an on-axis and off-axis close-mic placement.

## Cabinet IRs

Beyond the three built-in cabs, rusty-amp can load your own **impulse-response `.wav` file** as the cabinet. A loaded IR replaces the multi-mic blend with a single captured response (the mono drive still passes through the same speaker cone-breakup + thermal power-compression model, so it stays alive and dynamic). Because the file is already a finished, miked capture, the **Mic / Blend / Room** knobs are inert while an external IR is active.

Press **`I`** to open the IR browser, which scans these locations for `.wav` files:

| Order | Location |
| ----- | -------- |
| 1 | The directory in the **`RUSTY_AMP_IR_DIR`** environment variable, if set |
| 2 | `./irs/` next to where you launched the app |
| 3 | `~/.config/rusty-amp/irs/` |

Each is searched a few levels deep, so packs in subfolders are found too.

| Key | Action |
| ----- | -------- |
| `↑` / `↓` | Navigate the IR list |
| `Enter` | Load the selected IR (or **Built-in cabs (no IR)** to clear it) |
| `X` | A/B between the loaded IR and the built-in cab |
| `Esc` / `I` | Close the browser |

The loaded IR's name appears in the header in place of the cabinet label (`IR: …`) while it is active. Outside the browser, **`X`** toggles the same A/B at any time. Loading and clearing take effect live — the audio stream is never interrupted (the IR is decoded and resampled off the audio thread, then swapped in on a lock-free handoff, and the displaced cab is freed off the audio thread).

On load the IR is rate-matched to your interface (windowed-sinc resampler), trimmed to ~2048 taps with a raised-cosine tail fade, DC-removed, and energy-normalised so swapping IRs doesn't jump the level. Mono files feed both channels; stereo files keep their L/R. **No IRs are bundled** — load files you are licensed to use; the app never ships or redistributes third-party captures.

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

Drop the file in `~/.config/rusty-amp/presets/` and it will appear in the preset browser the next time you open it (or save another preset to trigger a reload).

## Recording

Press **`R`** to start recording. The header switches from `○ OFF AIR` to a blinking `● ON AIR` indicator next to `POWER ON`. Press **`R`** again to stop — the file is written immediately and the saved path is shown briefly in the footer.

Recordings capture the fully-processed signal (after the entire effects chain and output limiter) as a 32-bit float **stereo** WAV at the same sample rate as your audio interface — the full multi-mic cab spread and stereo effects are preserved. Files are named `rusty-amp-<unix-timestamp>.wav` and saved to your home directory (`~/`).

## CLAP plugins

rusty-amp can host a third-party **[CLAP](https://cleveraudio.org/) effect plugin** as a stereo insert in the signal chain — placed after the cabinet/effects rack, just before the master bus. This lets you drop in an external reverb, saturator, flanger, or anything else and dial it in without leaving the terminal.

Plugin hosting is **enabled by default** (it's in the pre-built binaries too), powered by the [`clack`](https://github.com/prokopyl/clack) CLAP host bindings. If you want a minimal amp with no plugin dependencies or plugin-loading FFI, build with the feature turned off:

```bash
cargo run --release --no-default-features
```

### Installing plugins

Put the plugin's `.clap` file (on macOS a `.clap` is a bundle directory) into one of the locations rusty-amp scans on startup:

| Platform | Scanned locations |
| -------- | ----------------- |
| macOS | `~/Library/Audio/Plug-Ins/CLAP/`, `/Library/Audio/Plug-Ins/CLAP/` |
| Linux | `~/.clap/`, `/usr/lib/clap/`, `/usr/local/lib/clap/` |
| Windows | `%COMMONPROGRAMFILES%\CLAP\`, `%LOCALAPPDATA%\Programs\Common\CLAP\` |

Any directory listed in the **`CLAP_PATH`** environment variable is also searched (subdirectories included). Most plugin installers place the `.clap` in the right folder automatically.

### Loading and configuring a plugin

Press **`V`** to open the plugin browser, which (re)scans the locations above.

| Key | Action |
| ----- | -------- |
| `↑` / `↓` | Navigate the plugin list |
| `Enter` | Load the selected plugin (or **None — bypass insert** to clear it) |
| `Tab` | Switch to the parameter editor for the loaded plugin |
| `Esc` / `V` | Close the browser |

When a plugin with parameters is loaded you drop straight into the **parameter editor**:

| Key | Action |
| ----- | -------- |
| `↑` / `↓` | Select a parameter |
| `←` / `→` | Adjust the selected parameter (by 1/20 of its range) |
| `Tab` | Return to the plugin list |
| `Esc` / `V` | Close |

The loaded plugin's name appears in the header (🔌) next to the amp and cabinet. Loading, clearing, and parameter edits all take effect live — the audio stream is never interrupted (swaps happen on a lock-free handoff, and the displaced plugin is freed off the audio thread).

### Limitations

- **Headless** — plugin GUIs are not opened; parameters are edited in the TUI (shown as raw numeric values).
- **Effects only** — instrument/synth plugins are not driven (there's no MIDI input).
- **One insert slot**, using the plugin's main mono/stereo audio ports (no sidechain or multi-out routing).
- Plugin state is **not saved** in rusty-amp presets, and is not recalled across restarts.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style rules, DSP conventions, and how to add new effects, amp models, cabinets, or presets.

## License

Apache 2.0
