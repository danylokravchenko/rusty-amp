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
  - { href: "#amp", label: "Amplifiers" }
  - { href: "#external-amp", label: "External amp plugins" }
  - { href: "#mics", label: "Cabinet mics" }
  - { href: "#cabs", label: "Cabinets" }
  - { href: "#irs", label: "External IRs" }
  - { href: "#packaging", label: "Packaging IRs" }
prev: { href: "pedals.html", label: "Pedals &amp; effects" }
next: { href: "presets.html", label: "Presets" }
---

## Amplifiers {#amp}

Three amp models, switchable live with <kbd>A</kbd> (or <kbd>↑</kbd>/<kbd>↓</kbd> on the selector row) — the cabinet state is preserved when you switch. Pick one to see its voicing and per-knob behaviour.

<div class="selector" style="--c:var(--rust)" data-tabs>
  <div class="tiles" role="tablist" aria-label="Amplifier models">
    <button class="tile is-active" role="tab" aria-selected="true" data-tab="jcm800">
      <div class="tile__name">Marshall JCM800</div>
      <div class="tile__sub">Punchy · dynamic · touch-sensitive</div>
      <div class="tile__amp"><i style="--r:-50deg"></i><i style="--r:-20deg"></i><i style="--r:10deg"></i><i style="--r:30deg"></i><i style="--r:55deg"></i><i style="--r:80deg"></i></div>
    </button>
    <button class="tile" role="tab" aria-selected="false" data-tab="mesa">
      <div class="tile__name">Mesa Dual Rectifier</div>
      <div class="tile__sub">Compressed · aggressive · modern</div>
      <div class="tile__amp"><i style="--r:-40deg"></i><i style="--r:0deg"></i><i style="--r:20deg"></i><i style="--r:45deg"></i><i style="--r:60deg"></i><i style="--r:85deg"></i></div>
    </button>
    <button class="tile" role="tab" aria-selected="false" data-tab="randall">
      <div class="tile__name">Randall Warhead</div>
      <div class="tile__sub">Tight · crushing · solid-state</div>
      <div class="tile__amp"><i style="--r:-30deg"></i><i style="--r:-5deg"></i><i style="--r:25deg"></i><i style="--r:50deg"></i><i style="--r:70deg"></i><i style="--r:90deg"></i></div>
    </button>
  </div>

  <div class="tab-panel is-active" role="tabpanel" data-panel="jcm800">
    <div class="specs">
      <div class="spec"><div class="spec__k">Gain range</div><div class="spec__v">1×–40× into dual 12AX7</div></div>
      <div class="spec"><div class="spec__k">Tone stack</div><div class="spec__v">Passive FMV (Marshall values)</div></div>
      <div class="spec"><div class="spec__k">Rectifier &amp; power</div><div class="spec__v">Tube sag (5 ms / 200 ms) + dynamic speaker-load bloom</div></div>
      <div class="spec"><div class="spec__k">Gain stages</div><div class="spec__v">2 × 12AX7 atan soft-clip</div></div>
    </div>
    <div class="kv"><span class="kv__k">Gain</span><span class="kv__v">Preamp gain 1×–40× into dual 12AX7.</span></div>
    <div class="kv"><span class="kv__k">Bass</span><span class="kv__v">Passive FMV tone stack — bass/mid/treble interact like the real network (Marshall values).</span></div>
    <div class="kv"><span class="kv__k">Mid</span><span class="kv__v">Sets the depth of the stack's inherent scoop.</span></div>
    <div class="kv"><span class="kv__k">Treble</span><span class="kv__v">Interacts with mid/bass, lossy &amp; peak-normalised.</span></div>
    <div class="kv"><span class="kv__k">Presence</span><span class="kv__v">High shelf at 3.5 kHz (±6 dB).</span></div>
    <div class="kv"><span class="kv__k">Master</span><span class="kv__v">Post-amp output level.</span></div>
  </div>

  <div class="tab-panel" role="tabpanel" data-panel="mesa">
    <div class="specs">
      <div class="spec"><div class="spec__k">Gain range</div><div class="spec__v">1×–36× into three stages</div></div>
      <div class="spec"><div class="spec__k">Tone stack</div><div class="spec__v">Passive FMV (Fender values)</div></div>
      <div class="spec"><div class="spec__k">Rectifier &amp; power</div><div class="spec__v">Silicon sag (0.5 ms / 80 ms) + dynamic speaker-load bloom</div></div>
      <div class="spec"><div class="spec__k">Gain stages</div><div class="spec__v">3-stage: atan → atan → exponential</div></div>
    </div>
    <div class="kv"><span class="kv__k">Gain</span><span class="kv__v">Preamp gain 1×–36× into three stages.</span></div>
    <div class="kv"><span class="kv__k">Bass</span><span class="kv__v">Passive FMV stack (Fender-type values: fuller lows, gentler scoop).</span></div>
    <div class="kv"><span class="kv__k">Mid</span><span class="kv__v">Gentler scoop than the Marshall.</span></div>
    <div class="kv"><span class="kv__k">Treble</span><span class="kv__v">Same interacting network.</span></div>
    <div class="kv"><span class="kv__k">Presence</span><span class="kv__v">High shelf at 4 kHz (±6 dB).</span></div>
    <div class="kv"><span class="kv__k">Master</span><span class="kv__v">Post-amp output level.</span></div>
  </div>

  <div class="tab-panel" role="tabpanel" data-panel="randall">
    <div class="specs">
      <div class="spec"><div class="spec__k">Gain range</div><div class="spec__v">1×–46× into FET+BJT stages</div></div>
      <div class="spec"><div class="spec__k">Tone stack</div><div class="spec__v">Active, independent bands + fixed +3 dB presence</div></div>
      <div class="spec"><div class="spec__k">Rectifier &amp; power</div><div class="spec__v">No sag — stiff solid-state rails + static speaker resonance</div></div>
      <div class="spec"><div class="spec__k">Gain stages</div><div class="spec__v">FET (x/√(1+x²)) → BJT (tanh) → rail-clip</div></div>
    </div>
    <div class="kv"><span class="kv__k">Gain</span><span class="kv__v">Preamp gain 1×–46× into FET+BJT stages.</span></div>
    <div class="kv"><span class="kv__k">Bass</span><span class="kv__v">Active tone stack — low shelf at 80 Hz.</span></div>
    <div class="kv"><span class="kv__k">Mid</span><span class="kv__v">Peak EQ at 500 Hz.</span></div>
    <div class="kv"><span class="kv__k">Treble</span><span class="kv__v">High shelf at 4.5 kHz.</span></div>
    <div class="kv"><span class="kv__k">Presence</span><span class="kv__v">High shelf at 5 kHz (+3 dB fixed offset, ±6 dB).</span></div>
    <div class="kv"><span class="kv__k">Master</span><span class="kv__v">Post-amp output level.</span></div>
  </div>
</div>

The tube amps (Marshall, Mesa) drive a **passive FMV tone stack** — a single RC network where the three controls interact and the mids inherently scoop, exactly like a real amp — followed by a **power-amp ↔ speaker interaction** model: the speaker's impedance resonance blooms the low end dynamically as the supply sags under hard playing. The Randall keeps an active (independent-band) stack and a small static speaker resonance, true to its stiff solid-state design.

## External amp plugins <span class="muted">(macOS · <kbd>U</kbd> · <kbd>Z</kbd>)</span> {#external-amp}

Beyond the three built-in amps, on macOS you can load a third-party **Audio Unit amp sim** — for example a Marshall plugin — and use it *in place of* the built-in amp. Press <kbd>U</kbd> to browse the Audio Units installed on your system and load one.

A loaded AU is an **amp-position override**: your pedal chain feeds into it, and because an amp-sim AU brings its own cabinet it replaces the built-in amp **and** cabinet in one go — the built-in cab and any loaded IR are bypassed while it's active. The header shows <code>AU: …</code> in place of the amp model, the tone-stack knobs and the AMP selector dim (they no longer shape the sound), and <kbd>Z</kbd> A/Bs the AU against the built-in amp live — no reload.

<div class="note note--info">
See <a href="plugins.html#au">Plugins → Audio Unit amps</a> for the full walkthrough: installation, the browser and parameter-editor keys, and limitations. AU hosting is macOS-only and compiles to nothing on Linux/Windows.
</div>

## Cabinet mics {#mics}

Three controls model a multi-mic'd 4×12 — the close mic's position, a blend from a dynamic to a ribbon, and a room mic for depth. The blend is a weighted sum of the three mics' impulse responses, so it costs no extra per-sample CPU.

<div class="miccards">
  <div class="miccard">
    <div class="miccard__top">
      <div class="knob__dial" style="--r:0deg"></div>
      <div class="miccard__id"><div class="miccard__name">Mic</div><div class="miccard__range">Close-mic position · 0–10</div></div>
    </div>
    <div class="miccard__bar"><span class="miccard__mark" style="--at:50%"></span></div>
    <div class="miccard__ends"><span>Edge · dark</span><span>On-axis · bright</span></div>
    <div class="miccard__def">Default <b>5.0</b> · centre neutral</div>
    <div class="miccard__desc">0 = edge (off-axis, dark, −6 dB at 5 kHz) · 5 = centre neutral · 10 = on-axis (bright, +6 dB at 5 kHz).</div>
  </div>

  <div class="miccard">
    <div class="miccard__top">
      <div class="knob__dial" style="--r:-95deg"></div>
      <div class="miccard__id"><div class="miccard__name">Blend</div><div class="miccard__range">Close-mic capsule · 0–10</div></div>
    </div>
    <div class="miccard__bar"><span class="miccard__mark" style="--at:15%"></span></div>
    <div class="miccard__ends"><span>SM57 dynamic</span><span>R121 ribbon</span></div>
    <div class="miccard__def">Default <b>1.5</b> · mostly SM57</div>
    <div class="miccard__desc">0 = SM57 dynamic (bright, present) · 10 = R121 ribbon (darker, fuller low-mids, silky top).</div>
  </div>

  <div class="miccard">
    <div class="miccard__top">
      <div class="knob__dial" style="--r:-95deg"></div>
      <div class="miccard__id"><div class="miccard__name">Room</div><div class="miccard__range">Room-mic amount · 0–10</div></div>
    </div>
    <div class="miccard__bar"><span class="miccard__mark" style="--at:15%"></span></div>
    <div class="miccard__ends"><span>Dry close mic</span><span>Full room</span></div>
    <div class="miccard__def">Default <b>1.5</b> · a touch of room</div>
    <div class="miccard__desc">Amount of a distant room mic mixed in — adds air and three-dimensional depth (0 = dry close mic only).</div>
  </div>
</div>

## Cabinets {#cabs}

Three multi-mic'd 4×12s, switchable live with <kbd>C</kbd>. Pick one to see its character and voiced frequency bands.

<div class="selector" style="--c:var(--teal)" data-tabs>
  <div class="tiles" role="tablist" aria-label="Cabinet models">
    <button class="tile is-active" role="tab" aria-selected="true" data-tab="cab-mesa">
      <div class="tile__name">Mesa 4×12 (V30)</div>
      <div class="tile__sub">Scooped · aggressive · forward</div>
      <div class="tile__cab"></div>
    </button>
    <button class="tile" role="tab" aria-selected="false" data-tab="cab-marshall">
      <div class="tile__name">Marshall 4×12 (Greenback)</div>
      <div class="tile__sub">Warm · mid-forward · smooth top</div>
      <div class="tile__cab"></div>
    </button>
    <button class="tile" role="tab" aria-selected="false" data-tab="cab-orange">
      <div class="tile__name">Orange PPC412 (V30)</div>
      <div class="tile__sub">Thick · chunky · closed-back birch</div>
      <div class="tile__cab"></div>
    </button>
  </div>

  <div class="tab-panel is-active" role="tabpanel" data-panel="cab-mesa">
    <p class="muted" style="margin:2px 0 4px">Scooped, aggressive, and forward-projecting — the modern high-gain reference.</p>
    <div class="freqs">
      <span class="freq"><b>−5 dB</b> @ 400 Hz · mid scoop</span>
      <span class="freq"><b>+7 dB</b> @ 3.5 kHz · presence</span>
      <span class="freq"><b>hard rolloff</b> above 6 kHz</span>
    </div>
  </div>
  <div class="tab-panel" role="tabpanel" data-panel="cab-marshall">
    <p class="muted" style="margin:2px 0 4px">Warm and mid-forward with a smooth top end — the classic rock voice.</p>
    <div class="freqs">
      <span class="freq"><b>+4 dB</b> @ 800 Hz · body</span>
      <span class="freq"><b>+5 dB</b> @ 2.5 kHz · presence</span>
      <span class="freq"><b>soft rolloff</b> above 5 kHz</span>
    </div>
  </div>
  <div class="tab-panel" role="tabpanel" data-panel="cab-orange">
    <p class="muted" style="margin:2px 0 4px">Thick and chunky from the closed-back birch enclosure — a wall of low-mids.</p>
    <div class="freqs">
      <span class="freq"><b>+5 dB</b> @ 600 Hz · low-mid “wall”</span>
      <span class="freq"><b>+3 dB</b> @ 1.2 kHz · grind</span>
      <span class="freq"><b>+5 dB</b> @ 3.2 kHz · presence</span>
    </div>
  </div>
</div>

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
