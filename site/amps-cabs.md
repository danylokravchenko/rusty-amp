---
layout: page.njk
permalink: amps-cabs.html
title: "Amps, cabinets & IRs · rusty-amp"
ogTitle: "rusty-amp · amps, cabinets & IRs"
description: "The three rusty-amp amp models and cabinets, the cabinet-mic controls, and how to load your own external .wav impulse responses."
eyebrow: "Amps, cabinets & IRs"
heading: "The amp head & cabinet"
lead: "Three switchable amps, three multi-mic'd 4×12 cabinets, and a loader for your own impulse responses."
toc:
  - { href: "#amp", label: "Amp knobs" }
  - { href: "#models", label: "Amp models" }
  - { href: "#mics", label: "Cabinet mics" }
  - { href: "#cabs", label: "Cabinet models" }
  - { href: "#irs", label: "External IRs" }
  - { href: "#packaging", label: "Packaging IRs" }
prev: { href: "pedals.html", label: "Pedals &amp; effects" }
next: { href: "presets.html", label: "Presets" }
---

## Amp knobs <span class="muted">(<kbd>↑</kbd>/<kbd>↓</kbd> or <kbd>A</kbd> on the selector row)</span> {#amp}

| Knob | Range | Marshall JCM800 | Mesa Dual Rectifier | Randall Warhead |
| ---- | ----- | --------------- | ------------------- | --------------- |
| Gain | 0–10 | Preamp gain 1×–40× into dual 12AX7 | Preamp gain 1×–36× into three stages | Preamp gain 1×–46× into FET+BJT stages |
| Bass | 0–10 | Passive FMV tone stack — bass/mid/treble interact like the real network (Marshall values) | Passive FMV stack (Fender-type values: fuller lows, gentler scoop) | Active tone stack — low shelf at 80 Hz |
| Mid | 0–10 | …the mid pot sets the depth of the stack's inherent scoop | …gentler scoop than the Marshall | Peak EQ at 500 Hz |
| Treble | 0–10 | …treble interacts with mid/bass, lossy & peak-normalised | …same interacting network | High shelf at 4.5 kHz |
| Presence | 0–10 | High shelf at 3.5 kHz (±6 dB) | High shelf at 4 kHz (±6 dB) | High shelf at 5 kHz (+3 dB fixed offset, ±6 dB) |
| Master | 0–10 | Post-amp output level | Post-amp output level | Post-amp output level |

The tube amps (Marshall, Mesa) drive a **passive FMV tone stack** — a single RC network where the three controls interact and the mids inherently scoop, exactly like a real amp — followed by a **power-amp ↔ speaker interaction** model: the speaker's impedance resonance blooms the low end dynamically as the supply sags under hard playing. The Randall keeps an active (independent-band) stack and a small static speaker resonance, true to its stiff solid-state design.

## Amp models {#models}

| Model | Character | Tone stack | Rectifier / power | Gain stages |
| ----- | --------- | ---------- | ----------------- | ----------- |
| **Marshall JCM800** | Punchy, dynamic, touch-sensitive | Passive FMV (Marshall values) | Tube sag (5 ms / 200 ms) + dynamic speaker-load bloom | 2 × 12AX7 atan soft-clip |
| **Mesa Dual Rectifier** | Compressed, aggressive, modern | Passive FMV (Fender values) | Silicon sag (0.5 ms / 80 ms) + dynamic speaker-load bloom | 3-stage: atan → atan → exponential |
| **Randall Warhead** | Tight, crushing, solid-state | Active, independent bands + fixed +3 dB presence | No sag — stiff solid-state rails + static speaker resonance | FET (x/√(1+x²)) → BJT (tanh) → rail-clip |

Cycle amp models with <kbd>A</kbd> at any time; the cabinet state is preserved when switching.

## Cabinet mics {#mics}

Three controls model a multi-mic'd 4×12 — the close mic's position, a blend from a dynamic to a ribbon, and a room mic for depth. The blend is a weighted sum of the three mics' impulse responses, so it costs no extra per-sample CPU.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Mic | 0–10 | Close-mic position: 0 = edge (off-axis, dark, −6 dB at 5 kHz) · 5 = centre neutral · 10 = on-axis (bright, +6 dB at 5 kHz). |
| Blend | 0–10 | Close-mic capsule: 0 = SM57 dynamic (bright, present) · 10 = R121 ribbon (darker, fuller low-mids, silky top). |
| Room | 0–10 | Amount of a distant room mic mixed in — adds air and three-dimensional depth (0 = dry close mic only). |

## Cabinet models {#cabs}

| Model | Character | Key frequencies |
| ----- | --------- | --------------- |
| **Mesa 4×12 (V30)** | Scooped, aggressive, forward-projecting | −5 dB mid scoop at 400 Hz, +7 dB presence at 3.5 kHz, hard rolloff above 6 kHz |
| **Marshall 4×12 (Greenback)** | Warm, mid-forward, smooth top end | +4 dB body at 800 Hz, +5 dB presence at 2.5 kHz, soft rolloff above 5 kHz |
| **Orange PPC412 (V30)** | Thick, chunky, mid-forward (closed-back birch) | +5 dB low-mid "wall" at 600 Hz, +3 dB grind at 1.2 kHz, +5 dB presence at 3.2 kHz |

Each cabinet is rendered by **impulse-response convolution** rather than a plain EQ. The built-in IRs are synthesized in-code (nothing to ship or download): the model's voiced EQ provides the magnitude skeleton, then early reflections (comb filtering), late cabinet/room reflections, and speaker modal resonances — including a deep, long-decaying cone "thump" — add the time-domain depth of a real miked cab. Each IR runs ~23 ms (~1100 taps at 48 kHz). Two slightly different L/R IRs decorrelate the stereo image for natural width.

Cycle cabinets with <kbd>C</kbd> at any time. The **Mic** knob applies a ±6 dB high shelf at 5 kHz per channel after convolution, modelling on-axis vs off-axis placement. For the partitioned-FFT convolution engine, see [How it works](how-it-works.html#cabinet).

## External cabinet IRs <span class="muted">(<kbd>I</kbd> · <kbd>X</kbd>)</span> {#irs}

Beyond the three built-in cabs, rusty-amp can load your own **impulse-response `.wav` file** as the cabinet. A loaded IR replaces the multi-mic blend with a single captured response (the mono drive still passes through the same speaker cone-breakup + thermal power-compression model, so it stays alive and dynamic). Because the file is already a finished, miked capture, the **Mic / Blend / Room** knobs are inert while an external IR is active.

<div class="note">
<b>One IR is a complete cabinet.</b> A speaker + cab + mic is a linear time-invariant system, and a single impulse response fully captures its linear response — exactly what a <code>.wav</code> IR is, and what every IR loader (Two Notes, Helix, NAM cab blocks, OwnHammer / God's Cab) uses: <b>one IR = one cab, one mic, one position</b>. rusty-amp adds the speaker's <em>nonlinear</em> behaviour back on top, so a loaded IR isn't static playback. A <em>multi-mic blend</em> is just a sum of IRs — pre-mix it into one file in your DAW, or load one mic at a time and A/B with <kbd>X</kbd>.
</div>

### Where it scans

Press <kbd>I</kbd> to open the IR browser, which scans these locations for `.wav` files:

| Order | Location |
| ----- | -------- |
| 1 | The directory in the `RUSTY_AMP_IR_DIR` environment variable, if set |
| 2 | `./irs/` next to where you launched the app |
| 3 | `~/.config/rusty-amp/irs/` |

Each location is searched **up to ~4 folders deep**, so a small pack with a couple of subfolders is found — but deeply-nested commercial packs sit past that limit, so the right move is to **curate a flat folder** rather than point the app at a raw download.

### IR browser keys

| Key | Action |
| --- | ------ |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate the IR list |
| <kbd>Enter</kbd> | Load the selected IR (or **Built-in cabs (no IR)** to clear it) |
| <kbd>X</kbd> | A/B between the loaded IR and the built-in cab |
| <kbd>Esc</kbd> / <kbd>I</kbd> | Close the browser |

The loaded IR's name appears in the header in place of the cabinet label (`IR: …`) while it is active. Outside the browser, <kbd>X</kbd> toggles the same A/B at any time. Loading and clearing take effect live — the audio stream is never interrupted (the IR is decoded and resampled off the audio thread, then swapped in on a lock-free handoff).

On load the IR is rate-matched to your interface (windowed-sinc resampler), trimmed to ~2048 taps with a raised-cosine tail fade, DC-removed, and energy-normalised so swapping IRs doesn't jump the level. Mono files feed both channels; stereo files keep their L/R.

<div class="note note--info">
<b>No IRs are bundled.</b> Load only files you are licensed to use — the app never ships or redistributes third-party captures.
</div>

## Packaging IR files {#packaging}

Drop `.wav` IRs into one of the scanned folders above — the simplest is `~/.config/rusty-amp/irs/` (create it if missing). The **filename (without `.wav`) becomes the label** in the browser, so name them readably (e.g. `Marshall_GB_SM57_cap.wav`).

### Accepted format

Practically any guitar IR works as-is.

| Property | Supported | Notes |
| -------- | --------- | ----- |
| Channels | Mono or stereo | Mono feeds both sides; stereo keeps L/R; extra channels are dropped |
| Sample rate | Any (44.1 / 48 / 96 kHz…) | Resampled to your interface rate on load |
| Bit depth | 16 / 24 / 32-bit int or 32-bit float | — |
| Length | Any (~20 ms to several hundred ms) | Trimmed to ~2048 taps (~43 ms @ 48 kHz) with a tail fade |

<div class="note">
<b>Curate — don't dump a whole pack.</b> Big libraries ship hundreds or thousands of files in deeply-nested folders (by sample rate → mic → processing). The browser scans only ~4 levels deep and a 2,000-entry list is unusable, so copy a handful of favourites into a <b>flat</b> <code>irs/</code> folder.
</div>

### Example: God's Cab

[God's Cab](https://wilkinsonaudio.com/products/gods-cab) is a large, free (donationware) IR pack laid out as `Gods_Cab_1.4/<rate>/<mic>/<NO-TS|TS>/<name>.wav`. Pick the sample-rate folder matching your interface, prefer the **NO-TS** captures (no Tube Screamer baked in, so rusty-amp's own TS-808 isn't stacked on top), and copy a few mics into `irs/`:

```bash
SRC=~/.config/rusty-amp/IR/Gods_Cab_1.4/48   # use 44.1 / 48 / 96 to taste
DST=~/.config/rusty-amp/irs
mkdir -p "$DST"

# A balanced cone capture per mic, renamed to clean browser labels.
cp "$SRC/SM57/NO-TS/57_1_inch_cone_near_pres_3.wav"     "$DST/GodsCab_SM57_cone.wav"
cp "$SRC/MD421/NO-TS/MD421_1_inch_cone_near_pres_3.wav" "$DST/GodsCab_MD421_cone.wav"
cp "$SRC/U87/NO-TS/U87_grill_cone_near_pres_3.wav"      "$DST/GodsCab_U87_cone.wav"
```

The capture spot moves brightest → darkest as `cap` → `cone` → `edge`; `1_inch`/`2_inch`/`grill` is mic distance; `pres_1..5` are alternate takes. Press <kbd>I</kbd> in the app and the copied files appear immediately.
