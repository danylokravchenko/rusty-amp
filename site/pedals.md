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
  - { href: "#pedals", label: "All pedals" }
prev: { href: "getting-started.html", label: "Get started" }
next: { href: "amps-cabs.html", label: "Amps, cabinets &amp; IRs" }
---

## The board <span class="muted">(add / remove / bypass)</span> {#board}

<figure class="shot">
  <div class="shot__bar"><i></i><i></i><i></i></div>
  <img src="assets/pedalboard.png" alt="rusty-amp pedal board" />
</figure>

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

## All pedals {#pedals}

Pick a pedal to load it into the editor — exactly like tabbing to its tile in the app. Every knob runs 0–10.

<div class="selector" data-tabs data-tabs-hash>
  <div class="tiles tiles--pedals" role="tablist" aria-label="Pedals">
    <button class="tile is-active" style="--c:var(--gray)" role="tab" aria-selected="true" data-tab="gate">
      <div class="tile__name"><span class="tile__dot"></span>Noise Gate</div>
      <div class="tile__sub">Thresh · Release</div>
    </button>
    <button class="tile" style="--c:var(--gold)" role="tab" aria-selected="false" data-tab="comp">
      <div class="tile__name"><span class="tile__dot"></span>Compressor</div>
      <div class="tile__sub">Sustain · Attack · Level</div>
    </button>
    <button class="tile" style="--c:var(--magenta)" role="tab" aria-selected="false" data-tab="fuzz">
      <div class="tile__name"><span class="tile__dot"></span>Fuzz</div>
      <div class="tile__sub">Fuzz · Tone · Level</div>
    </button>
    <button class="tile" style="--c:var(--green)" role="tab" aria-selected="false" data-tab="ts">
      <div class="tile__name"><span class="tile__dot"></span>TS-808</div>
      <div class="tile__sub">Drive · Tone · Level</div>
    </button>
    <button class="tile" style="--c:#e08840" role="tab" aria-selected="false" data-tab="ds1">
      <div class="tile__name"><span class="tile__dot"></span>DS-1</div>
      <div class="tile__sub">Drive · Tone · Level</div>
    </button>
    <button class="tile" style="--c:var(--lime)" role="tab" aria-selected="false" data-tab="preeq">
      <div class="tile__name"><span class="tile__dot"></span>Pre-amp EQ</div>
      <div class="tile__sub">Low · Mid · High</div>
    </button>
    <button class="tile" style="--c:var(--teal)" role="tab" aria-selected="false" data-tab="peq">
      <div class="tile__name"><span class="tile__dot"></span>Parametric EQ</div>
      <div class="tile__sub">Low · Mid · High</div>
    </button>
    <button class="tile" style="--c:var(--purple)" role="tab" aria-selected="false" data-tab="delay">
      <div class="tile__name"><span class="tile__dot"></span>Delay</div>
      <div class="tile__sub">Time · Feedback · Mix</div>
    </button>
    <button class="tile" style="--c:var(--blue)" role="tab" aria-selected="false" data-tab="reverb">
      <div class="tile__name"><span class="tile__dot"></span>Stereo Reverb</div>
      <div class="tile__sub">Room · Damp · Mix</div>
    </button>
  </div>

  <div class="tab-panel is-active" style="--c:var(--gray)" role="tabpanel" data-panel="gate">
    <p class="muted">An envelope follower drives a smooth gain ramp (no clicks) that silences hum and string noise between phrases. Runs first so it cleans the rawest signal.</p>
    <div class="kv"><span class="kv__k">Thresh</span><span class="kv__v"><span class="kv__r">0–10</span> Gate open threshold. 0 = opens at very low levels (−80 dB), 10 = always open. Start around 2–3 for high-gain tones.</span></div>
    <div class="kv"><span class="kv__k">Release</span><span class="kv__v"><span class="kv__r">0–10</span> How long the gate stays open after the signal drops below threshold. Higher = slower, more natural decay.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--gold)" role="tabpanel" data-panel="comp">
    <p class="muted">Sits right after the gate, before the drive stages, so it evens out picking dynamics and adds sustain going into the amp — the classic “studio” upgrade for clean and edge-of-breakup tones. A peak-follower detector drives a hard-knee gain computer; auto makeup keeps the level steady as you add compression.</p>
    <div class="kv"><span class="kv__k">Sustain</span><span class="kv__v"><span class="kv__r">0–10</span> Compression amount — lowers the threshold (−6 dB → −40 dB) and raises the ratio (2:1 → 10:1). Higher = more squash and sustain.</span></div>
    <div class="kv"><span class="kv__k">Attack</span><span class="kv__v"><span class="kv__r">0–10</span> How fast the compressor clamps a transient (0.5 ms → 50 ms). Low = snappy/tight, high = lets the pick attack through.</span></div>
    <div class="kv"><span class="kv__k">Level</span><span class="kv__v"><span class="kv__r">0–10</span> Output makeup gain (≈0–2×). 5 = unity with auto makeup.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--magenta)" role="tabpanel" data-panel="fuzz">
    <p class="muted">Big-Muff-style fuzz that runs first in the drive chain so it sees the rawest pickup signal. Two cascaded clipping stages give the long, singing sustain and near-square saturation of a vintage fuzz — much heavier than the TS or DS-1. The voice is mid-scooped at 700 Hz for the classic “wall of sound” timbre.</p>
    <div class="kv"><span class="kv__k">Fuzz</span><span class="kv__v"><span class="kv__r">0–10</span> Sustain/gain into the two cascaded soft-clip stages. High values drive the waveform toward a gated square wave.</span></div>
    <div class="kv"><span class="kv__k">Tone</span><span class="kv__v"><span class="kv__r">0–10</span> Low-pass after the scoop. 0 = dark/woolly (~400 Hz), 10 = bright/buzzy (~6 kHz).</span></div>
    <div class="kv"><span class="kv__k">Level</span><span class="kv__v"><span class="kv__r">0–10</span> Output volume of the pedal into the next stage.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--green)" role="tabpanel" data-panel="ts">
    <p class="muted">The legendary mid-hump boost — asymmetric diode clipping that tightens the low end and pushes an amp into focused saturation. Great as a clean boost into an already-driven amp.</p>
    <div class="kv"><span class="kv__k">Drive</span><span class="kv__v"><span class="kv__r">0–10</span> Pre-clip gain (1×–51×). High values push the asymmetric diode clippers into saturation.</span></div>
    <div class="kv"><span class="kv__k">Tone</span><span class="kv__v"><span class="kv__r">0–10</span> Low-pass cutoff after clipping. 0 = dark (~500 Hz), 10 = bright (~7 kHz).</span></div>
    <div class="kv"><span class="kv__k">Level</span><span class="kv__v"><span class="kv__r">0–10</span> Output volume of the pedal into the next stage.</span></div>
  </div>

  <div class="tab-panel" style="--c:#e08840" role="tabpanel" data-panel="ds1">
    <p class="muted">More aggressive than the TS, with a cubic clip stage and a seesaw “tilt” tone control that trades bass for treble around 1 kHz.</p>
    <div class="kv"><span class="kv__k">Drive</span><span class="kv__v"><span class="kv__r">0–10</span> Gain into the cubic clip stage (1×–61×). More aggressive than the TS.</span></div>
    <div class="kv"><span class="kv__k">Tone</span><span class="kv__v"><span class="kv__r">0–10</span> Tilt control (bass↔treble seesaw around ~1 kHz). 0 = dark &amp; full, 5 = flat, 10 = bright &amp; cutting.</span></div>
    <div class="kv"><span class="kv__k">Level</span><span class="kv__v"><span class="kv__r">0–10</span> Output volume of the pedal into the next stage.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--lime)" role="tabpanel" data-panel="preeq">
    <p class="muted">Sits <b>before the amp</b>, so it shapes the signal that the gain stage actually clips — a different job from the post-cab Parametric EQ, which colours the final mix. Scoop the mids going in for a tighter chug, or push them for lead sustain. All three bands map 0–10 to −12 dB → 0 dB → +12 dB; centre (5.0) is flat.</p>
    <div class="kv"><span class="kv__k">Low</span><span class="kv__v"><span class="kv__r">100 Hz</span> Low shelf.</span></div>
    <div class="kv"><span class="kv__k">Mid</span><span class="kv__v"><span class="kv__r">650 Hz</span> Peak (Q 1.0).</span></div>
    <div class="kv"><span class="kv__k">High</span><span class="kv__v"><span class="kv__r">3 kHz</span> High shelf.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--teal)" role="tabpanel" data-panel="peq">
    <p class="muted">Post-cabinet — shapes the final stereo tone after distortion. All three bands map 0–10 to −15 dB → 0 dB → +15 dB; centre (5.0) is unity gain.</p>
    <div class="kv"><span class="kv__k">Low</span><span class="kv__v"><span class="kv__r">120 Hz</span> Low shelf.</span></div>
    <div class="kv"><span class="kv__k">Mid</span><span class="kv__v"><span class="kv__r">800 Hz</span> Peak (Q 1.5).</span></div>
    <div class="kv"><span class="kv__k">High</span><span class="kv__v"><span class="kv__r">5 kHz</span> High shelf.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--purple)" role="tabpanel" data-panel="delay">
    <p class="muted">Stereo ping-pong: feedback cross-feeds the two channels so repeats bounce left ↔ right.</p>
    <div class="kv"><span class="kv__k">Time</span><span class="kv__v"><span class="kv__r">0–10</span> Delay time 0–500 ms.</span></div>
    <div class="kv"><span class="kv__k">Feedback</span><span class="kv__v"><span class="kv__r">0–10</span> Repeat level. Capped at 85% internally to prevent runaway.</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--blue)" role="tabpanel" data-panel="reverb">
    <p class="muted">Two decorrelated Freeverb cores (the right channel's delay lines are offset) produce a wide, deep stereo tail.</p>
    <div class="kv"><span class="kv__k">Room</span><span class="kv__v"><span class="kv__r">0–10</span> Decay time (Freeverb room size).</span></div>
    <div class="kv"><span class="kv__k">Damp</span><span class="kv__v"><span class="kv__r">0–10</span> High-frequency absorption in the feedback path.</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend (0 = fully dry, 10 = fully wet).</span></div>
  </div>
</div>
