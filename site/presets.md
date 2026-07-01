---
layout: page.njk
permalink: presets.html
title: "Presets · rusty-amp"
ogTitle: "rusty-amp · presets"
description: "Load, save, and write rusty-amp presets — including the bundled artist-inspired tones and the full TOML preset format."
eyebrow: "Presets"
heading: "Save, load & write tones"
lead: "Presets are plain <code>.toml</code> files. Load one while playing, save your own without restarting, or hand-write a preset from scratch."
toc:
  - { href: "#where", label: "Where they live" }
  - { href: "#browser", label: "Preset browser" }
  - { href: "#save", label: "Save dialog" }
  - { href: "#bundled", label: "Bundled presets" }
  - { href: "#write", label: "Writing your own" }
prev: { href: "amps-cabs.html", label: "Amps, cabinets &amp; IRs" }
next: { href: "plugins.html", label: "CLAP plugins" }
---

## Where presets live {#where}

rusty-amp searches these directories, in order:

<ol class="steps">
<li><code>./presets/</code> — bundled presets (read-only, shipped with the repo).</li>
<li><code>~/.config/rusty-amp/presets/</code> — your personal presets.</li>
</ol>

Press <kbd>P</kbd> while playing to open the preset browser. Press <kbd>S</kbd> (from anywhere) to save the current state as a new user preset. The browser updates instantly — no restart required.

Bundled presets are marked as system presets and cannot be deleted from within the app. User presets show a `[user]` tag and can be deleted with <kbd>D</kbd>.

## Preset browser <span class="muted">(<kbd>P</kbd>)</span> {#browser}

Press <kbd>P</kbd> while playing to open the browser overlay. Move the cursor with <kbd>↑</kbd>/<kbd>↓</kbd> and press <kbd>Enter</kbd> to apply — the change takes effect immediately, with the audio uninterrupted.

<div class="overlay">
  <div class="overlay__bar"><span class="dots"><i></i><i></i><i></i></span><span class="ttl">Presets</span></div>
  <div class="plist">
    <div class="prow"><span class="prow__name">metallica</span><span class="prow__meta">Marshall JCM800 · Marshall Greenback</span></div>
    <div class="prow is-sel"><span class="prow__name">pantera</span><span class="prow__meta">Randall Warhead · Mesa V30</span></div>
    <div class="prow"><span class="prow__name">slipknot</span><span class="prow__meta">Mesa Dual Rectifier · Mesa V30</span></div>
    <div class="prow"><span class="prow__name">my lead tone</span><span class="prow__meta">Mesa Dual Rectifier · Mesa V30</span><span class="prow__tag">user</span></div>
  </div>
  <div class="overlay__foot">
    <span><kbd>↑</kbd>/<kbd>↓</kbd> navigate</span>
    <span><kbd>Enter</kbd> apply</span>
    <span><kbd>S</kbd> save</span>
    <span><kbd>D</kbd> delete</span>
    <span><kbd>Esc</kbd> close</span>
  </div>
</div>

<div class="keymap">
  <div><kbd>↑</kbd> / <kbd>↓</kbd> Navigate the preset list</div>
  <div><kbd>Enter</kbd> Apply the selected preset (audio uninterrupted)</div>
  <div><kbd>S</kbd> Open the save dialog for the current state</div>
  <div><kbd>D</kbd> Delete the selected preset (user presets only)</div>
  <div><kbd>Esc</kbd> / <kbd>P</kbd> Close without changing anything</div>
</div>

User presets are marked with a `[user]` tag in the list. The <kbd>D</kbd> hint appears in the footer only when the cursor is on a deletable preset.

## Save dialog <span class="muted">(<kbd>S</kbd>)</span> {#save}

Press <kbd>S</kbd> from anywhere to capture the current rig as a new preset. <kbd>Tab</kbd> moves between the two fields; <kbd>Enter</kbd> writes the file.

<div class="overlay overlay--narrow">
  <div class="overlay__bar"><span class="dots"><i></i><i></i><i></i></span><span class="ttl">Save preset</span></div>
  <div class="dform">
    <div class="field field--focus">
      <div class="field__label">Name</div>
      <div class="field__box">My Lead Tone<span class="caret"></span></div>
    </div>
    <div class="field">
      <div class="field__label">Description</div>
      <div class="field__box"><span class="field__ph">Sustain-focused, delay + reverb…</span></div>
    </div>
  </div>
  <div class="overlay__foot">
    <span><kbd>Tab</kbd> switch field</span>
    <span><kbd>Enter</kbd> save</span>
    <span><kbd>Esc</kbd> cancel</span>
  </div>
</div>

<div class="keymap">
  <div><kbd>Tab</kbd> Switch between Name and Description</div>
  <div><kbd>Enter</kbd> Save the preset and return to the browser</div>
  <div><kbd>Esc</kbd> Cancel without saving</div>
</div>

The preset is written to `~/.config/rusty-amp/presets/<name>.toml` and appears in the browser immediately — no restart required.

## Bundled presets {#bundled}

| File | Amp | Cabinet | Description |
| ---- | --- | ------- | ----------- |
| `metallica.toml` | Marshall JCM800 | Marshall Greenback | Hetfield's rhythm tone — TS clean boost, scooped mids, bone dry |
| `pantera.toml` | Randall Warhead | Mesa V30 | Dimebag's rhythm tone — DS-1, deep mid-scoop, Furman PQ-3 EQ |
| `pantera_floods.toml` | Randall Warhead | Mesa V30 | Floods solo — DS-1 light, open mids, delay + reverb |
| `slipknot.toml` | Mesa Dual Rectifier | Mesa V30 | Mick Thomson / Jim Root — TS boost, modern EQ scoop, full saturation |
| `death.toml` | Mesa Dual Rectifier | Mesa V30 | Chuck Schuldiner — TS boost, mids-up for note clarity |
| `slayer.toml` | Marshall JCM800 | Marshall Greenback | Hanneman & King's thrash assault — cranked JCM800, extreme mid-scoop, zero mercy |
| `metalcore_shred.toml` | Mesa Dual Rectifier | Mesa V30 | Modern metalcore shred — TS tight boost, djent-adjacent EQ, slapback delay |
| `solo_seeker.toml` | Mesa Dual Rectifier | Mesa V30 | Lead tone — sustain-focused, delay + reverb, on-axis mic for pick-attack clarity |
| `clean_melodic.toml` | Marshall JCM800 | Marshall Greenback | Warm glassy clean — gentle compression, edge-of-breakup gain, delay + hall reverb |
| `disturbed.toml` | Mesa Dual Rectifier | Mesa V30 | Dan Donegan's nu-metal chug — TS boost, scooped-but-punchy grind, bone-dry rhythm |

## Writing your own preset {#write}

A preset is a TOML file. Every section except `[tube_screamer]`, `[amp]`, and `[reverb]` is optional — omitting a section leaves that effect's current state unchanged. All knob values are normalised `0.0–1.0`.

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

# Omit [flanger] entirely to leave it off (the default for the bundled presets),
# or include it with enabled = false to store values but keep it bypassed.
[flanger]
enabled  = false      # optional, defaults to true when the section is present
rate     = 0.30       # 0.0 – 1.0  (LFO speed, 0.05–5 Hz, exponential)
depth    = 0.55       # 0.0 – 1.0  (sweep width; delay swings ~0.5–5 ms)
feedback = 0.35       # 0.0 – 1.0  (regeneration, capped at 90%)
mix      = 0.50       # 0.0 = dry, 0.5 = deepest flange, 1.0 = fully wet

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

<div class="note">
Drop the file in <code>~/.config/rusty-amp/presets/</code> and it will appear in the preset browser the next time you open it (or save another preset to trigger a reload).
</div>
