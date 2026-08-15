# SmartStation: Why This Project, In Plain Words

*A Digital Twin built to test whether Rust actually earns its keep for
real-time signal processing - and how a bachelor's thesis can prove it,
not just assert it.*

---

## 1. The plain-language version

Every machine that matters has sensors on it - a motor's vibration, a
room's temperature and humidity, a machine's current draw. The obvious
thing to do with that data is put it on a dashboard so a person can look
at it. Almost every "smart" system stops there. This project is built to
go one step further: read the signal, understand what it means, and act
on it automatically - light an LED, sound a buzzer, update a live web
view - without a human in the loop deciding what the data means each
time.

That distinction turns out to have a name, a real academic literature
behind it, and a testable claim attached to it. This document explains
what that claim is, why it's worth testing with Rust specifically, and
what the actual math and code have to do to prove it.

## 2. Why "Digital Twin" is a specific claim, not a buzzword

The phrase "Digital Twin" gets used for almost anything with a sensor
and a screen. The academic literature is much stricter about it, and the
strictness is useful because it forces a real design decision rather
than a marketing one.

The most-cited classification of this idea comes from
Kritzinger, Karner, Traar, Henjes, and Sihn's 2018 review, *Digital
Twin in Manufacturing: A Categorical Literature Review and
Classification*, published in IFAC-PapersOnLine. Their contribution
is a simple three-way split based purely on how data actually flows
between a physical object and its digital representation:

- **Digital Model** - a digital representation with *no* automatic
  connection to the physical object at all. A CAD drawing is a digital
  model. Nothing updates automatically in either direction.
- **Digital Shadow** - an automatic, *one-directional* flow: sensors
  push real data into the digital representation, so it always reflects
  current reality, but the digital side has no way to act back on the
  physical object. This is what most "smart dashboards" actually are.
- **Digital Twin** - a *bidirectional*, automatic loop: sensor data
  feeds the model, the model computes a decision, and that decision is
  sent back to change something physical.

This is worth taking seriously rather than skipping past, because it's
the exact design decision this project makes concrete: a dashboard that
only displays numbers is a digital shadow. It becomes a digital twin
specifically at the moment the software's own conclusion - "this
vibration pattern looks abnormal" - is what lights the red LED and
sounds the buzzer, not a person looking at a chart and deciding to
react. Grieves and Vickers, in their 2017 chapter on the concept,
frame the entire point of a digital twin as being able to
"mitigate" undesirable behavior in a complex system before it fully
manifests - which only makes sense if the digital side can act, not
merely observe (Grieves & Vickers, 2017; the term itself originates in
Grieves' 2014 white paper on virtual factory replication).

So the closed loop - sensors in, computed decision out, driving a real
LED/buzzer *and* a live web view - is not a nice-to-have feature. It's
the specific thing that makes the classification claim "this is a
digital twin" actually true instead of aspirational.

## 3. The research question underneath the hardware

The hardware (a Pico 2 W running the sensor loop, LEDs, a buzzer, a Wi-Fi
link to a web dashboard) is the demonstration vehicle. The actual
research question the thesis is built around is narrower and more
testable:

> Can a signal-processing pipeline written in Rust - a systems language
> with compile-time memory safety and no garbage collector - match or
> beat the throughput of Python's established NumPy/SciPy stack on
> identical workloads, while keeping the kind of predictable, low-jitter
> timing that a real-time control loop needs?

That's a claim that can be measured, not just argued. Measuring it
honestly requires being fair to both sides, which is where the math and
the fairness of the comparison both matter.

## 4. The math: why FFT is the right tool, and why it's fast

### 4.1 What the DFT actually computes

A vibration sensor gives a sequence of numbers over time -
$x_0, x_1, \dots, x_{N-1}$. On its own, a time-domain signal like this
doesn't directly show *which frequencies* are present in it, and a
developing mechanical fault typically shows up first as a new or
shifting frequency component, long before it's visible any other way.
The Discrete Fourier Transform converts the signal from the time domain
into the frequency domain:

$$
X_k = \sum_{n=0}^{N-1} x_n \, e^{-i 2\pi k n / N}, \qquad k = 0, 1, \dots, N-1
$$

Computed directly, exactly as written, this needs a full sum of $N$
terms for *each* of the $N$ output frequencies - $O(N^2)$ multiplications
in total. For a modest 1 million samples, that's on the order of
$10^{12}$ operations. That is not real-time-feasible on a
microcontroller, or arguably anywhere.

### 4.2 The Cooley-Tukey speedup

The fix is not a new idea from this thesis - it's sixty years old.
Cooley and Tukey's 1965 paper, *An Algorithm for the Machine
Calculation of Complex Fourier Series* (published in *Mathematics of
Computation*), showed that the DFT sum can be split recursively into
the DFTs of the even-indexed and odd-indexed samples:

$$
X_k = E_k + e^{-i 2\pi k / N} \, O_k
$$

where $E_k$ and $O_k$ are themselves $N/2$-point DFTs of the even and
odd subsequences. Applying this split recursively turns the complexity
from $O(N^2)$ into $O(N \log N)$. At $N = 1{,}000{,}000$:

$$
\frac{N^2}{N \log_2 N} = \frac{10^{12}}{\approx 2 \times 10^{7}} \approx 50{,}000
$$

That's the origin of the roughly 50,000&times; theoretical speedup this
project cites for large sample sizes - it's not a Rust-specific number,
it's what the *algorithm itself* buys over the naive approach,
regardless of what language implements it. This algorithm, unchanged in
its essential structure, is still what runs inside NumPy's and SciPy's
own FFT implementations today (Harris et al., 2020; Virtanen et al.,
2020) - which matters for the next section, because it means the
algorithmic advantage is not what's actually being tested.

## 5. Why the Rust-vs-Python comparison has to be built carefully

It would be easy - and dishonest - to write a slow, naive DFT loop in
Python, compare it to a fast Rust FFT, and call that a fair
benchmark. It wouldn't be. NumPy and SciPy do not run a Python
interpreter loop for their FFT calls; they dispatch into
highly-optimized, compiled C and Fortran routines (Harris et al., 2020;
Virtanen et al., 2020). A fair comparison has to pit the *whole
pipeline* against the whole pipeline - including orchestration, memory
handling, and I/O, which in a real deployment do run in the host
language - not cherry-pick the one part of Python that isn't actually
Python.

What's left to actually test, once the algorithm itself is held
constant, is real and specific to language design:

- **No garbage collector.** Rust's memory is freed deterministically
  when it goes out of scope, enforced entirely at compile time by the
  ownership and borrowing system. There is no unpredictable
  garbage-collection pause that could stall a control loop at the wrong
  moment - a real risk in managed, garbage-collected languages that
  Python's runtime does not eliminate.
- **Compile-time memory safety with zero runtime cost.** Jung, Jourdan,
  Krebbers, and Dreyer's 2018 paper *RustBelt: Securing the Foundations
  of the Rust Programming Language* formally verified Rust's core
  safety guarantees for a realistic subset of the language - proving,
  rather than merely asserting, that Rust "promises to overcome" the
  usual trade-off between low-level control and high-level safety
  (Jung et al., 2018). That verification is what justifies trusting
  Rust for this kind of use case at all, rather than treating "Rust is
  safe" as marketing.
- **Zero-cost abstractions.** High-level Rust constructs - iterators,
  generics, closures - compile down to code as efficient as the
  hand-written equivalent (Klabnik & Nichols, 2023), so writing
  readable code isn't supposed to cost throughput.

The actual thesis benchmark, then, is not "Rust vs. Python the
language" - it's the full Rust DSP pipeline (FFT, filters, anomaly
detection, running on real embedded hardware) against the full
NumPy/SciPy pipeline, at multiple sample sizes, measuring both raw
throughput *and* latency consistency, not just an average.

## 6. Why this matters from a coding/systems perspective

Beyond the raw numbers, there's a practical argument that only shows up
once you're actually writing the embedded firmware, not just benchmarking
on a desktop: a predictive-maintenance system has to run somewhere close
to the sensor, often on hardware with real memory and power constraints,
and it has to keep running correctly for long, unattended stretches.
Python's interpreter and runtime were never designed for that
environment. Rust compiles to a small, dependency-free binary that runs
directly on the microcontroller, with no interpreter, no separate
runtime process, and - because of the ownership system - no entire class
of memory bugs (use-after-free, data races, buffer overruns) that would
otherwise need to be hunted down by hand on hardware that's much harder
to debug than a desktop machine.

## 7. What the roadmap is actually proving, stage by stage

- **Signal fundamentals first** (a `Signal` struct, basic operations) -
  before any speed claims, the basic representation has to be right.
- **The FFT and filters, tested against synthetic signals with known
  frequency content** - correctness before performance, since a fast
  wrong answer is worse than a slow right one.
- **Real sensor hardware** - because synthetic test signals are clean
  in a way real vibration and thermal data never is, and the system has
  to survive that.
- **The benchmark itself**, at multiple sample sizes, against NumPy and
  SciPy specifically, because that's the actual research claim being
  tested.

## 8. Conclusion

The case for this project rests on three claims, each backed by a real
source rather than assumption: that the digital twin/digital shadow
distinction is a genuine engineering requirement, not a marketing label,
and meeting it requires a real, closed, bidirectional loop
(Kritzinger et al., 2018; Grieves & Vickers, 2017); that the FFT is the
mathematically correct tool for catching an early mechanical fault
signature, and its $O(N \log N)$ complexity is what makes doing that in
real time possible at all (Cooley & Tukey, 1965); and that Rust's
combination of compile-time memory safety and the absence of a garbage
collector (Jung et al., 2018) makes it a serious, *measurable* candidate
for this kind of latency-sensitive workload - tested directly against
the real Python numerical stack (Harris et al., 2020; Virtanen et al.,
2020), rather than just claimed.

---
# On date : 15.08.26
System right now looks like this:
<img width="1280" height="721" alt="image" src="https://github.com/user-attachments/assets/c4e7de42-b76d-4735-913d-38a745cff4c5" />
<img width="721" height="1280" alt="image" src="https://github.com/user-attachments/assets/6a84751e-e38f-4521-a982-5cea6fa1b579" />

---
## Sources cited

- Kritzinger, W., Karner, M., Traar, G., Henjes, J., & Sihn, W. (2018).
  Digital Twin in Manufacturing: A Categorical Literature Review and
  Classification. *IFAC-PapersOnLine*, 51(11), 1016-1022.
- Grieves, M., & Vickers, J. (2017). Digital Twin: Mitigating
  Unpredictable, Undesirable Emergent Behavior in Complex Systems. In
  F.-J. Kahlen, S. Flumerfelt, & A. Alves (Eds.), *Transdisciplinary
  Perspectives on Complex Systems* (pp. 85-113). Springer.
- Grieves, M. (2014). *Digital Twin: Manufacturing Excellence through
  Virtual Factory Replication* (White paper). Florida Institute of
  Technology.
- Cooley, J. W., & Tukey, J. W. (1965). An Algorithm for the Machine
  Calculation of Complex Fourier Series. *Mathematics of Computation*,
  19(90), 297-301.
- Harris, C. R., Millman, K. J., van der Walt, S. J., et al. (2020).
  Array Programming with NumPy. *Nature*, 585(7825), 357-362.
- Virtanen, P., Gommers, R., Oliphant, T. E., et al. (2020). SciPy 1.0:
  Fundamental Algorithms for Scientific Computing in Python. *Nature
  Methods*, 17, 261-272.
- Jung, R., Jourdan, J.-H., Krebbers, R., & Dreyer, D. (2018). RustBelt:
  Securing the Foundations of the Rust Programming Language.
  *Proceedings of the ACM on Programming Languages*, 2(POPL), Article 66.
- Klabnik, S., & Nichols, C. (2023). *The Rust Programming Language*.
  No Starch Press.
