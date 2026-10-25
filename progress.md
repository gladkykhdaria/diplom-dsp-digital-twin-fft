# Praesagium — Progress Log

Short, dated record of what's actually been decided and built so far.


---

**July 10,2026** — Started building the project for real, after the idea
came together at the DTU Summer School . All the hw components were bought.

**August 8, 2026** — DHT22 sensor found to fail 100% of the time
whenever the Wi-Fi driver (cyw43) was initialized, even with the AP
closed. After ruling out several causes (CPU priority, decoupling caps,
RF shielding, separate power, AP vs station mode), fixed structurally
by switching to I2C-based sensors instead (BME280 + MPU-6050), which
don't conflict with Wi-Fi.(i was just stupid)

**Sept 3, 2026** — Wiring work in KiCad; a wiring schematic screenshot
from this date exists, alongside an earlier hand-drawn version.

**Sept 4, 2026** — Motor + MOSFET (IRLZ34N) drive stage wiring and
testing. Motor wasn't spinning despite every individual multimeter
check (gate voltage, drain voltage, continuity) passing — root cause
found to be a missing ground return path: the motor's supply negative
was never tied into the shared ground. Fixed by connecting it.(again me + my stupidity)

**Sept 6, 2026** — Two things resolved the same day:
- Debug-probe (SWD) connection failures traced to motor switching
  noise on the shared ground (no flyback diode yet, motor flickering
  from an uncalibrated threshold). Fixed by adding a 1N4007 flyback
  diode across the motor terminals — confirmed stable over a 300+
  second run with zero SWD errors afterward.
- Vibration FFT calibration session: logged idle noise floor
  (0.735–1.63) vs. deliberate tap/spin disturbance peak (51.44), and
  set the anomaly threshold to 77.0 (~1.5× the observed max) from
  that real data rather than a guess.

**Sept 10, 2026** — Initial thesis-supervision request email sent 


**~Oct 5, 2026** — Prof. Pavaloiu confirmed as thesis supervisor. Valentin-Marian Stanca confirmed as co-superadviser.

**~Oct 14, 2026** — Valentin-Marian Stanca sketched a new direction by hand: make the work
"catchier," formalize the sensor → actuator feedback loop explicitly,
and add power/energy consumption as a measured axis of the Rust vs
Python comparison.

**Oct 14, 2026** — Several things done the same day:
- Restructured the Rust vs Python methodology into two explicit stages
  (Stage 1: Pico records real vibration data while running; Stage 2:
  identical FFT workload benchmarked in both languages on that same
  recorded data, on the same laptop) — a fix for a methodological gap
  a friend had flagged.
- Built a full project-scope write-up (architecture, hardware, pin
  reference, wiring, firmware, benchmark methodology, pipeline status,
  references).
- Explored the professor's pivot direction specifically: upgrading
  single-shot FFT to STFT, formalizing the feedback loop, and adding
  an energy-per-computation angle — framed as a Scandinavian
  "efficient by design" pitch. Status: **parked, not yet committed**.
- Wrote a full functional/non-functional requirements spec (FR-01
  through FR-18, NFR-01 through NFR-10), plus two rounds of creative
  additions: explainable alarm trace, drift-vs-spike detection, an
  FFI control arm for the benchmark, adaptive power-aware sampling,
  alarm hysteresis, a live on-dashboard spectrogram, a live
  energy-per-decision readout, and a live Rust-vs-Python replay demo.
  All marked as optional/stretch, not committed scope.
- Delivered comment-stripped versions of both code projects
  (`pico_hotspot`, `smart_station_web`) on request.

**Oct 19, 2026** — Several things done the same day:
- Built an architecture write-up with diagrams: the overall sensor-to-
  dashboard data flow, the feedback loop, the three-task concurrency
  model, and the TCP protocol sequence.
- Refactored the firmware's temperature/humidity/vibration control flow
  into explicit stages matching the professor's own sketch: a pure
  `controller_decide()` (decision only, no hardware access) and a
  `drive_actuators()` that is the only function touching any LED,
  buzzer, or motor pin. No behavior change, just the feedback loop made
  explicit in code instead of only in wiring.
- Built the live spectrogram for real (FR-16): a rolling ring buffer of
  compressed FFT magnitude frames on the Pico, a new `spectro` TCP
  command, and a scrolling canvas heatmap on the web dashboard. Kept
  deliberately one-way: the spectrogram never feeds the alarm logic.

  
**Oct 21, 2026** — Wrote a project-context handoff document (full
research question, hardware, software architecture, debugging history,
file inventory, and standing preferences) so a new chat could continue
the project without re-explaining everything.
 
**Oct 24, 2026** — Two hardware test sessions on real flashed firmware:
- First session: a debug-probe disconnect ("target device did not
  respond") right as the session was interrupted while the motor was
  active — consistent with the same motor-noise-on-shared-ground issue
  from Sept 6, not a new firmware bug. Added an explicit per-call log
  line (`drive_actuators call #N: motor pin commanded = ...`) so a
  firmware-side cause can be distinguished from a hardware-side one
  without a multimeter.
- Second session: confirmed, from the new log line, that the firmware
  commanded the motor pin identically every single time a "red" sim
  command was sent. What looked like "the motor stopped working" was
  actually the automatic BME280 polling (every 2.5s) reverting the
  simulated red state back to the real (green/yellow) room reading —
  the sim override is one-shot by design, not persistent. Not a bug.
**Oct 25, 2026** — Closed the last two open core requirements:
- **FR-11** (export real recorded vibration data): added a `dump` TCP
  command to the firmware. A new fixed-size ring buffer
  (`RAW_HISTORY`, 2048 raw Z-axis samples, ~10.24s at 200Hz) captures
  every raw accelerometer sample, independent of the FFT windowing,
  and streams it out as CSV on request.
- **FR-12** (Rust vs Python benchmark on real data): built
  `export_vibration_data.py` (pulls a `dump` off the Pico and saves it
  as CSV), `benchmark.rs`, and `benchmark.py` — identical methodology
  in both languages: load the same real recorded CSV, tile it up to
  three sizes (1,024 / 65,536 / 1,048,576 samples) since recording an
  hour of real data live isn't practical, run the FFT 30 times per
  size, report mean/stddev/min/max in milliseconds.
- Ran the full pipeline end to end on real hardware for the first
  time: exported a genuine 2048-sample recording from the Pico, ran
  both benchmarks against it. Rust was faster and far more consistent
  (lower jitter) at the smallest size; Python/NumPy was faster on raw
  throughput at the two larger sizes (NumPy's FFT calls into an
  optimized compiled C library, so this is partly measuring that
  library against `rustfft`, not language overhead alone), but Rust
  stayed more consistent across all three sizes. Added 3 new tests for
  the tiling logic (15 total, all passing) confirming tiling a real
  recording doesn't distort its frequency content.
**Oct 25, 2026 (later the same day)** — Decided to adopt the professor's
full pivot direction rather than leave any of it as optional stretch
scope: every remaining row in requirements.md's FR table, plus both
rounds of brainstormed ideas, implemented in one pass.
- **FR-13** (adaptive, power-aware sampling): the firmware now tracks a
  calm/alert sampling mode. While calm, the FFT itself only runs on
  every 4th vibration window instead of every one — raw sampling for
  the dump/spectrogram histories is unaffected, only the expensive
  compute step is throttled. Any sign of real risk (an active alarm, a
  spike, or the slow baseline climbing toward the threshold) switches
  back to full-rate FFT immediately.
- **FR-14** (drift vs. spike): added a slow exponential-moving-average
  baseline (mean and variance) of vibration energy, separate from both
  the instant reading and the fixed calibration thresholds. A reading
  far above that slow baseline (by a few standard deviations) is
  classified as a spike; the slow baseline itself climbing well past
  its value right after calibration is classified as drift. Both
  comparisons avoid needing a square root, which isn't readily
  available in this no_std build.
- **Explainable alarm trace**: every time the alarm actually trips, the
  firmware now records which frequency bin was dominant, how far over
  the threshold the triggering energy was, and how many consecutive
  windows the episode has lasted — not just the boolean that already
  existed.
- **FR-17** (live energy-per-decision estimate): a rough estimate,
  built from a known active-mode current draw and a cycle-count guess
  for the sampling and FFT work, updated every window and shown live.
  No INA219 yet — swapping one in later only changes the one function
  that computes the estimate.
- All four of the above are exposed through one new `diag` TCP command
  and a matching `/api/diag` route on the dashboard, with a new
  Diagnostics panel showing the sampling mode, trend, live/cumulative
  energy estimate, and the current alarm trace in plain language.
- **FR-18** (live Rust vs. Python replay): a new `/api/replay` route on
  the web app. One click pulls a real recorded vibration dump straight
  off the Pico, then runs the same windowed FFT-and-threshold decision
  through a Rust path (in-process, `rustfft`) and a Python path (a
  short-lived subprocess, NumPy) over that exact same data, timing each
  and reporting whether the two agree on the alarm decision. Shown on
  the dashboard as two timing bars side by side.
- **FFI control arm**: `smart_station_dsp` now also builds as a C ABI
  shared library (`smart_station_fft_magnitude`), exposing the exact
  same FFT code `magnitude_spectrum` already used. A new
  `benchmark_ffi.py` calls straight into that compiled library via
  `ctypes` and runs it through the same three-size, 30-repetition
  methodology as the other two arms — so a Rust-vs-Python timing gap
  can be separated from an algorithm-choice gap. Built, and smoke-tested
  end to end on a synthetic recording (mean time scaled sensibly across
  all three sizes, consistent with the pure-Rust arm plus a small,
  expected ctypes call overhead).
- Went through all ten NFRs against everything built today: most still
  hold by construction (fixed-size state, same style of doc comments,
  same local-network-only exposure); flagged two that are argued from
  design rather than freshly re-measured (the 5ms real-time budget, and
  flash footprint) as worth confirming with a real build/timing capture
  before leaning on them in the defense.
- Updated requirements.md: all 18 FRs now marked done: the 12 original
  core ones plus all 6 that were previously marked Stretch, now marked
  Adopted. Repackaged all three project zips with the new code.
**Oct 25, 2026 (evening)** — Flashed and ran everything from the pivot
work above on real hardware for the first time, then found and fixed
two real bugs surfaced by that run:
- Confirmed FR-13/14/17/18 on the real device: log output showing
  adaptive calm-mode windows actually skipping the FFT, the new `diag`
  command responding correctly, and the dashboard's Diagnostics panel
  showing live sampling mode, trend, and energy readings.
- **Spectrogram bug found and fixed**: the live spectrogram panel was
  showing as a flat, mostly-dark line with one red stripe. Root cause,
  confirmed from real hardware logs and screenshots: the on-device
  compression step included FFT bin 0 (DC), which on the Z-axis
  carries the ~1g constant gravity offset, a value vastly larger than
  any real vibration content. That one bin dominated the dashboard's
  per-frame auto-scaling and crushed every genuine bin to near-black.
  Fixed by excluding bin 0 from the compression, confirmed afterward
  by a real hardware screenshot showing genuine blocky vibration
  texture instead of a flat line.
- **New HD spectrogram added**: a second, laptop-computed view
  (`/api/spectro_hd`), recomputing a proper windowed, overlapping STFT
  (256-sample Hann window, 75% overlap, 32 bins) from the same raw
  `dump` buffer the Pico already exports, shown side by side with the
  original on-device view rather than replacing it.
- **Second, subtler version of the same bug, in the new HD path**:
  the HD panel rendered as a completely blank rectangle. Root cause:
  tapering a near-constant, gravity-heavy signal with a Hann window
  makes it non-constant over the window, so the window's own spectral
  shape (several bins wide, not a single spike) smears that huge DC
  term across the first several bins instead of just bin 0, defeating
  the earlier bin-0-only fix. Fixed by subtracting each window's own
  mean before applying the Hann window (standard detrending practice
  for any windowed FFT on real sensor data), confirmed by a follow-up
  screenshot showing real, distinct texture on both spectrograms
  side by side.
- **Dashboard styling fixes**: a header-duplication artifact seen in a
  printed/exported PDF of the dashboard traced to the sticky nav bar
  being re-anchored to the top of each page by the browser's print
  pagination (a print-only quirk, not a live-browsing bug), fixed with
  a `@media print` rule; and the panel layout reworked from a
  multi-column grid of small boxes into a single full-width column per
  request, so every panel spans the full page width.
