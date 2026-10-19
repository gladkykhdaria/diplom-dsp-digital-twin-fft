# Praesagium

A small embedded Digital Twin built on a Raspberry Pi Pico 2 W. It watches a
DC motor through two real sensors, runs an FFT on the vibration data
directly on the chip, and reacts on its own with LEDs, a buzzer, and the
motor itself when something looks wrong. The actual point of the project
is the research question underneath it: is Rust more efficient than
Python for real-time DSP, tested directly on real recorded hardware data,
on the same machine. This repo is the bachelor thesis project for that
question.

## Where to find what

- **idea-statement.md**: the project write-up sent to the supervisor.
  Research question, hardware, pin-out, wiring, and the questions about
  digital twin vs smart home and the benchmark methodology.
- **tech.md**: technical build log. What was added, what broke, how it
  got fixed, written as it happened.
- **requirements.md**: functional and non-functional requirements for the
  whole system, plus extra ideas being considered (STFT, feedback loop
  formalization, energy measurement) that aren't committed yet.
- **progress.md**: short dated log of what's actually been decided and
  built so far.

## Status

Core hardware and firmware are working: sensors, on-device FFT, automatic
alarm and motor control, web dashboard. Current open items are confirming
the vibration threshold holds across repeat sessions, exporting recorded
data from the Pico, and running the actual Rust vs Python benchmark.
