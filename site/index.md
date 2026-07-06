---
layout: base.njk
permalink: index.html
title: "rusty-amp · a guitar amp & pedalboard rig in your terminal"
ogTitle: "rusty-amp"
description: "rusty-amp recreates classic tube and solid-state guitar amps, a full board of stompbox effects, and multi-mic'd 4×12 cabinets — all from a fast, keyboard-only terminal interface."
---

<header class="hero">
  <div class="wrap">
    <p class="eyebrow">● POWER ON · ON AIR</p>
    <h1>rusty-amp</h1>
    <p class="tagline">a guitar amp &amp; pedalboard rig in your <span class="term">terminal</span></p>
    <p class="blurb">
      Plug in your guitar, pick an amp, and play. rusty-amp recreates classic tube and
      solid-state amplifiers, a full board of stompbox effects, and multi-mic'd 4×12
      cabinets — all driven from a fast, keyboard-only interface with live metering.
      It ships with artist-inspired presets, so you can dial in a great tone in seconds.
    </p>
    <div class="cta">
      <a class="btn btn--primary" href="getting-started.html">▶ Get started</a>
      <a class="btn" href="https://github.com/danylokravchenko/rusty-amp/releases/latest">⬇ Download binary</a>
      <a class="btn" href="how-it-works.html">⚙ How it works</a>
      <a class="btn" href="#hear">🔊 Hear samples</a>
    </div>

    <figure class="shot">
      <div class="shot__bar"><i></i><i></i><i></i></div>
      <video controls muted loop playsinline preload="none" poster="assets/screenshot.png" style="width:100%;display:block">
        <source src="assets/demo.mp4" type="video/mp4" />
        <img src="assets/demo.gif" alt="rusty-amp demo: switching amps and stacking pedals from the terminal" />
      </video>
    </figure>
  </div>
</header>

<main class="wrap">

  <section>
    <div class="panel">
      <span class="panel__label">Signal chain</span>
      <p class="chain">
        <b>Guitar</b> <span class="arr">→</span> GATE <span class="arr">→</span> WHAMMY
        <span class="arr">→</span> WAH <span class="arr">→</span> COMP <span class="arr">→</span> FUZZ <span class="arr">→</span> TS-808
        <span class="arr">→</span> DS-1 <span class="arr">→</span> ML-2 <span class="arr">→</span> PRE-EQ
        <span class="arr">→</span> <b>AMP</b> <span class="arr">→</span> <b>CAB</b>
        <span class="arr">→</span> EQ <span class="arr">→</span> FLANGER
        <span class="arr">→</span> CHORUS <span class="arr">→</span> PHASER <span class="arr">→</span> DELAY
        <span class="arr">→</span> REVERB <span class="arr">→</span> <b>OUTPUT</b>
      </p>
      <div class="meter"></div>
      <p class="small muted" style="text-align:center;margin:0">
        Sample-by-sample DSP · 8× oversampled amp stages · partitioned-FFT cabinet convolution · true stereo out
      </p>
    </div>
  </section>

  <section>
    <h2>Motivation</h2>
    <p>
      Every person who plays guitar knows the routine: you need to boot up heavyweight
      software, load a plugin for amp and cabinet simulation, and wait. That software is
      great for professional studio work, but what if you just want to play your instrument
      without any complications? In addition, it's usually platform-locked, the licenses cost
      real money, and they limit how many devices you can authorize.
    </p>
    <p>
      🎸 rusty-amp — a complete guitar amp and pedalboard rig that's a single ~1.5&nbsp;MB
      binary. No installer. No license server. No subscription. No “activate this device”.
      You download one tiny file, plug in your guitar, and you're playing on any platform
      right in your terminal.
    </p>
    <div class="panel panel--accent" style="--accent:var(--gold)">
      <span class="panel__label">Important</span>
      <p style="margin:0">rusty-amp does not try to replace professional studio software.</p>
    </div>
    <p>
      What's important is that it doesn't cut corners on tone. Inside that tiny binary:
      amplifiers, multi-mic'd cabinets, and a full pedalboard, with artist-inspired presets to
      land a great tone. Under the hood, it's real DSP, not a toy. Each amp is modeled on the
      actual circuit, so the controls push and pull on each other the way they do on a real
      analog tone stack, and the low end “blooms” dynamically as you dig in, with the power amp
      and speaker interacting just like the real thing. The cabinets aren't just an EQ curve
      either. A real cab in a room has a “fingerprint”: its size, the cone resonance, the mic,
      and the early reflections. rusty-amp captures all of it as an impulse response, over a
      thousand taps long, so every reflection and resonance rings out, and the tone comes
      through deep and juicy instead of flat and sterile.
    </p>
    <p>
      On top of that sit complex effects which add width and depth to sound. All of it runs in
      real time on a single audio thread, and the whole rig consumes only about 15% CPU, thanks
      to Rust, convolution, and FFT.
    </p>
  </section>

  <section>
    <h2>Highlights</h2>
    <div class="grid grid--3">
      <div class="card"><div class="ico">🔊</div><h3>3 amplifiers</h3><p>Marshall JCM800, Mesa Dual Rectifier, and Randall Warhead — switchable while you play.</p></div>
      <div class="card"><div class="ico">📦</div><h3>3 cabinets + your IRs</h3><p>Mesa, Marshall, and Orange 4×12s, each captured with three blendable mics. Load your own <code>.wav</code> IR and A/B it live.</p></div>
      <div class="card"><div class="ico">🎛️</div><h3>Full pedalboard</h3><p>Gate, compressor, fuzz, Tube Screamer, DS-1, EQ, ping-pong delay, and stereo reverb. Add, remove, and bypass on the fly.</p></div>
      <div class="card"><div class="ico">🎧</div><h3>Studio-grade stereo</h3><p>Wide, three-dimensional sound from the cab, delay, and reverb — a real L/R image, not a faked widener.</p></div>
      <div class="card"><div class="ico">💾</div><h3>Ready-made presets</h3><p>Instant tones inspired by Metallica, Pantera, Slayer, Death, Slipknot, and more.</p></div>
      <div class="card"><div class="ico">🔌</div><h3>CLAP plugin host</h3><p>Drop a third-party CLAP effect into the chain and tweak its parameters without leaving the TUI.</p></div>
      <div class="card"><div class="ico">🎚️</div><h3>AU amp host <span class="muted">(macOS)</span></h3><p>Load an Audio Unit amp sim — e.g. a Marshall plugin — as an amp-position override that replaces the built-in amp &amp; cab, and A/B it live.</p></div>
      <div class="card"><div class="ico">🎵</div><h3>Built-in tuner</h3><p>Press <kbd>T</kbd> for a chromatic tuner with a ±cents needle and a live note spectrum.</p></div>
      <div class="card"><div class="ico">🥁</div><h3>Practice metronome</h3><p>Press <kbd>M</kbd> for an adjustable-tempo click you play along to — mixed into your monitor but never into your recordings.</p></div>
      <div class="card"><div class="ico">⏺️</div><h3>One-key recording</h3><p>Capture the fully-processed signal straight to a stereo WAV file with a single keystroke.</p></div>
      <div class="card"><div class="ico">🖥️</div><h3>Cross-platform</h3><p>Runs natively on macOS, Windows, and Linux via <a href="https://github.com/RustAudio/cpal">cpal</a>.</p></div>
    </div>
  </section>

  <section id="hear">
    <h2>Hear it</h2>
    <p class="lead">Nothing is better than a showcase! DI guitar track — no re-amping, no post-processing, straight out of rusty-amp.</p>
    <div class="samplegrid">
      <div class="sample" style="--c:var(--rust)">
        <div class="sample__head"><span class="sample__name">Default tone</span><span class="sample__rig">Mesa Dual Rectifier · Mesa V30</span></div>
        <p class="sample__desc">What you hear on first launch — stock knob positions, Tube Screamer, nothing else in the chain.</p>
        <audio controls preload="none" src="assets/audio/default.wav"></audio>
      </div>
      <div class="sample" style="--c:var(--blue)">
        <div class="sample__head"><span class="sample__name">Clean melodic</span><span class="sample__rig">Marshall JCM800 · Greenback</span></div>
        <p class="sample__desc">Warm glassy clean — gentle compression, edge-of-breakup gain, delay + hall reverb.</p>
        <audio controls preload="none" src="assets/audio/clean_melodic.wav"></audio>
      </div>
      <div class="sample" style="--c:var(--gold)">
        <div class="sample__head"><span class="sample__name">Solo seeker</span><span class="sample__rig">Mesa Dual Rectifier · Mesa V30</span></div>
        <p class="sample__desc">Lead tone — sustain-focused, delay + reverb, on-axis mic for pick-attack clarity.</p>
        <audio controls preload="none" src="assets/audio/solo_seeker.wav"></audio>
      </div>
      <div class="sample" style="--c:var(--magenta)">
        <div class="sample__head"><span class="sample__name">Metalcore shred</span><span class="sample__rig">Mesa Dual Rectifier · Mesa V30</span></div>
        <p class="sample__desc">Tight djent-adjacent rhythm and singing high-gain leads — Tube Screamer slamming the front end, pre-amp EQ tightening the chug.</p>
        <audio controls preload="none" src="assets/audio/metalcore.wav"></audio>
      </div>
      <div class="sample" style="--c:var(--purple)">
        <div class="sample__head"><span class="sample__name">Crowbar sludge</span><span class="sample__rig">Marshall JCM800 · Orange PPC412</span></div>
        <p class="sample__desc">Downtuned NOLA sludge — a massive low-mid wall of raw Marshall grind, Tube Screamer firming the drop tuning, cavernous Orange weight.</p>
        <audio controls preload="none" src="assets/audio/crowbar.wav"></audio>
      </div>
      <div class="sample sample--soon">
        <div class="sample__head"><span class="sample__name">More coming soon</span></div>
        <p class="sample__desc">The other bundled presets are getting their own clips — check back.</p>
      </div>
    </div>
    <p style="margin-top:14px"><a class="btn" href="presets.html#bundled">All bundled presets →</a></p>
  </section>

  <section>
    <h2>The pedalboard</h2>
    <p class="lead">Eleven effects, each added, removed, and bypassed independently — the board shows only what you're using.</p>
    <div class="pedalgrid" style="margin-top:18px">
      <div class="pedal" style="--c:var(--gray)"><div class="pedal__name"><span class="dot"></span>Noise Gate</div><div class="pedal__knobs">Thresh · Release</div><div class="pedal__desc">Envelope-follower gate that silences hum and hiss between riffs.</div></div>
      <div class="pedal" style="--c:var(--cyan)"><div class="pedal__name"><span class="dot"></span>Whammy</div><div class="pedal__knobs">Pitch · Mix · Tone</div><div class="pedal__desc">Octave-up/down pitch shifter in front of the amp — dive-bombs and fake-baritone heaviness.</div></div>
      <div class="pedal" style="--c:var(--orchid)"><div class="pedal__name"><span class="dot"></span>Auto-Wah</div><div class="pedal__knobs">Freq · Sens · Q · Mix</div><div class="pedal__desc">Envelope-swept resonant filter — a vocal auto-wah that tracks your picking.</div></div>
      <div class="pedal" style="--c:var(--gold)"><div class="pedal__name"><span class="dot"></span>Compressor</div><div class="pedal__knobs">Sustain · Attack · Level</div><div class="pedal__desc">Hard-knee compressor with auto makeup — evens out picking, adds sustain.</div></div>
      <div class="pedal" style="--c:var(--magenta)"><div class="pedal__name"><span class="dot"></span>Fuzz</div><div class="pedal__knobs">Fuzz · Tone · Level</div><div class="pedal__desc">Big-Muff-style two-stage clipper with a scooped “wall of sound” voice.</div></div>
      <div class="pedal" style="--c:var(--green)"><div class="pedal__name"><span class="dot"></span>TS-808</div><div class="pedal__knobs">Drive · Tone · Level</div><div class="pedal__desc">The legendary Tube Screamer — asymmetric diode clip, mid-hump boost.</div></div>
      <div class="pedal" style="--c:#e08840"><div class="pedal__name"><span class="dot"></span>DS-1</div><div class="pedal__knobs">Drive · Tone · Level</div><div class="pedal__desc">Aggressive cubic distortion with a bass↔treble tilt tone control.</div></div>
      <div class="pedal" style="--c:var(--steel)"><div class="pedal__name"><span class="dot"></span>ML-2 Metal Core</div><div class="pedal__knobs">Dist · Low · High · Level</div><div class="pedal__desc">Ultra-high-gain cascaded distortion with a powerful active two-band EQ.</div></div>
      <div class="pedal" style="--c:var(--lime)"><div class="pedal__name"><span class="dot"></span>Pre-amp EQ</div><div class="pedal__knobs">Low · Mid · High</div><div class="pedal__desc">Shapes the signal <em>before</em> the amp clips — tighten the chug or push leads.</div></div>
      <div class="pedal" style="--c:var(--teal)"><div class="pedal__name"><span class="dot"></span>Parametric EQ</div><div class="pedal__knobs">Low · Mid · High</div><div class="pedal__desc">Post-cabinet tone shaping of the final stereo mix.</div></div>
      <div class="pedal" style="--c:var(--indigo)"><div class="pedal__name"><span class="dot"></span>Flanger</div><div class="pedal__knobs">Rate · Depth · Feedback · Mix</div><div class="pedal__desc">LFO-swept comb filter — the classic metallic “jet plane” sweep, in stereo.</div></div>
      <div class="pedal" style="--c:var(--pink)"><div class="pedal__name"><span class="dot"></span>Chorus</div><div class="pedal__knobs">Rate · Depth · Mix</div><div class="pedal__desc">LFO-swept long delay, no feedback — lush, watery thickening in stereo.</div></div>
      <div class="pedal" style="--c:var(--yellow)"><div class="pedal__name"><span class="dot"></span>Phaser</div><div class="pedal__knobs">Rate · Depth · Feedback · Mix</div><div class="pedal__desc">Four-stage all-pass sweep — the hollow, vocal “whoosh”, in stereo.</div></div>
      <div class="pedal" style="--c:var(--purple)"><div class="pedal__name"><span class="dot"></span>Delay</div><div class="pedal__knobs">Time · Feedback · Mix</div><div class="pedal__desc">Stereo ping-pong — repeats bounce L↔R, up to 500 ms.</div></div>
      <div class="pedal" style="--c:var(--blue)"><div class="pedal__name"><span class="dot"></span>Stereo Reverb</div><div class="pedal__knobs">Room · Damp · Mix</div><div class="pedal__desc">Dual decorrelated Freeverb cores for a wide, deep tail.</div></div>
    </div>
    <p style="margin-top:18px"><a class="btn" href="pedals.html">Full pedal &amp; knob reference →</a></p>
  </section>

  <section>
    <h2>Amps &amp; cabinets</h2>
    <div class="grid grid--2">
      <div class="panel panel--accent" style="--accent:var(--rust)">
        <span class="panel__label">3 amp models</span>
        <ul class="clean">
          <li><b>Marshall JCM800</b> — punchy, dynamic, touch-sensitive · dual 12AX7, passive FMV tone stack, tube sag.</li>
          <li><b>Mesa Dual Rectifier</b> — compressed, aggressive, modern · triple gain stage, silicon sag.</li>
          <li><b>Randall Warhead</b> — tight, crushing, solid-state · FET→BJT→rail-clip, active tone stack.</li>
        </ul>
      </div>
      <div class="panel panel--accent" style="--accent:var(--teal)">
        <span class="panel__label">3 cabinets + IR loader</span>
        <ul class="clean">
          <li><b>Mesa 4×12 (V30)</b> — scooped, aggressive, forward-projecting.</li>
          <li><b>Marshall 4×12 (Greenback)</b> — warm, mid-forward, smooth top.</li>
          <li><b>Orange PPC412 (V30)</b> — thick, chunky, closed-back birch.</li>
          <li>Load any <code>.wav</code> impulse response and A/B against the built-ins with <kbd>X</kbd>.</li>
        </ul>
      </div>
    </div>
    <p style="margin-top:6px"><a class="btn" href="amps-cabs.html">Amps, cabinets &amp; IRs →</a></p>
  </section>

  <section>
    <h2>Quick start</h2>
    <p class="muted">Pre-built binaries have presets baked in — nothing else to download.</p>
    <pre data-lang="macOS · Apple Silicon"><code>curl -L https://github.com/danylokravchenko/rusty-amp/releases/latest/download/rusty-amp-macos-aarch64 -o rusty-amp
chmod +x rusty-amp
xattr -d com.apple.quarantine rusty-amp   # clear the unsigned-binary quarantine flag
./rusty-amp</code></pre>
    <div class="grid grid--3" style="margin-top:10px">
      <a class="card" href="getting-started.html"><h3>① Install</h3><p>Grab a binary or build from source.</p></a>
      <a class="card" href="getting-started.html#startup"><h3>② Pick devices</h3><p>Choose your interface, input channel, and output.</p></a>
      <a class="card" href="presets.html"><h3>③ Load a preset</h3><p>Press <kbd>P</kbd> and play.</p></a>
    </div>
  </section>

  <div class="pager">
    <span></span>
    <a class="next" href="getting-started.html">
      <div class="dir">Next →</div>
      <div class="ttl">Get started: install &amp; controls</div>
    </a>
  </div>

</main>
