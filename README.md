# 🎸 rusty-amp

**A complete guitar amp and pedalboard rig that runs right in your terminal.**

Plug in your guitar, pick an amp, and play. rusty-amp recreates classic tube and solid-state amplifiers, a full board of stompbox effects, and multi-mic'd 4×12 cabinets — all driven from a fast, keyboard-only interface with live metering. It ships with artist-inspired presets, so you can dial in a great tone in seconds and tweak from there.

![Screenshot](/site/assets/screenshot.png)

> 📖 **Full documentation:** **[danylokravchenko.github.io/rusty-amp](https://danylokravchenko.github.io/rusty-amp/)** — install guide, every pedal and knob, amps & cabinets, presets, plugins, and how it all works under the hood.

## Highlights

- 🔊 **3 amplifiers** — Marshall JCM800, Mesa Dual Rectifier, and Randall Warhead, switchable while you play
- 📦 **3 cabinets + your own IRs** — Mesa, Marshall, and Orange 4×12s, each captured with three blendable mics; load your own `.wav` IR and A/B it live
- 🎛️ **A full pedalboard** — noise gate, compressor, fuzz, Tube Screamer, DS-1, EQ, flanger, chorus, ping-pong delay, and stereo reverb; add, remove, and bypass on the fly
- 🎧 **True studio-grade stereo** — wide, three-dimensional sound from the cab, delay, and reverb
- 💾 **Ready-made presets** — instant tones inspired by Metallica, Pantera, Slayer, Death, and more
- 🔌 **CLAP plugin host** — drop a third-party CLAP effect into the chain and tweak its parameters from the TUI
- 🎚️ **AU amp host (macOS)** — load an Audio Unit amp sim (e.g. a Marshall plugin) as an amp-position override, either amp+cab or amp-only (keeping the built-in cab/IR), with unit-aware params and latency-aligned A/B
- 🎵 **Built-in tuner** — a chromatic tuner with a ±cents needle and a live note spectrum
- ⏺️ **One-key recording** straight to a stereo WAV file
- 🖥️ **Cross-platform** — runs on macOS, Windows, and Linux

## Install

Pre-built binaries have presets baked in — there's nothing else to download. Grab the latest from [Releases](https://github.com/danylokravchenko/rusty-amp/releases/latest).

**macOS (Apple Silicon):**

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-macos-aarch64 -o rusty-amp
chmod +x rusty-amp
xattr -d com.apple.quarantine rusty-amp   # clear the unsigned-binary quarantine flag
./rusty-amp
```

**Linux (x86_64):**

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-linux-x86_64 -o rusty-amp
chmod +x rusty-amp
./rusty-amp
```

**Windows / build from source** (requires Rust 1.95+):

```bash
cargo run --release
```

See the [getting-started guide](https://danylokravchenko.github.io/rusty-amp/getting-started.html) for the startup flow, device selection, and the full keyboard reference.

## What you need

- **A guitar and an audio interface** with a high-impedance instrument input (e.g. Focusrite Scarlett)
- **Speakers or headphones** — for the full stereo image, use stereo output

## Documentation

The complete docs live at **[danylokravchenko.github.io/rusty-amp](https://danylokravchenko.github.io/rusty-amp/)**:

| Topic | What's there |
| ----- | ------------ |
| [Get started](https://danylokravchenko.github.io/rusty-amp/getting-started.html) | Install, startup flow, full controls, tuner, recording |
| [Pedals & effects](https://danylokravchenko.github.io/rusty-amp/pedals.html) | Every pedal and knob, with reference tables |
| [Amps & cabinets](https://danylokravchenko.github.io/rusty-amp/amps-cabs.html) | Amp models, cabinet mics, loading external `.wav` IRs |
| [Presets](https://danylokravchenko.github.io/rusty-amp/presets.html) | Browser, save dialog, bundled tones, the TOML format |
| [CLAP plugins](https://danylokravchenko.github.io/rusty-amp/plugins.html) | Installing, loading, and configuring plugins |
| [AU amp plugins](https://danylokravchenko.github.io/rusty-amp/plugins.html) | Loading a macOS Audio Unit amp sim as an amp override |
| [How it works](https://danylokravchenko.github.io/rusty-amp/how-it-works.html) | The full DSP signal chain, under the hood |

The site source lives in [`site/`](site/) and is published to GitHub Pages automatically — see [`site/README.md`](site/README.md) for how to edit and preview it.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style rules, DSP conventions, and how to add new effects, amp models, cabinets, or presets.

## License

Apache 2.0
