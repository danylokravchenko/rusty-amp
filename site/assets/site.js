// Mobile nav toggle + active-link highlighting for rusty-amp docs.
(function () {
  var burger = document.querySelector(".nav__burger");
  var links = document.querySelector(".nav__links");
  if (burger && links) {
    burger.addEventListener("click", function () {
      links.classList.toggle("open");
    });
  }

  // Highlight the current page in the nav.
  var here = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".nav__links a").forEach(function (a) {
    var href = a.getAttribute("href");
    if (!href) return;
    if (href === here || (here === "" && href === "index.html")) {
      a.classList.add("is-active");
    }
  });

  // Tabbed selectors (amps, cabinets, pedals, …). A [data-tabs] group holds
  // [data-tab="id"] tiles and matching [data-panel="id"] panels. Adding
  // data-tabs-hash makes the group sync the active tile to the URL hash, so
  // tiles double as shareable deep links.
  document.querySelectorAll("[data-tabs]").forEach(function (group) {
    var tabs = Array.prototype.slice.call(group.querySelectorAll("[data-tab]"));
    var panels = Array.prototype.slice.call(group.querySelectorAll("[data-panel]"));
    var useHash = group.hasAttribute("data-tabs-hash");

    function activate(id, writeHash) {
      var found = false;
      tabs.forEach(function (t) {
        var on = t.dataset.tab === id;
        if (on) found = true;
        t.classList.toggle("is-active", on);
        t.setAttribute("aria-selected", on ? "true" : "false");
      });
      panels.forEach(function (p) {
        p.classList.toggle("is-active", p.dataset.panel === id);
      });
      if (found && useHash && writeHash && history.replaceState) {
        history.replaceState(null, "", "#" + id);
      }
      return found;
    }

    tabs.forEach(function (t) {
      t.addEventListener("click", function () { activate(t.dataset.tab, true); });
      t.addEventListener("keydown", function (e) {
        var i = tabs.indexOf(t);
        var next = e.key === "ArrowRight" || e.key === "ArrowDown" ? i + 1
                 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? i - 1 : -2;
        if (next === -2) return;
        e.preventDefault();
        var dest = tabs[(next + tabs.length) % tabs.length];
        dest.focus();
        dest.click();
      });
    });

    // Open the tab named in the URL hash (e.g. pedals.html#fuzz) on load.
    if (useHash && location.hash) {
      activate(location.hash.slice(1), false);
    }
  });

  // ── Interactive recording-transport demo ─────────────────────────────────
  var rec = document.querySelector("[data-rec]");
  if (rec) {
    var recBtn = rec.querySelector(".rec__btn");
    var recTime = rec.querySelector(".rec__time");
    var recFile = rec.querySelector(".rec__file");
    var recAir = rec.querySelector(".rec__air");
    var recording = false, secs = 0, timer = null;

    // Build the waveform bars, then give them varied target heights.
    var wave = rec.querySelector(".rec__wave");
    if (wave && !wave.children.length) {
      for (var k = 0; k < 48; k++) wave.appendChild(document.createElement("i"));
    }
    Array.prototype.forEach.call(rec.querySelectorAll(".rec__wave i"), function (bar, i) {
      bar.style.setProperty("--h", (14 + ((i * 7) % 38)) + "px");
      bar.style.animationDelay = (i * 0.045) + "s";
    });

    function fmt(s) {
      var m = Math.floor(s / 60), ss = s % 60;
      return (m < 10 ? "0" : "") + m + ":" + (ss < 10 ? "0" : "") + ss;
    }

    recBtn.addEventListener("click", function () {
      recording = !recording;
      rec.classList.toggle("is-rec", recording);
      if (recording) {
        secs = 0;
        recTime.textContent = "00:00";
        recFile.textContent = "";
        recAir.textContent = "● ON AIR";
        recBtn.innerHTML = '<span class="dot">■</span>Stop';
        timer = setInterval(function () { secs++; recTime.textContent = fmt(secs); }, 1000);
      } else {
        clearInterval(timer);
        recAir.textContent = "○ OFF AIR";
        recBtn.innerHTML = '<span class="dot">●</span>Record';
        var ts = Math.floor(Date.now() / 1000);
        recFile.innerHTML = "Saved <code>~/rusty-amp-" + ts + ".wav</code> · " + fmt(secs) + " · 32-bit float stereo";
      }
    });
  }
})();
