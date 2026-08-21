# SmartStation

**A Digital Twin, built from scratch, in Rust.**

---

## Why I picked this project

I wanted to build something where I could actually *feel* the difference
between "the code runs" and "the code is correct." A website that shows
some numbers is easy to fake. A system that reads a real sensor,
decides in real time whether something is wrong, and lights an actual
LED because of that decision - that's not fakeable. Either the timing is
right and it works, or it isn't and it doesn't. I wanted a project with
that kind of honesty built into it.

I'm a Computer Engineering student, and most of my coursework so far has
been "does this program produce the right output." This project is
different on purpose: it's "does this program produce the right output,
*on time*, on real hardware, talking to a real physical thing that
doesn't care about my deadlines." That's a much more interesting
problem, and it's the actual problem embedded and real-time engineers
deal with every day.

## Why Digital Twins, specifically

I could have built a simple IoT dashboard - sensor goes in, number shows
up on a screen. That's been done a thousand times and it's genuinely not
that interesting to me. What *is* interesting is the idea that the
system doesn't just show you data, it **acts on its own understanding of
that data** - the same loop real predictive-maintenance systems use in
industry, just small enough to fit on my desk. That closed loop -
sensor in, decision out, physical reaction - is what actually earns the
name "Digital Twin" rather than "dashboard with extra steps," and I
wanted to build something that earns that name for real, not just call
it one.

## Why Rust

Honestly? Because everyone told me Python is "obviously" the right
choice for this kind of signal processing, and I wanted to find out for
myself whether that's actually still true once you care about real-time
behavior on constrained hardware, not just "is it fast enough on my
laptop." Rust gives me memory safety without a garbage collector
sitting in the background deciding when to pause my program - which
matters enormously the moment your program's job is to react to
something within milliseconds. I wanted a project that puts that claim
to an actual test, with real numbers, not just an opinion I repeat
because I read it somewhere.

It also just clicked for me as a language. I like that the compiler
argues with me *before* my code ships instead of after it crashes in
front of someone.

## What it actually does

- Two real sensors feed live data into the system: a BME280
  (temperature, humidity, pressure) and an MPU-6050 (acceleration,
  gyroscope) - sharing one I2C bus properly, both reading correctly and
  continuously.
- The system classifies that data itself - green/yellow/red per
  parameter - and drives real LEDs and a buzzer based on its own
  judgment, not a person watching a chart.
- A Raspberry Pi Pico 2 W runs its own Wi-Fi hotspot and serves that
  data (and takes commands) over a simple network protocol.
- A Rust web app on the other end gives a live dashboard: real sensor
  readings, the system's status, and a way to simulate values to test
  every possible outcome, all on one consolidated view.
- Underneath all of that sits the actual research question: whether a
  Rust-based signal-processing pipeline (FFT and friends, for the
  vibration side of the project) can match or beat Python's NumPy/SciPy
  stack on identical workloads, while staying predictable enough for
  real-time use.

## Why it matters to me

Because I don't want to hand in a project where I just followed a
tutorial. I want to hand in a project where I hit a genuinely hard
engineering problem - real sensor data getting corrupted the moment
Wi-Fi turned on - and instead of hiding that or quietly working around
it, I chased it through CPU scheduling, timing analysis, electrical
noise, and hardware-level fixes, ruling things out one at a time like
an actual investigation, until it actually resolved: not a workaround,
a real root cause (a single-wire sensor protocol with zero tolerance
for shared hardware, replaced with a properly clocked I2C sensor
instead). That process, start to finish, is the part I'm proudest of,
more than any single feature working. It's the part that actually
taught me something.

Getting a system to genuinely fight me, and then figuring out exactly
why, is a much better story than everything working on the first try
would have been.

## Status

Fully working end-to-end on **real sensor data**. Both the BME280 and
the MPU-6050 read correctly and continuously, sharing one I2C bus,
simultaneously with Wi-Fi fully active - the earlier Wi-Fi conflict is
resolved, not just documented as a limitation. See `tech.md` for the
full, honest history of how it got there, including everything that
didn't work along the way.

See `idea.md` for the full theoretical case (Digital Twin literature,
the math behind the FFT, and the Rust-vs-Python argument), and
`tech.md` for the complete technical history.
