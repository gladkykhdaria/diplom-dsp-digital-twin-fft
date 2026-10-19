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

**Oct 19, 2026** — Resolved a GitHub push permission issue (stale
cached credential for a second GitHub account blocking pushes to the
`hotspot` branch) — tooling fix, not a project decision. (again me and my stupidity :o)

---
