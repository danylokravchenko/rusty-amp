---
layout: page.njk
permalink: plugins.html
title: "Plugins · rusty-amp"
ogTitle: "rusty-amp · plugins"
description: "Host external plugins in rusty-amp — a third-party CLAP effect as a stereo insert, or a macOS Audio Unit amp sim as an amp-position override — without leaving the terminal."
eyebrow: "Plugins"
heading: "Host external plugins 🔌"
lead: 'Drop a third-party <a href="https://cleveraudio.org/">CLAP</a> effect into the chain — a reverb, saturator, flanger, anything — or load a macOS <b>Audio Unit</b> amp sim to replace the built-in amp &amp; cab, and dial it in without leaving the terminal.'
toc:
  - { href: "#overview", label: "CLAP: overview" }
  - { href: "#install", label: "Installing plugins" }
  - { href: "#load", label: "Loading & configuring" }
  - { href: "#limits", label: "CLAP limitations" }
  - { href: "#au", label: "Audio Unit amps (macOS)" }
  - { href: "#au-load", label: "Loading an AU amp" }
  - { href: "#au-limits", label: "AU limitations" }
prev: { href: "presets.html", label: "Presets" }
next: { href: "tools.html", label: "Tuner &amp; recording" }
---

## CLAP effects {#overview}

rusty-amp can host a third-party **CLAP effect plugin** as a stereo insert in the signal chain — placed after the cabinet/effects rack, just before the master bus.

Plugin hosting is **enabled by default** (it's in the pre-built binaries too), powered by the [clack](https://github.com/prokopyl/clack) CLAP host bindings. If you want a minimal amp with no plugin dependencies or plugin-loading FFI, build with the feature turned off:

```bash
cargo run --release --no-default-features
```

## Installing plugins {#install}

Put the plugin's `.clap` file (on macOS a `.clap` is a bundle directory) into one of the locations rusty-amp scans on startup:

| Platform | Scanned locations |
| -------- | ----------------- |
| macOS | `~/Library/Audio/Plug-Ins/CLAP/`, `/Library/Audio/Plug-Ins/CLAP/` |
| Linux | `~/.clap/`, `/usr/lib/clap/`, `/usr/local/lib/clap/` |
| Windows | `%COMMONPROGRAMFILES%\CLAP\`, `%LOCALAPPDATA%\Programs\Common\CLAP\` |

Any directory listed in the `CLAP_PATH` environment variable is also searched (subdirectories included). Most plugin installers place the `.clap` in the right folder automatically.

## Loading & configuring a plugin <span class="muted">(<kbd>V</kbd>)</span> {#load}

Press <kbd>V</kbd> to open the plugin browser, which (re)scans the locations above.

| Key | Action |
| --- | ------ |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate the plugin list |
| <kbd>Enter</kbd> | Load the selected plugin (or **None — bypass insert** to clear it) |
| <kbd>Tab</kbd> | Switch to the parameter editor for the loaded plugin |
| <kbd>Esc</kbd> / <kbd>V</kbd> | Close the browser |

When a plugin with parameters is loaded you drop straight into the **parameter editor**:

| Key | Action |
| --- | ------ |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Select a parameter |
| <kbd>←</kbd> / <kbd>→</kbd> | Adjust the selected parameter (by 1/20 of its range) |
| <kbd>Tab</kbd> | Return to the plugin list |
| <kbd>Esc</kbd> / <kbd>V</kbd> | Close |

The loaded plugin's name appears in the header (🔌) next to the amp and cabinet. Loading, clearing, and parameter edits all take effect live — the audio stream is never interrupted (swaps happen on a lock-free handoff, and the displaced plugin is freed off the audio thread).

## CLAP limitations {#limits}

<ul class="clean">
<li><b>Headless</b> — plugin GUIs are not opened; parameters are edited in the TUI (shown as raw numeric values).</li>
<li><b>Effects only</b> — instrument/synth plugins are not driven (there's no MIDI input).</li>
<li><b>One insert slot</b>, using the plugin's main mono/stereo audio ports (no sidechain or multi-out routing).</li>
<li>Plugin state is <b>not saved</b> in rusty-amp presets, and is not recalled across restarts.</li>
</ul>

## Audio Unit amps <span class="muted">(macOS)</span> {#au}

On macOS, rusty-amp can also host an **Audio Unit (AU) effect** — typically an amp-sim plugin such as a Marshall — as an **amp-position override**. This is different from the CLAP insert: rather than sitting at the end of the chain, a loaded AU *replaces the built-in amp **and** cabinet*. Your pre-amp pedal signal (gate → compressor → … → pre-EQ) is fed into the AU, and its stereo output continues through the post-cab rack (parametric EQ → flanger → chorus → delay → reverb) and the master bus. That's the correct routing for amp sims, which bring their own cabinet.

AU hosting uses the CoreAudio/AudioToolbox frameworks, so it is **macOS-only** and **enabled by default** in the macOS builds. It compiles to nothing on Linux/Windows. To build a macOS binary without it:

```bash
cargo run --release --no-default-features --features clap
```

rusty-amp lists every effect Audio Unit registered with the system — both **Effect** (`aufx`) and **Music Effect** (`aumf`) component types, the same units your DAW sees. Installing an AU is handled by its own installer (into `/Library/Audio/Plug-Ins/Components/` or the per-user equivalent under `~/Library/…`); rusty-amp discovers them automatically on open — there's no folder to manage.

## Loading & configuring an AU amp <span class="muted">(<kbd>U</kbd>)</span> {#au-load}

Press <kbd>U</kbd> to open the AU amp browser, which enumerates the installed Audio Units.

| Key | Action |
| --- | ------ |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate the AU list |
| <kbd>Enter</kbd> | Load the selected AU (or **None — use built-in amp** to clear it) |
| <kbd>Tab</kbd> | Switch to the parameter editor for the loaded AU |
| <kbd>Esc</kbd> / <kbd>U</kbd> | Close the browser |

When an AU with parameters is loaded you drop straight into the **parameter editor** (identical controls to the CLAP editor):

| Key | Action |
| --- | ------ |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Select a parameter |
| <kbd>←</kbd> / <kbd>→</kbd> | Adjust the selected parameter (by 1/20 of its range) |
| <kbd>Tab</kbd> | Return to the AU list |
| <kbd>Esc</kbd> / <kbd>U</kbd> | Close |

Loading an AU makes it the **active amp immediately**: the built-in amp and cabinet are bypassed, the header shows `AU: NAME` in place of the amp model, and the built-in tone-stack knobs and AMP selector dim to signal they no longer affect the sound (exactly as the mic knobs dim under an external IR). Press <kbd>Z</kbd> to A/B between the loaded AU and the built-in amp live — no reload.

## AU limitations {#au-limits}

<ul class="clean">
<li><b>macOS only</b> — the feature is inert on Linux/Windows.</li>
<li><b>Assumes an amp + cab AU</b> — while an AU is active the built-in cabinet (and any loaded IR) is bypassed. Amp-only AUs that expect a separate cab aren't paired with the built-in cab yet.</li>
<li><b>Headless</b> — the AU's GUI is not opened; parameters are edited in the TUI as raw numeric values.</li>
<li><b>Effect components only</b> (<code>aufx</code> / <code>aumf</code>) using the main stereo (or mono) ports; instruments aren't driven.</li>
<li>Reported plugin <b>latency is not compensated</b>.</li>
<li>AU state is <b>not saved</b> in rusty-amp presets, and is not recalled across restarts.</li>
</ul>
