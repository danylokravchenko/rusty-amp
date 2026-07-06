---
layout: page.njk
permalink: pedals.html
title: "Pedals & effects · rusty-amp"
ogTitle: "rusty-amp · pedals & effects"
description: "Every rusty-amp pedal explained: noise gate, whammy pitch shifter, auto-wah, compressor, fuzz, TS-808, DS-1, ML-2 Metal Core, pre-amp EQ, parametric EQ, flanger, chorus, phaser, delay, and stereo reverb — with full knob reference tables."
eyebrow: "Pedals & effects"
heading: "Configuring the board"
lead: "Fifteen effects you can add, remove, and bypass independently. Each knob runs 0–10; here's exactly what every one does."
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
Gate → Whammy → Wah → Comp → Fuzz → TS-808 → DS-1 → ML-2 → Pre-EQ → <b>Amp</b> → <b>Cab</b> → Parametric EQ → Flanger → Chorus → Phaser → Delay → Reverb.
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
    <button class="tile" style="--c:var(--cyan)" role="tab" aria-selected="false" data-tab="whammy">
      <div class="tile__name"><span class="tile__dot"></span>Whammy</div>
      <div class="tile__sub">Pitch · Mix · Tone</div>
    </button>
    <button class="tile" style="--c:var(--orchid)" role="tab" aria-selected="false" data-tab="wah">
      <div class="tile__name"><span class="tile__dot"></span>Auto-Wah</div>
      <div class="tile__sub">Freq · Sens · Q · Mix</div>
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
    <button class="tile" style="--c:var(--steel)" role="tab" aria-selected="false" data-tab="ml2">
      <div class="tile__name"><span class="tile__dot"></span>ML-2 Metal Core</div>
      <div class="tile__sub">Dist · Low · High · Level</div>
    </button>
    <button class="tile" style="--c:var(--lime)" role="tab" aria-selected="false" data-tab="preeq">
      <div class="tile__name"><span class="tile__dot"></span>Pre-amp EQ</div>
      <div class="tile__sub">Low · Mid · High</div>
    </button>
    <button class="tile" style="--c:var(--teal)" role="tab" aria-selected="false" data-tab="peq">
      <div class="tile__name"><span class="tile__dot"></span>Parametric EQ</div>
      <div class="tile__sub">Low · Mid · High</div>
    </button>
    <button class="tile" style="--c:var(--indigo)" role="tab" aria-selected="false" data-tab="flanger">
      <div class="tile__name"><span class="tile__dot"></span>Flanger</div>
      <div class="tile__sub">Rate · Depth · Feedback · Mix</div>
    </button>
    <button class="tile" style="--c:var(--pink)" role="tab" aria-selected="false" data-tab="chorus">
      <div class="tile__name"><span class="tile__dot"></span>Chorus</div>
      <div class="tile__sub">Rate · Depth · Mix</div>
    </button>
    <button class="tile" style="--c:var(--yellow)" role="tab" aria-selected="false" data-tab="phaser">
      <div class="tile__name"><span class="tile__dot"></span>Phaser</div>
      <div class="tile__sub">Rate · Depth · Feedback · Mix</div>
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

  <div class="tab-panel" style="--c:var(--cyan)" role="tabpanel" data-panel="whammy">
    <p class="muted">A time-domain pitch shifter that transposes your note up or down by up to an octave. It sits right after the gate — a real whammy's spot, in front of everything — so the <em>shifted</em> note is what the compressor and drives then work on: octave-down for a fake-baritone heaviness, octave-up shrieks and dive-bombs feeding raw into the distortion. Two crossfaded grains sweep a delay line at the pitch ratio; a little warble is inherent to the method and part of the whammy character.</p>
    <div class="kv"><span class="kv__k">Pitch</span><span class="kv__v"><span class="kv__r">0–10</span> Transpose interval. 0 = −12 semitones (octave down), 5 = unison, 10 = +12 semitones (octave up).</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend. 0 = dry, 10 = fully wet (full dive-bomb / pure interval).</span></div>
    <div class="kv"><span class="kv__k">Tone</span><span class="kv__v"><span class="kv__r">0–10</span> Low-pass on the wet path, taming the granular fizz. 0 = dark (~800 Hz), 10 = open (~10 kHz).</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--orchid)" role="tabpanel" data-panel="wah">
    <p class="muted">A resonant bandpass whose peak is swept by an envelope follower — the filter "opens" as you dig in and closes as the note decays. A real wah is treadle-swept, but a terminal has no expression pedal, so your picking dynamics drive it: with <b>Sens</b> at 0 it's a static "cocked wah" parked at the <b>Freq</b> position; turn Sens up and it's a dynamic, vocal auto-wah. Sits early — after the whammy, before the compressor and the drives — so it rides raw pick dynamics (a compressor ahead of it would flatten the envelope it sweeps on) and the vocal quack is what the distortion then saturates, the classic wah-into-fuzz voice. Built on a TPT state-variable filter so the peak can sweep continuously with no per-sample coefficient-rebuild cost.</p>
    <div class="kv"><span class="kv__k">Freq</span><span class="kv__v"><span class="kv__r">0–10</span> Base peak position (the heel-down point), ~300 Hz–1.5 kHz. With Sens at 0 this is a fixed cocked-wah tone.</span></div>
    <div class="kv"><span class="kv__k">Sens</span><span class="kv__v"><span class="kv__r">0–10</span> How far the envelope sweeps the peak up from the base (0 = static, 10 = a wide auto-sweep on your picking).</span></div>
    <div class="kv"><span class="kv__k">Q</span><span class="kv__v"><span class="kv__r">0–10</span> Resonance — the sharpness of the peak. Higher = a narrower, more vocal "quack".</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend. A real wah is fully wet (10); a lower blend keeps some dry body under the filter.</span></div>
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

  <div class="tab-panel" style="--c:var(--steel)" role="tabpanel" data-panel="ml2">
    <p class="muted">The Boss ML-2 Metal Core — an ultra-high-gain distortion built to make any amp sound metal on its own. Far more gain than the DS-1, from <b>two cascaded clipping stages</b> that saturate then square off into a long, compressed, singing sustain. A fixed mid-scoop gives it that aggressive metal voice, and a powerful active two-band EQ (Low/High, ±15 dB) reshapes the whole tone. Runs last in the drive chain, after the DS-1, so it's the most aggressive stage feeding the pre-amp EQ. Tightened on both sides of the clipper and 8× oversampled so the brutal gain stays articulate, not fizzy.</p>
    <div class="kv"><span class="kv__k">Dist</span><span class="kv__v"><span class="kv__r">0–10</span> Gain into the two cascaded clip stages (1×–101× on the first, plus a fixed second push). High = a wall of compressed saturation.</span></div>
    <div class="kv"><span class="kv__k">Low</span><span class="kv__v"><span class="kv__r">0–10</span> Active low shelf at 120 Hz, ±15 dB. 5 = flat, 0 = tight/thin, 10 = a bass-heavy wall.</span></div>
    <div class="kv"><span class="kv__k">High</span><span class="kv__v"><span class="kv__r">0–10</span> Active high shelf at 3.2 kHz, ±15 dB. 5 = flat, 0 = dark, 10 = bright and cutting.</span></div>
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

  <div class="tab-panel" style="--c:var(--indigo)" role="tabpanel" data-panel="flanger">
    <p class="muted">A short LFO-swept delay mixed back with the dry signal — the moving comb-filter notches give the classic metallic “jet plane” sweep. Sits after the cab and parametric EQ, before the delay, so it modulates the finished tone ahead of the ambience. The two channels read one LFO a quarter-cycle apart, so the sweep drifts across the stereo field.</p>
    <div class="kv"><span class="kv__k">Rate</span><span class="kv__v"><span class="kv__r">0–10</span> LFO speed, 0.05–5 Hz (exponential). Low = a slow, deep whoosh; high = a fast warble.</span></div>
    <div class="kv"><span class="kv__k">Depth</span><span class="kv__v"><span class="kv__r">0–10</span> Sweep width — how far the delay swings (~0.5 ms up to ~5 ms). Higher = a wider, more dramatic sweep.</span></div>
    <div class="kv"><span class="kv__k">Feedback</span><span class="kv__v"><span class="kv__r">0–10</span> Regeneration (0–90%). Feeds the swept tap back in, sharpening the notches into a resonant, ringing voice.</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend. 0 = dry, 5 = deepest flange (equal dry/wet), 10 = fully wet.</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--pink)" role="tabpanel" data-panel="chorus">
    <p class="muted">A long LFO-swept delay (~8–20 ms) mixed back with the dry signal — much longer than the flanger's sub-millisecond tap and with no feedback, so instead of a metallic comb sweep you get lush pitch-shimmer: several slightly detuned, slightly delayed copies of the note. Sits after the flanger, before the delay, modulating the finished tone ahead of the ambience. The two channels read one LFO half a cycle apart, so the shimmer drifts across the stereo field and widens the image.</p>
    <div class="kv"><span class="kv__k">Rate</span><span class="kv__v"><span class="kv__r">0–10</span> LFO speed, 0.05–5 Hz (exponential). Low = a slow, glassy drift; high = a fast, vibrato-like warble.</span></div>
    <div class="kv"><span class="kv__k">Depth</span><span class="kv__v"><span class="kv__r">0–10</span> Sweep width — how far the delay swings (up to ~12 ms). Higher = more detuning and a thicker, more obvious effect.</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend. 0 = dry, 5 = classic chorus (equal dry/wet), 10 = fully wet (pitch-vibrato).</span></div>
  </div>

  <div class="tab-panel" style="--c:var(--yellow)" role="tabpanel" data-panel="phaser">
    <p class="muted">The third sibling to the flanger and chorus — but where those sweep a delay line, the phaser sweeps a cascade of four all-pass filters and sums the result with the dry signal. The all-pass chain is flat on its own; the notches only appear in the dry+wet sum, gliding up and down the spectrum for that hollow, vocal "whoosh" (Phase 90 / Small Stone territory). Sits after the chorus, before the delay, in stereo — the two channels read one LFO a quarter-cycle apart so the sweep drifts across the image.</p>
    <div class="kv"><span class="kv__k">Rate</span><span class="kv__v"><span class="kv__r">0–10</span> LFO speed, 0.05–5 Hz (exponential). Low = a slow, deep swirl; high = a fast throb.</span></div>
    <div class="kv"><span class="kv__k">Depth</span><span class="kv__v"><span class="kv__r">0–10</span> Sweep width — how far the all-pass break frequency glides (~200 Hz up to ~1.6 kHz). Higher = a wider, more dramatic sweep.</span></div>
    <div class="kv"><span class="kv__k">Feedback</span><span class="kv__v"><span class="kv__r">0–10</span> Regeneration (0–90%). Feeds the wet output back in, sharpening the notches into resonant, throaty peaks.</span></div>
    <div class="kv"><span class="kv__k">Mix</span><span class="kv__v"><span class="kv__r">0–10</span> Dry/wet blend. 0 = dry, 5 = deepest phase (equal dry/wet), 10 = fully wet (the notches fade as the dry sum vanishes).</span></div>
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
