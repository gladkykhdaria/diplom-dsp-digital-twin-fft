# Performance Evaluation of Rust versus Python for Real-Time Digital Signal Processing in Embedded Digital Twin Applications

*Project name: **Praesagium** — from Latin, "foretelling, premonition,
foreknowledge." Sensing that something is about to go wrong, before it
actually does.*

## What this thesis actually is

This is not only a build project. The goal isn't just to make a
working Digital Twin - it's to answer a real, testable question:
**is Rust actually more efficient than Python for this kind of
real-time signal processing task, or not?**

Because during my study at uni I heard so much differnt opinions bout Python and Rust languages, so I was genually interested to check that by myself.

The system itself (sensors, FFT, anomaly detection, physical response)
exists so that question has something real to be measured on. The
same FFT workload will be run on both languages, on the same machine,
at multiple sizes, and timed and compared directly - throughput and
timing consistency (jitter), not just a single speed number. The build
is the experiment; the comparison is the actual research contribution.

## What the project does, in plain words

A small embedded system (a Raspberry Pi Pico 2 W) that watches a
machine using real sensors, looks for warning signs in the data using
signal processing (an FFT), and reacts on its own - lights, a buzzer,
and a motor - when something looks wrong. It's a small, working example
of a "Digital Twin": a system that doesn't just show you numbers, it
understands them and acts.

## Why I'm doing this

I came across Rust a while ago and got genuinely interested in it. I
never became a professional at it, but I like the language a lot -
mainly because of how much it cares about safety and efficiency, in a
way that's hard to accidentally get wrong. I knew early on that I
wanted my thesis to actually use it, not just study it.

Separately, I took a Digital Signal Processing course at university.
It wasn't my best semester grade-wise, but the subject itself genuinely
pulled me in, and I kept thinking about it afterward, which is usually
a better sign for me than the grade is.

Those two interests didn't really connect into one idea until this
summer, at a DTU summer school, where I learned about Digital Twins for
the first time and got to help build one. That's when it clicked -
Rust, DSP, and Digital Twins weren't three separate interests anymore,
they were one actual project. I started building it for real in July.

Liking Rust isn't a reason to write a thesis, though - so rather than
just assuming it's the better choice, the actual point of this project
is to test that assumption directly, against Python, on real hardware,
with real numbers.

## What actually runs on the Pico

The Pico runs firmware I wrote in Rust, using the `embassy` framework,
which lets the chip do several things at once (read sensors, run a web
server, run the math) without needing a full operating system
underneath it. This matters because the chip is small - it has no OS,
just the code I write running directly on the hardware.

In plain terms, the firmware:
- Reads two real sensors continuously (temperature/humidity/pressure,
  and vibration)
- Runs a real FFT (a way of turning a vibration signal into "which
  frequencies are present in it") directly on the chip, in real time
- Decides, based on that data, whether something looks wrong
- Turns on LEDs, a buzzer, and a motor automatically when it does
- Talks to a small web dashboard over Wi-Fi, so you can see it live

**On memory:** the Pico only has 520KB of memory total - no gigabytes
to spare like a laptop. So every part of the code has to fit
deliberately: the part that runs all the tasks at once uses about
96KB, the FFT window uses a small fixed amount (well under 20KB), and
the rest is free for everything else. Nothing is ever created and
thrown away at runtime the way a normal program might - everything has
a fixed, known size decided in advance. That's a real constraint of
embedded programming, and working within it was part of the point.


## Questions I need to be able to answer clearly
 
**1. Is this actually a Digital Twin, or just a digital shadow?**
 
The real distinction (Kritzinger et al.'s classification, the standard
one in this field) comes down to one thing: does data flow *both*
ways, automatically?
 
- A **digital model** is manual in both directions - someone builds a
  simulation by hand, no live data feeds it.
- A **digital shadow** is one-way - the real object sends data to its
  digital copy automatically, but nothing flows back. A dashboard that
  shows you live sensor readings is a shadow, not a twin.
- A **digital twin** is two-way and automatic - the real object sends
  data in, *and* the digital side sends decisions back out, without a
  person in the loop.
This system qualifies specifically because of the second half: the
FFT doesn't just display the vibration data, it computes a real
decision from it, and that decision directly drives the physical
motor, buzzer, and LEDs - automatically, with nobody clicking a
button. That closed loop is the actual requirement, and it's also the
one part of "twin" terminology that's easy to claim without actually
having - so this is the specific thing I need to be ready to point at
concretely if asked, not just assert.
 
**2. How is this different from a normal smart-home device?**
 
A smart-home sensor reads a value and shows it to a person - a
temperature sensor shows "24°C," and a human decides what that means
and what to do about it. The decision-making happens in your head, not
in the device.
 
This system is different in a specific, technical way: it doesn't
react to a raw number crossing a threshold, it computes the *frequency
content* of a signal over time (the FFT) and reacts to patterns in
that - the kind of thing that reveals a developing mechanical problem
long before a simple "is this number too high" check would catch
anything unusual at all. The decision is computed, not read off a
dial, and it's the computed decision - not a person - that triggers
the physical response. That's the real difference: not that it has
sensors and outputs (a smart plug has that too), but *what kind of
reasoning* sits between them.
 
**3. How will the Rust vs Python comparison actually be run? Python
definitely isn't running on the Pico, right?**
 
Correct - Python cannot run on the Pico at all, for two separate
reasons, not just one: the firmware is `no_std` (bare-metal, no
operating system underneath it, which a Python interpreter needs), and
even if that weren't true, the Pico's 520KB of memory is nowhere near
enough to hold a Python interpreter plus NumPy.
 
So the comparison isn't "Rust on the Pico vs Python on the Pico" - it's
structured as two separate, deliberately different jobs:
 
- **On the Pico (Rust only, `microfft`):** proves the system can run
  in real time on genuinely constrained embedded hardware at all. This
  is part of the argument for Rust, but it isn't the benchmark itself
  - there's nothing to compare it against on the same chip, since
  Python was never a candidate for running there.
- **The actual benchmark (laptop, both languages, same machine):** the
  identical FFT workload, implemented in Rust (`rustfft`) and Python
  (NumPy/SciPy), run on the *same* laptop hardware, at several input
  sizes, timing both throughput and timing consistency (jitter - how
  much the timing varies run to run, not just the average speed).
Running both languages on the same machine is the methodologically
important part: if I compared Rust-on-a-150MHz-microcontroller against
Python-on-a-laptop, I'd be measuring a hardware difference, not a
language difference, and the comparison would prove nothing about Rust
vs Python at all. Keeping the hardware identical and only changing the
language is what makes the result actually mean something.
 

## Hardware used

| Component | Role | Datasheet |
|---|---|---|
| Raspberry Pi Pico 2 W (x2 - one target, one debug probe) | Main board + programmer | <img width="120" src="https://github.com/user-attachments/assets/104795cc-58ba-49f5-9e9a-8761364fb80f" /> |
| BME280 | Temperature, humidity, pressure sensor | <img width="120" src="https://github.com/user-attachments/assets/c17676cd-e378-4dcc-98ef-a2bdaa96bdf5" /> |
| MPU-6050 (GY-521) | Accelerometer + gyroscope (vibration sensing) | <img width="120" src="https://github.com/user-attachments/assets/3fa81646-f624-4a9a-9012-884b2b5eb411" /> |
| 6x LEDs (red/yellow/green x2) | Status indicators | |
| 1x active buzzer | Alarm sound | |
| DC motor + propeller | The "machine" being monitored, and the physical response | |
| IRLZ34N MOSFET | Lets the Pico switch the motor on/off | <img width="120" src="https://github.com/user-attachments/assets/0252dd16-27b0-4f3c-85f0-ed72b022c952" /> |
| 1N4007 diode | Protects the MOSFET from the motor's voltage spike | <img width="120" src="https://github.com/user-attachments/assets/f87f2dfa-93f6-4842-ade7-09da71bf9184" /> |
| Resistors (various) | Current-limiting for LEDs, gate resistor | |

## Wiring - pin reference

| Signal | Pico pin | Physical pin # |
|---|---|---|
| BME280 / MPU-6050 - SDA | GP4 | 6 |
| BME280 / MPU-6050 - SCL | GP5 | 7 |
| BME280 / MPU-6050 - VCC | 3V3(OUT) | 36 |
| BME280 / MPU-6050 - GND | any GND | 38 |
| Temp LED - green | GP12 | 16 |
| Temp LED - yellow | GP13 | 17 |
| Temp LED - red | GP14 | 19 |
| Humidity LED - green | GP9 | 12 |
| Humidity LED - yellow | GP10 | 14 |
| Humidity LED - red | GP11 | 15 |
| Buzzer | GP15 | 20 |
| Motor (via MOSFET gate) | GP18 | 24 |

Both sensors share one I2C bus (I2C0) at different addresses (BME280 =
`0x76`, MPU-6050 = `0x68`), rather than needing two separate buses.

## Wiring diagram

Here you can see my struggle to do it in KiCad. 
That`s why I also did draw it by hand first - in order to establish inmy had how it should looks like. But in Kicad this prt is still 'in progress.'


<img width="507" height="640" alt="image" src="https://github.com/user-attachments/assets/436758b3-bc9d-45d0-932b-dec3c2e29286" />

<img width="1040" height="728" alt="Снимок экрана 2026-09-03 144846" src="https://github.com/user-attachments/assets/dd926b5d-b782-4dcb-b3da-7e69b52a74f0" />

## What the hardware actually looks like right now
<img width="721" height="1280" alt="image" src="https://github.com/user-attachments/assets/3665196d-82e1-46a1-8a92-2444ae5de0b9" />


## What I'm working on right now

The full pipeline, end to end - sensors in, decision made on-device,
physical response out:


<img width="1360" height="1000" alt="image" src="https://github.com/user-attachments/assets/9b0f5b2b-e735-48a7-8121-e6760123c0eb" />

Right now I'm specifically working on the last piece of that pipeline:
turning the MPU-6050's vibration data into a real, working anomaly
detector - so the system isn't just reacting to temperature and
humidity, but genuinely watching for the kind of irregular vibration
that would signal a real mechanical problem, and calibrating exactly
how sensitive that detection should be using real measured data instead
of guesses.

 
## Development pipeline - tasks in order
 
| # | Task | Status |
|---|---|---|
| 1 | Signal struct + basic operations (add/scale/normalize) | Done |
| 2 | FFT, desktop side (`rustfft`), correctness tested against synthetic tones | Done |
| 3 | FFT, on-device (`microfft`), verified on real RP2350 hardware | Done |
| 4 | Real sensor integration - BME280 + MPU-6050 over shared I2C | Done |
| 5 | Motor + MOSFET drive stage wired and debugged | Done |
| 6 | Vibration FFT running on-device in real time (200Hz sampling, 256-sample window) | Done |
| 7 | Anomaly threshold set from one real calibration run (0.735-51.44 -> 77.0) | In progress - needs repeat sessions to confirm it's stable, not a one-off |
| 8 | Confirm the anomaly detector doesn't get stuck re-triggering on its own motor vibration | In progress |
| 9 | Low/high-pass filters, written from scratch | Next |
| 10 | Rust vs Python benchmark - same FFT workload, both languages, multiple sizes | Next |
| 11 | Draft thesis chapters based on the results | Next |
 



