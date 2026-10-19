# Praesagium - Requirements Specification

What the system has to do, how it has to behave, and a few extra ideas worth
considering. The extra ideas are kept separate from the core scope so it's
clear nothing there is committed yet.

## 1. Functional requirements

Core requirements are the committed scope. Stretch rows are optional and
come from the ideas further down; they only become "required" if adopted.

| ID | Requirement | Status |
|---|---|---|
| FR-01 | Continuously read temperature, humidity, and pressure from the BME280 sensor. | Core |
| FR-02 | Continuously sample acceleration and gyroscope data from the MPU-6050 at a fixed 200 Hz rate. | Core |
| FR-03 | Compute a real-time FFT over fixed-size, 256-sample vibration windows directly on-device. | Core |
| FR-04 | Classify temperature and humidity readings into three status levels (green, yellow, red). | Core |
| FR-05 | Detect vibration anomalies by comparing spectral AC energy against a calibrated threshold. | Core |
| FR-06 | Combine temperature, humidity, and vibration alarm states into one system-wide alarm signal. | Core |
| FR-07 | Drive the LEDs, buzzer, and motor automatically from the combined alarm state, with no manual control path. | Core |
| FR-08 | Provide a calibration mode that derives the vibration threshold from real recorded idle vs. disturbance data, not a guess. | Core |
| FR-09 | Serve live sensor and status data over a TCP protocol on the local network. | Core |
| FR-10 | Provide a web dashboard that displays live readings and system status over Wi-Fi. | Core |
| FR-11 | Export real recorded MPU-6050 vibration data from the device for use outside the firmware. | Core |
| FR-12 | Run an equivalent FFT workload in both Rust and Python on that same recorded data, on the same laptop, across multiple input sizes, measuring throughput and timing jitter. | Core |
| FR-13 | Support adaptive sampling: lower resolution while calm, higher resolution when risk rises. | Stretch |
| FR-14 | Maintain a rolling long-horizon baseline to separate gradual drift from a sudden spike. | Stretch |
| FR-15 | Apply hysteresis (separate on/off thresholds) to the alarm logic so it can't chatter around one boundary. | Stretch |
| FR-16 | Stream a compact STFT magnitude array to the dashboard and render it as a live spectrogram. | Stretch |
| FR-17 | Display an estimated energy-per-decision figure on the dashboard, live, next to the current reading. | Stretch |
| FR-18 | Provide a replay mode that runs the same recorded window through the Rust and Python FFT paths side by side and shows both timings and both decisions at once. | Stretch |

## 2. Non-functional requirements

How the system has to behave, not what it does. These are the constraints
that make it a real embedded project rather than a desktop simulation.

| ID | Category | Requirement |
|---|---|---|
| NFR-01 | Memory | Must operate within the Pico's 520KB RAM with no runtime heap allocation. Every buffer is fixed-size and known at compile time. |
| NFR-02 | Real-time | Sensor sampling and FFT computation must complete within the sampling interval (5 ms budget at 200 Hz) with no missed deadlines. |
| NFR-03 | Reliability | Firmware must run continuously, for hours, without a manual reset under normal operating conditions. |
| NFR-04 | Electrical robustness | Motor switching must not corrupt onboard communication or the debug link (flyback protection on the drive stage). |
| NFR-05 | Methodological validity | The language benchmark must hold hardware constant: same machine, same recorded dataset, varying only the language, so the result isolates a language effect. |
| NFR-06 | Fault tolerance | A failed sensor read must degrade safely rather than panic or produce undefined behavior. |
| NFR-07 | Maintainability | Code must be structured and documented well enough to support the thesis write-up and be independently reproduced. |
| NFR-08 | Responsiveness | The web dashboard must reflect sensor state in near real time under normal Wi-Fi conditions. |
| NFR-09 | Footprint | The compiled firmware image must fit the RP2350's flash with comfortable headroom for later changes. |
| NFR-10 | Scope | Network exposure is local-network only by design. It's not hardened for public internet exposure, which is out of scope for this thesis. |

## 3. Ideas worth considering

Four ideas that go beyond "it works." Each adds a genuinely new angle
instead of just polish, and each is sized to fit inside the Pico's
constraints. None of these are committed yet, they're just options.

### Explainable alarm trace

Cheapest one to add. Right now an alarm is just a boolean. This would log a
short, structured reason alongside it: peak frequency, how long it lasted,
how far over threshold it went, so the dashboard can show why it triggered
and not just that it did.

It turns a black-box threshold check into something that reads like an
actual diagnostic tool, and it's good material to show: "here's
what it saw," not just a red light. It's a small addition on top of the
existing FFT output, a few extra fields in a fixed-size struct, no new
sensing or heavy compute involved.

### Drift vs. spike distinction

A new research angle. Keep a rolling long-horizon baseline (a compact
running average and variance, not the full waveform history) alongside the
instant detector, so the system can tell a slow upward drift in baseline
vibration, the signature of gradual wear, apart from a sudden one-off
spike.

This is the real difference between "reacting to a threshold" and
"predictive maintenance," a distinction that's actually citable in
condition-monitoring literature, not just a bigger buffer. It only needs a
few extra running statistics in memory, not raw history, so it stays well
inside the existing memory budget.

### FFI control arm for the benchmark

Methodologically the sharpest of the four. Alongside pure Rust (rustfft)
and pure Python (NumPy/SciPy), add a third arm that calls the exact same
Rust FFT implementation from Python through a binding such as PyO3. Same
algorithm, same code, just a different host language calling it.

Without this arm, a Rust vs Python difference could just mean two
different implementations, not two different languages. This arm isolates
interpreter and call overhead specifically, which is a more defensible
methodology than most student benchmarks bother with. It's a moderate
amount of work: a small PyO3 wrapper around the Rust FFT crate, built as a
Python extension module. It's entirely a Stage 2, laptop-side addition and
touches nothing on the Pico.

### Adaptive, power-aware sampling

Ties directly to the energy angle. Sample vibration less often, or over a
shorter window, while everything looks calm, and automatically increase
resolution the moment risk rises, instead of running at a fixed
worst-case rate all the time.

This makes the system's own power profile part of its design, not just
something measured afterward, and it directly supports the energy-aware
pivot. It needs a small state machine
around the existing sampling task; the harder part is proving it's stable
and not oscillating between rates, which is actually a good thing to show
rigor on.

None of these four replace the core plan. They're additions that would sit
on top of it. The explainable alarm trace and the FFI control arm are the
cheapest to actually ship before the deadline; the other two are bigger
commitments.

## 4. Round two

That's what keeps "looks
complicated" compatible with "actually shippable."

### Hysteresis on the alarm logic (ties to the feedback loop)

Right now there's one threshold: cross it, alarm on; drop below it, alarm
off. Hysteresis uses two thresholds instead, a higher one to trigger and a
lower one to clear, so a signal sitting right at the boundary can't
flicker the alarm on and off every few samples.

This is a named, textbook control-systems concept (a Schmitt-trigger-style
switching rule).

### Live spectrogram on the dashboard (ties to STFT)

Once the firmware computes an STFT instead of a single FFT, stream the
resulting magnitude values to the web dashboard as they're produced and
draw them as a scrolling heatmap, frequency on one axis, time on the other,
color for intensity.

This is the single most "catchy," demo-ready thing on this whole list.
Watching a live spectrogram scroll by, with a visible stripe lighting up
the instant the motor is disturbed, reads as serious DSP work at a glance,
not just a number changing color. It's a moderate amount of work: sending
an already-computed array over the TCP link that already exists, plus a
canvas element and maybe 40 lines of JS on the dashboard side. No new
library is needed for a basic heatmap.

### Live energy-per-decision readout (ties to the power angle)

Alongside the live temperature, humidity, and vibration readout, show a
running estimate of energy spent per FFT decision, even a rough one, from
the RP2350's known current draw at its running clock speed, before any
INA219 or RAPL measurement gets built.

This makes "power-aware" visible on the actual running system in real
time, instead of only appearing later as an offline benchmark number. The
live system and the energy narrative become the same thing instead of two
separate pieces of the thesis. A rough estimate is cheap, just a constant
times a cycle count. A precise one needs the INA219 already listed as an
option, so the rough version can ship first and get upgraded later without
changing the dashboard.

### Side-by-side replay: Rust vs Python, live (ties to the whole pitch)

A replay mode that takes one recorded window from Stage 1 and runs it
through the Rust path and the Python path at the same time, showing both
results on screen together: a timing bar for each, and whether the two
actually agree on the decision.


---
Praesagium, requirements and ideas, 2026-10-15
