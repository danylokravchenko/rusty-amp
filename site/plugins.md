---
layout: page.njk
permalink: plugins.html
title: "CLAP plugins · rusty-amp"
ogTitle: "rusty-amp · CLAP plugins"
description: "Host a third-party CLAP effect plugin as a stereo insert in rusty-amp — install, load, and configure plugins without leaving the terminal."
eyebrow: "CLAP plugins"
heading: "Host external effects 🔌"
lead: 'Drop a third-party <a href="https://cleveraudio.org/">CLAP</a> effect into the chain — a reverb, saturator, flanger, anything — and dial it in without leaving the terminal.'
toc:
  - { href: "#overview", label: "Overview" }
  - { href: "#install", label: "Installing plugins" }
  - { href: "#load", label: "Loading & configuring" }
  - { href: "#limits", label: "Limitations" }
prev: { href: "presets.html", label: "Presets" }
next: { href: "tools.html", label: "Tuner &amp; recording" }
---

## Overview {#overview}

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

## Limitations {#limits}

<ul class="clean">
<li><b>Headless</b> — plugin GUIs are not opened; parameters are edited in the TUI (shown as raw numeric values).</li>
<li><b>Effects only</b> — instrument/synth plugins are not driven (there's no MIDI input).</li>
<li><b>One insert slot</b>, using the plugin's main mono/stereo audio ports (no sidechain or multi-out routing).</li>
<li>Plugin state is <b>not saved</b> in rusty-amp presets, and is not recalled across restarts.</li>
</ul>
