---
layout: page.njk
permalink: pedals.html
title: "Pedals & effects · rusty-amp"
ogTitle: "rusty-amp · pedals & effects"
description: "Every rusty-amp pedal explained: noise gate, compressor, fuzz, TS-808, DS-1, pre-amp EQ, parametric EQ, delay, and stereo reverb — with full knob reference tables."
eyebrow: "Pedals & effects"
heading: "Configuring the board"
lead: "Nine effects you can add, remove, and bypass independently. Each knob runs 0–10; here's exactly what every one does."
toc:
  - { href: "#board", label: "The board" }
  - { href: "#gate", label: "Noise Gate" }
  - { href: "#comp", label: "Compressor" }
  - { href: "#fuzz", label: "Fuzz" }
  - { href: "#ts", label: "TS-808" }
  - { href: "#ds1", label: "DS-1" }
  - { href: "#preeq", label: "Pre-amp EQ" }
  - { href: "#peq", label: "Parametric EQ" }
  - { href: "#delay", label: "Delay" }
  - { href: "#reverb", label: "Reverb" }
prev: { href: "getting-started.html", label: "Get started" }
next: { href: "amps-cabs.html", label: "Amps, cabinets &amp; IRs" }
---

## The board <span class="muted">(add / remove / bypass)</span> {#board}

The **Guitar Rig** shows one compact tile per pedal that's on the board, followed by a `+ ADD` tile. <kbd>Tab</kbd> or <kbd>←</kbd>/<kbd>→</kbd> to a tile to load it into the full-size editor below — the editor takes on the pedal's livery colour. Only **enabled** pedals are on the board at startup; everything else lives in the picker.

| Key | Action |
| --- | ------ |
| <kbd>Enter</kbd> / <kbd>Space</kbd> <span class="muted">(on `+ ADD`)</span> | Open the picker listing pedals not on the board |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Navigate the picker |
| <kbd>Enter</kbd> <span class="muted">(in picker)</span> | Add the selected pedal and jump focus to it |
| <kbd>Esc</kbd> | Close the picker |
| <kbd>D</kbd> <span class="muted">(on a pedal)</span> | Remove it from the board |
| <kbd>Space</kbd> <span class="muted">(on a pedal)</span> | Bypass / un-bypass it without removing it |

<div class="note">
<b>Chain order is fixed</b> and follows the signal flow shown on the header ribbon:
Gate → Comp → Fuzz → TS-808 → DS-1 → Pre-EQ → <b>Amp</b> → <b>Cab</b> → Parametric EQ → Delay → Reverb.
Where a pedal sits in that order is part of its character — see <a href="how-it-works.html">How it works</a>.
</div>

## <span style="color:var(--gray)">●</span> Noise Gate {#gate}

An envelope follower drives a smooth gain ramp (no clicks) that silences hum and string noise between phrases. Runs first so it cleans the rawest signal.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Thresh | 0–10 | Gate open threshold. 0 = opens at very low levels (−80 dB), 10 = always open. Start around 2–3 for high-gain tones. |
| Release | 0–10 | How long the gate stays open after the signal drops below threshold. Higher = slower, more natural decay. |

## <span style="color:var(--gold)">●</span> Compressor {#comp}

Sits right after the gate, before the drive stages, so it evens out picking dynamics and adds sustain going into the amp — the classic "studio" upgrade for clean and edge-of-breakup tones. A peak-follower detector drives a hard-knee gain computer; auto makeup keeps the level steady as you add compression.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Sustain | 0–10 | Compression amount — lowers the threshold (−6 dB → −40 dB) and raises the ratio (2:1 → 10:1). Higher = more squash and sustain. |
| Attack | 0–10 | How fast the compressor clamps a transient (0.5 ms → 50 ms). Low = snappy/tight, high = lets the pick attack through. |
| Level | 0–10 | Output makeup gain (≈0–2×). 5 = unity with auto makeup. |

## <span style="color:var(--magenta)">●</span> Fuzz <span class="muted">(Big Muff style)</span> {#fuzz}

Runs first in the drive chain so it sees the rawest pickup signal. Two cascaded clipping stages give the long, singing sustain and near-square saturation of a vintage fuzz — much heavier than the TS or DS-1. The voice is mid-scooped at 700 Hz for the classic "wall of sound" timbre.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Fuzz | 0–10 | Sustain/gain into the two cascaded soft-clip stages. High values drive the waveform toward a gated square wave. |
| Tone | 0–10 | Low-pass after the scoop. 0 = dark/woolly (~400 Hz), 10 = bright/buzzy (~6 kHz). |
| Level | 0–10 | Output volume of the pedal into the next stage. |

## <span style="color:var(--green)">●</span> TS-808 Tube Screamer {#ts}

The legendary mid-hump boost — asymmetric diode clipping that tightens the low end and pushes an amp into focused saturation. Great as a clean boost into an already-driven amp.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Drive | 0–10 | Pre-clip gain (1×–51×). High values push the asymmetric diode clippers into saturation. |
| Tone | 0–10 | Low-pass cutoff after clipping. 0 = dark (~500 Hz), 10 = bright (~7 kHz). |
| Level | 0–10 | Output volume of the pedal into the next stage. |

## <span style="color:#e08840">●</span> DS-1 Distortion {#ds1}

More aggressive than the TS, with a cubic clip stage and a seesaw "tilt" tone control that trades bass for treble around 1 kHz.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Drive | 0–10 | Gain into the cubic clip stage (1×–61×). More aggressive than the TS. |
| Tone | 0–10 | Tilt control (bass↔treble seesaw around ~1 kHz). 0 = dark & full, 5 = flat, 10 = bright & cutting. |
| Level | 0–10 | Output volume of the pedal into the next stage. |

## <span style="color:var(--lime)">●</span> Pre-amp EQ {#preeq}

Sits **before the amp**, so it shapes the signal that the gain stage actually clips — a different job from the post-cab Parametric EQ below, which colours the final mix. Scoop the mids going in for a tighter chug, or push them for lead sustain. All three bands map 0–10 to −12 dB → 0 dB → +12 dB. Centre (5.0) is flat.

| Knob | Frequency | Type |
| ---- | --------- | ---- |
| Low | 100 Hz | Low shelf |
| Mid | 650 Hz | Peak (Q 1.0) |
| High | 3 kHz | High shelf |

## <span style="color:var(--teal)">●</span> Parametric EQ {#peq}

Post-cabinet — shapes the final stereo tone after distortion. All three bands map 0–10 to −15 dB → 0 dB → +15 dB. Centre (5.0) is unity gain.

| Knob | Frequency | Type |
| ---- | --------- | ---- |
| Low | 120 Hz | Low shelf |
| Mid | 800 Hz | Peak (Q 1.5) |
| High | 5 kHz | High shelf |

## <span style="color:var(--purple)">●</span> Delay <span class="muted">(stereo ping-pong)</span> {#delay}

Feedback cross-feeds the two channels so repeats bounce left ↔ right.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Time | 0–10 | Delay time 0–500 ms. |
| Feedback | 0–10 | Repeat level. Capped at 85% internally to prevent runaway. |
| Mix | 0–10 | Dry/wet blend. |

## <span style="color:var(--blue)">●</span> Stereo Reverb {#reverb}

Two decorrelated Freeverb cores (the right channel's delay lines are offset) produce a wide, deep stereo tail.

| Knob | Range | Effect |
| ---- | ----- | ------ |
| Room | 0–10 | Decay time (Freeverb room size). |
| Damp | 0–10 | High-frequency absorption in the feedback path. |
| Mix | 0–10 | Dry/wet blend (0 = fully dry, 10 = fully wet). |
