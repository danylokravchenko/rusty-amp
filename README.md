# 🎸 rusty-amp

**A complete guitar amp and pedalboard rig that runs right in your terminal.**

Plug in your guitar, pick an amp, and play. rusty-amp recreates classic tube and solid-state amplifiers, a full board of stompbox effects, and multi-mic'd 4×12 cabinets — all driven from a fast, keyboard-only interface with live metering. It ships with artist-inspired presets, so you can dial in a great tone in seconds and tweak from there.

![Screenshot](/site/assets/screenshot.png)

> 📖 **Full documentation:** **[danylokravchenko.github.io/rusty-amp](https://danylokravchenko.github.io/rusty-amp/)** — install guide, every pedal and knob, amps & cabinets, presets, plugins, and how it all works under the hood.

## Demo

![Demo](/site/assets/demo.gif)

## Motivation

Every person who plays guitar knows the routine: you need to boot up heavyweight software, load a plugin for amp and cabinet simulation, and wait. That software is great for professional studio work, but what if you just want to play your instrument without any complications? In addition, it's usually platform-locked, the licenses cost real money, and they limit how many devices you can authorize.

🎸 rusty-amp — a complete guitar amp and pedalboard rig that's a single ~1.5 MB binary. No installer. No license server. No subscription. No "activate this device". You download one tiny file, plug in your guitar, and you're playing on any platform right in your terminal.

> [!IMPORTANT]
> Rusty-amp does not try to replace professional studio software.

What's important that it doesn't cut corners on tone. Inside that tiny binary: amplifiers, multi-mic'd cabinets, and a full pedalboard, with artist-inspired presets to land a great tone. Under the hood, it's real DSP, not a toy. Each amp is modeled on the actual circuit, so the controls push and pull on each other the way they do on a real analog tone stack, and the low end "blooms" dynamically as you dig in, with the power amp and speaker interacting just like the real thing. The cabinets aren't just an EQ curve either. A real cab in a room has a "fingerprint": its size, the cone resonance, the mic, and the early reflections. rusty-amp captures all of it as an impulse response, over a thousand taps long, so every reflection and resonance rings out, and the tone comes through deep and juicy instead of flat and sterile.

On top of that sit complex effects which add width and depth to sound. All of it runs in real time on a single audio thread, and the whole rig consumes only about 15% CPU, thanks to Rust, convolution, and FFT.

## Highlights

- 🔊 **3 amplifiers** — Marshall JCM800, Mesa Dual Rectifier, and Randall Warhead, switchable while you play
- 📦 **3 cabinets + your own IRs** — Mesa, Marshall, and Orange 4×12s, each captured with three blendable mics; load your own favourite external `wav` IR
- 🎛️ **A full pedalboard** — noise gate, whammy pitch shifter, auto-wah, compressor, fuzz, Tube Screamer, DS-1, ML-2 Metal Core, graphic EQ, parametric EQ, flanger, chorus, phaser, tremolo/vibrato, ping-pong delay, and stereo reverb; add, remove, and bypass on the fly
- 🎧 **True studio-grade stereo** — wide, three-dimensional sound from the cab, delay, and reverb
- 💾 **Ready-made presets** — instant tones inspired by Metallica, Pantera, Slayer, Death, and more
- 🔌 **CLAP plugin host** — drop a third-party CLAP effect into the chain and tweak its parameters from the TUI
- 🎚️ **AU amp host (macOS)** — load an Audio Unit amp sim (e.g. a Marshall plugin) as an amp-position override, either amp+cab or amp-only (keeping the built-in cab/IR), with unit-aware params and latency-aligned
- 🎵 **Built-in tuner** — a chromatic tuner with a ±cents needle and a live note spectrum
- 🥁 **Practice metronome** — an adjustable-tempo click you hear while playing but that never bleeds into your recordings
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
| [Get started](https://danylokravchenko.github.io/rusty-amp/getting-started.html) | Install, startup flow, full controls |
| [Tools](https://danylokravchenko.github.io/rusty-amp/tools.html) | Tuner, metronome, recording |
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
