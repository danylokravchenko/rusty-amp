---
layout: page.njk
permalink: how-it-works.html
title: "How it works · rusty-amp"
ogTitle: "rusty-amp · how it works"
description: "Under the hood of rusty-amp: the full sample-by-sample signal chain, 8× oversampled amp stages, passive FMV tone stacks, and partitioned-FFT cabinet convolution."
eyebrow: "Under the hood"
heading: "How rusty-amp works"
lead: "Your guitar runs through a full signal chain — pedals, amp, cabinet, and a stereo effects rack — sample by sample. Here's every stage."
toc:
  - { href: "#summary", label: "The short version" }
  - { href: "#chain", label: "Full signal chain" }
  - { href: "#amp", label: "Amp stages" }
  - { href: "#cabinet", label: "Cabinet convolution" }
prev: { href: "plugins.html", label: "CLAP plugins" }
next: { href: "https://github.com/danylokravchenko/rusty-amp/blob/main/CONTRIBUTING.md", label: "Contributing on GitHub ↗" }
---

## The short version {#summary}

<div class="grid grid--3">
  <div class="card">
    <div class="ico">⚡</div>
    <h3>8× oversampled drive</h3>
    <p>The amp's distortion stages run at 8× oversampling (linear-phase polyphase-FIR) for smooth, alias-free high-gain saturation.</p>
  </div>
  <div class="card">
    <div class="ico">🎚️</div>
    <h3>Real FMV tone stack</h3>
    <p>The tube amps use a passive FMV tone stack — interacting controls and inherent mid scoop — plus modelled power-amp ↔ speaker interaction.</p>
  </div>
  <div class="card">
    <div class="ico">📐</div>
    <h3>IR convolution cabs</h3>
    <p>Cabinets are rendered by multi-mic impulse-response convolution via a partitioned-FFT engine, for three-dimensional depth.</p>
  </div>
</div>

## Full signal chain {#chain}

Every block below is processed per sample. Bracketed stages are `[bypassable]` — remove or bypass them and they drop out of the path entirely.

<pre class="chain-art"><code>Guitar
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
Cabinet  [Mesa 4×12 V30 | Marshall 4×12 Greenback | Orange PPC412 V30 — switchable in real time]
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
Per-channel output soft limiter → stereo (L, R)</code></pre>

For the per-knob behaviour of each pedal, see [Pedals & effects](pedals.html).

## The amp stages {#amp}

All three amps share an **8× oversampled** nonlinear core with a linear-phase polyphase-FIR anti-alias filter, plus a dynamic grid-bias "bloom" that makes the gain respond to how hard you play. Beyond that, each model diverges:

| Model | Character | Tone stack | Rectifier / power | Gain stages |
| ----- | --------- | ---------- | ----------------- | ----------- |
| **Marshall JCM800** | Punchy, dynamic, touch-sensitive | Passive FMV (Marshall values) | Tube sag (5 ms / 200 ms) + dynamic speaker-load bloom | 2 × 12AX7 atan soft-clip |
| **Mesa Dual Rectifier** | Compressed, aggressive, modern | Passive FMV (Fender values) | Silicon sag (0.5 ms / 80 ms) + dynamic speaker-load bloom | 3-stage: atan → atan → exponential |
| **Randall Warhead** | Tight, crushing, solid-state | Active, independent bands + fixed +3 dB presence | No sag — stiff solid-state rails + static speaker resonance | FET (x/√(1+x²)) → BJT (tanh) → rail-clip |

The **passive FMV tone stack** is a single RC network where bass, mid, and treble interact and the mids inherently scoop — exactly like a real amp — followed by a **power-amp ↔ speaker interaction** model: the speaker's impedance resonance blooms the low end dynamically as the supply sags under hard playing. The Randall keeps an active, independent-band stack and a small static speaker resonance, true to its stiff solid-state design. See [Amps & cabinets](amps-cabs.html#amp) for the per-knob breakdown.

## Cabinet convolution {#cabinet}

Each cabinet is rendered by **impulse-response convolution** rather than a plain EQ. The built-in IRs are synthesized in-code (nothing to ship or download — though you can also [load your own `.wav` IR](amps-cabs.html#irs)): the model's voiced EQ provides the magnitude skeleton, then early reflections (comb filtering), late cabinet/room reflections, and speaker modal resonances — including a deep, long-decaying cone "thump" — add the time-domain depth of a real miked cab.

Each IR runs ~23 ms (~1100 taps at 48 kHz), long enough for the late room reflections and the low cone resonance to ring out. Two slightly different left/right IRs decorrelate the stereo image for natural width.

### The convolution engine

The convolution is computed with a **partitioned-FFT (uniformly-partitioned overlap-save)** engine rather than a direct tap-by-tap loop. At ~1100 taps per channel this is the single heaviest DSP stage, and the frequency-domain approach cuts its cost several-fold while producing the exact same linear convolution — so the tone is unchanged. The only trade-off is a fixed ~2.7 ms of latency (128 samples at 48 kHz), shared equally by both channels so the stereo image stays aligned.

### Three mics, one blend

Each cabinet is captured by **three mics** — a close SM57 dynamic, a close R121 ribbon, and a room mic — each with its own voicing and reflection texture (the room mic carries extra pre-delay and denser late reflections for air). The **Blend** and **Room** knobs mix these captures. Because convolution is linear, the blend is just a weighted **sum of the three IRs**, recombined into the live convolver only when a knob moves — so any mic mix costs exactly two convolutions per sample, no more.

The **Mic** knob applies a high-shelf filter (±6 dB at 5 kHz) per channel after convolution, modelling the tonal difference between an on-axis and off-axis close-mic placement. See [cabinet models](amps-cabs.html#cabs) for the per-cab voicing.
