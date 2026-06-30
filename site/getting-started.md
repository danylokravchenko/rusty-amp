---
layout: page.njk
permalink: getting-started.html
title: "Get started · rusty-amp"
ogTitle: "rusty-amp · get started"
description: "Install rusty-amp from a pre-built binary or build from source, then learn the keyboard controls and startup flow."
eyebrow: "Get started"
heading: "Install, launch & play"
lead: "Everything you need to get a guitar signal running through rusty-amp — and the full keyboard map once you're in."
toc:
  - { href: "#need", label: "What you need" }
  - { href: "#install", label: "Install" }
  - { href: "#build", label: "Build from source" }
  - { href: "#startup", label: "Startup flow" }
  - { href: "#controls", label: "Controls" }
prev: { href: "index.html", label: "Home" }
next: { href: "pedals.html", label: "Pedals &amp; effects reference" }
---

## What you need {#need}

- **A guitar and an audio interface** with a high-impedance instrument input (e.g. Focusrite Scarlett).
- **Speakers or headphones** — for the full stereo image, use stereo output.

## Install {#install}

The fastest way to start is a pre-built binary — presets are baked in, so there's nothing else to download. Grab the latest from [Releases](https://github.com/danylokravchenko/rusty-amp/releases/latest), or [build from source](#build).

### macOS (Apple Silicon)

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-macos-aarch64 -o rusty-amp
chmod +x rusty-amp

# Remove the macOS quarantine flag (required for unsigned binaries)
xattr -d com.apple.quarantine rusty-amp

./rusty-amp
```

### Linux (x86_64)

```bash
curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-linux-x86_64 -o rusty-amp
chmod +x rusty-amp

./rusty-amp
```

<div class="note">
<b>Windows:</b> no pre-built binary is published yet — <a href="#build">build from source</a> instead.
rusty-amp runs natively on all three platforms via <a href="https://github.com/RustAudio/cpal">cpal</a>,
which selects the OS audio backend automatically (CoreAudio, WASAPI/ASIO, or ALSA/JACK).
</div>

## Build from source {#build}

Runs on **macOS, Windows, and Linux**. Requires **Rust 1.95+** (`rustup` recommended).

```bash
cargo run --release
# or after building:
./target/release/rusty-amp
```

CLAP plugin hosting is on by default. For a minimal amp with no plugin dependencies, build with `cargo run --release --no-default-features` — see [Plugins](plugins.html).

## Startup flow {#startup}

<ol class="steps">
<li><b>Select input device</b> — your audio interface appears in the list.</li>
<li><b>Select guitar input channel</b> — a Focusrite 2i2 has 2; guitar is usually channel 2 if plugged into Input 2.</li>
<li><b>Select output device</b> — pick your speakers or headphones.</li>
</ol>

The processed signal is **true stereo**: the left channel goes to output 0, the right to output 1 (a mono output device receives the summed mix). On a stereo interface or headphones you hear the full multi-mic cab spread, ping-pong delay, and stereo reverb.

<div class="note note--info">
The app launches immediately with default values. Press <kbd>P</kbd> at any time to open the preset browser,
<kbd>S</kbd> to save the current state as a new preset, and <kbd>R</kbd> to start or stop recording.
</div>

## Controls {#controls}

| Key | Action |
| --- | ------ |
| <kbd>Tab</kbd> / <kbd>Shift-Tab</kbd> | Jump between sections |
| <kbd>←</kbd> / <kbd>→</kbd> | Move focus one knob at a time within the focused section |
| <kbd>↑</kbd> / <kbd>+</kbd> / <kbd>=</kbd> | Increase focused knob by 5% — or cycle amp model forward on the selector row |
| <kbd>↓</kbd> / <kbd>-</kbd> | Decrease focused knob by 5% — or cycle amp model backward on the selector row |
| <kbd>A</kbd> | Cycle amp model forward (works from any section) |
| <kbd>C</kbd> | Cycle cabinet model (Mesa V30 → Marshall Greenback → Orange PPC412) |
| <kbd>I</kbd> | Open the cabinet-IR browser to load/clear an external `.wav` IR ([see IRs](amps-cabs.html#irs)) |
| <kbd>X</kbd> | A/B between a loaded external IR and the built-in cab (once an IR is loaded) |
| <kbd>Space</kbd> | Toggle (bypass) the focused pedal — or open the **Add pedal** picker on the `+ ADD` tile |
| <kbd>Enter</kbd> | Open the **Add pedal** picker when the `+ ADD` tile is focused |
| <kbd>D</kbd> | Remove the focused pedal from the board (bypassed and hidden — re-add it from `+ ADD`) |
| <kbd>P</kbd> | Open the preset browser overlay |
| <kbd>T</kbd> | Open the [tuner](tools.html#tuner) — bypasses the rig for a clean signal |
| <kbd>V</kbd> | Open the CLAP [plugin browser](plugins.html) |
| <kbd>S</kbd> | Save the current state as a new user preset |
| <kbd>R</kbd> | Start / stop [recording](tools.html#recording) — saves a WAV file to your home directory when stopped |
| <kbd>Q</kbd> / <kbd>Ctrl-C</kbd> | Quit |

Focus starts on the **selector row** (amp + cabinet). <kbd>Tab</kbd> moves down through the amp, cabinet mics, each pedal on the board, and finally the `+ ADD` tile. See [the pedalboard](pedals.html#board) for add/remove details, and [Presets](presets.html) for the browser and save dialog.

<div class="note note--info">
Looking for the <b>chromatic tuner</b> (<kbd>T</kbd>) or <b>one-key recording</b> (<kbd>R</kbd>)? They have their own page — see <a href="tools.html">Tuner &amp; recording</a>.
</div>
