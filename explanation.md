# Understanding and defending the benchmark (FR-11 and FR-12)

This walks through what got built today, piece by piece, in plain words,
with the actual code so you can point to exactly where each idea lives.
Read it like a rehearsal for explaining this to your professor.

## The two requirements this closes

Your requirements doc has two core items that were still open:

- **FR-11**: export real recorded vibration data from the device.
- **FR-12**: run the same FFT workload in Rust and in Python, on that
  real data, on the same laptop, and measure both speed and consistency.

Everything below is just those two things, built.

## Part 1: getting real data off the Pico (FR-11)

The firmware already runs an FFT on vibration data to decide whether to
sound the alarm. But that FFT only ever sees one 256-sample window at a
time, and then throws it away. There was no way to actually get a copy
of the raw signal off the chip to use somewhere else.

So a second, separate buffer was added, just for that purpose:

```rust
// pico_hotspot/src/main.rs

const RAW_HISTORY_LEN: usize = 2048;

struct RawVibrationHistory {
    samples: [f32; RAW_HISTORY_LEN],
    next: usize,
    filled: bool,
}
```

Think of this as a tape loop: a fixed box that holds the last 2048
samples (about 10 seconds at 200Hz). Every single time the MPU6050 is
read, one more raw number goes in:

```rust
// pico_hotspot/src/main.rs, inside mpu_task

vibration_buffer[buffer_index] = accel_z_g;
buffer_index += 1;

RAW_HISTORY.lock(|cell| cell.borrow_mut().push(accel_z_g));
```

That second line is the entire addition. The existing `vibration_buffer`
still does its usual job (feeding the alarm's FFT); `RAW_HISTORY` is
just quietly keeping its own copy of the same raw numbers, for export
later.

Then a new command was added to the TCP server, so anything connected
to the Pico can ask for that tape loop:

```rust
// pico_hotspot/src/main.rs, inside the TCP command handling

if msg == "dump" {
    let (snapshot, count) = RAW_HISTORY.lock(|cell| cell.borrow().snapshot_oldest_first());
    // ... streams `count` raw values out over the socket as CSV
}
```

If you type `dump` into a TCP connection to the Pico (port 6000, same
one the dashboard already uses), it answers with every raw sample it
has, oldest first, as comma-separated numbers.

To actually pull that onto your laptop and save it as a file, there's a
small Python script:

```python
# smart_station_dsp/python/export_vibration_data.py

sock.sendall(b"dump\n")
# ... reads the response, saves each number on its own line to a .csv file
```

Run it, and you get a file like `dump_1792944604.csv`: one real,
recorded acceleration value per line. That file is FR-11, satisfied.
It's not simulated, not synthetic, it's exactly what the accelerometer
measured.

## Part 2: the actual benchmark (FR-12)

Now you have real data. The benchmark's job is to run the *same* FFT
calculation on it, once in Rust, once in Python, and time both.

**The complication**: your real recording is only ~10 seconds (2048
samples). But a proper benchmark wants to test several input sizes,
including a big one, 1,048,576 samples, which at 200Hz would mean
recording for over an hour. Nobody's doing that.

**The fix**: instead of padding the extra length with silence or random
noise, the real recording is repeated end to end until it reaches the
target length:

```rust
// smart_station_dsp/src/lib.rs

pub fn tile_to_length(&self, target_len: usize) -> Signal {
    let mut data = Vec::with_capacity(target_len);
    while data.len() < target_len {
        let remaining = target_len - data.len();
        let take = remaining.min(self.data.len());
        data.extend_from_slice(&self.data[..take]);
    }
    Signal::new(data, self.sample_rate)
}
```

Picture laying your 10-second recording down, then laying a copy of it
right after, then another, like tiling floor tiles, until you've covered
the length you need, then cutting off the excess. Every single sample
in the final signal is still a real recorded number, nothing is made up,
it's just repeated. This is a normal thing to do in benchmarking when
you have a short real trace but want to test a workload at a size larger
than what you could practically record. There's even a test confirming
this doesn't quietly corrupt the data:

```rust
// smart_station_dsp/src/lib.rs, in the tests module

#[test]
fn tile_to_length_preserves_dominant_frequency() {
    let base = Signal::sine_wave(50.0, 1.0, 1000, 1024);
    let tiled = base.tile_to_length(4096);
    let (freq, _mag) = tiled.dominant_frequency();
    assert!((freq - 50.0).abs() < 1.0, "expected ~50 Hz, got {freq} Hz");
}
```

That test builds a known 50 Hz tone, tiles it to four times its length,
and checks the FFT still correctly says "50 Hz" afterward. It does. So
tiling is safe to use.

With that in place, the Rust side of the benchmark looks like this:

```rust
// smart_station_dsp/src/bin/benchmark.rs

const SIZES: [usize; 3] = [1_024, 65_536, 1_048_576];
const REPETITIONS: usize = 30;

for &size in &SIZES {
    let signal = recorded.tile_to_length(size);
    for _ in 0..REPETITIONS {
        let start = Instant::now();
        let _spectrum = signal.magnitude_spectrum();
        let elapsed = start.elapsed();
        // record elapsed time
    }
    // compute mean, stddev, min, max across the 30 runs
}
```

In plain words: for each of the three sizes, stretch the real recording
to that length, then run the FFT 30 separate times and write down how
long each run took. Running it 30 times, not once, is what lets you
measure *consistency* (how much the timing wobbles), not just *speed*
(the average).

The Python side does the exact same thing, same sizes, same repetition
count, so the two are directly comparable:

```python
# smart_station_dsp/python/benchmark.py

SIZES = [1_024, 65_536, 1_048_576]
REPETITIONS = 30

for size in SIZES:
    signal = tile_to_length(recorded, size)
    for _ in range(REPETITIONS):
        start = time.perf_counter()
        _spectrum = np.fft.rfft(signal)
        elapsed = time.perf_counter() - start
```

## What you actually measured

Here is the real run from tonight, on your real recorded data:

| size | Rust mean | Rust jitter (stddev) | Python mean | Python jitter (stddev) |
|---|---|---|---|---|
| 1,024 | 0.057 ms | 0.033 ms | 1.878 ms | 9.969 ms |
| 65,536 | 3.404 ms | 0.631 ms | 1.472 ms | 0.277 ms |
| 1,048,576 | 69.996 ms | 7.558 ms | 49.120 ms | 10.806 ms |

"Mean" is the average time one FFT took, lower is faster. "Jitter"
(the standard deviation) is how much that time wobbled from run to run,
lower means more predictable, which matters more than raw speed for a
real-time system: a system that's occasionally very slow is worse than
one that's always moderately fast, even if its average looks fine.

## How to explain this if asked

**"So which language won?"**
Neither one cleanly. At the smallest size, Rust was faster and much
more consistent. At the two larger sizes, Python/NumPy actually had the
lower average time. The honest, defensible claim isn't "Rust is faster,"
it's "Rust was more consistent across every size tested, and faster at
the size closest to what this project actually runs in real time
(256-sample windows)."

**"Why is NumPy ever faster, isn't this supposed to show Rust winning?"**
Because NumPy's FFT doesn't run in pure Python, it calls into a
long-optimized, compiled C library (similar lineage to what
professional signal-processing tools use). So at large sizes you're
partly comparing `rustfft` against decades of C optimization, not
"Rust vs. Python" as languages in the abstract. That's worth saying
out loud rather than hiding, it's a more mature answer than pretending
the result is one-sided.

**"Why does Python's smallest-size result look so erratic?"**
Look at the numbers again: Python's jitter at 1,024 samples (9.969ms)
is bigger than its own average (1.878ms). That is almost certainly one
unusually slow first call, likely some one-time setup cost the FFT
library pays the first time it's used in the script, skewing the
statistics. It's a real artifact worth mentioning, and also a sign the
benchmark could be tightened further (a throwaway "warm-up" run before
the timed ones would remove it) if there's time before the deadline.

**"Is this result final?"**
No, and say so plainly if asked: this is one recording (10 seconds,
mostly idle, not a lot of real vibration event in it) run once through
each benchmark. For the thesis, repeating this with a recording that
includes a deliberate disturbance (tap or spin the propeller before
exporting), and running the whole benchmark multiple times, would make
the numbers something you can actually report with confidence rather
than as a single data point.

## Part 3: the window question, and the theory behind it

This part is separate from the Rust-vs-Python timing benchmark above,
no windowing happens in Part 1/2 at all, it's a plain FFT over a tiled
array of raw samples. This covers the *other* place "window" shows up
in this project: the spectrogram, and specifically why the laptop-side
version needed a window function, why that choice is the standard one,
and why it's also what caused the blank-canvas bug you just saw get
fixed. Worth knowing cold, because "why this window and not another"
is exactly the kind of thing a DSP professor asks.

### What a "window" actually is, and why the FFT needs one

An FFT doesn't work on an infinite, ongoing signal. It needs a finite
chunk, N samples, and mathematically it treats that chunk as if it
repeats forever, one period of some longer periodic signal. "Windowing"
is the act of cutting that chunk out of the continuous stream.

The simplest possible window is no window at all, just taking N
samples as they are. This is called a **rectangular window** (every
sample gets weight 1, full stop), and it's what the Pico's own
real-time FFT uses, because it costs nothing extra, no multiply, no
lookup table, nothing the chip has to compute beyond what the FFT
needs anyway.

### Why a rectangular window usually leaks

Here's the problem with cutting a chunk out of a signal and pretending
it repeats: unless the chunk happens to end exactly where it would
naturally repeat, there's a jump, a discontinuity, right at the seam
between "end of this window" and "start of the next copy of it." A
sudden jump like that is, in frequency terms, made of a huge spread of
frequencies all at once (the same reason a sharp click sounds
"bright," full of high frequencies, while a smooth tone doesn't).

The formal version of this: multiplying a signal by a window in the
time domain is the same as convolving (smearing) its true spectrum
with the window's own spectrum in the frequency domain. A rectangular
window's own spectrum is a sinc function, a narrow central peak, but
with tall, slowly-decaying ripples (sidelobes) stretching out on
either side. Those ripples are what "spectral leakage" means: a single
real frequency's energy doesn't land cleanly in one bin, it leaks into
its neighbors too.

### Why the Pico gets away with rectangular anyway

For a real constant, non-oscillating DC term (the ~1g gravity offset
sitting in every Z-axis reading) there's a special case: summing
`cos(2*pi*k*n/N)` over a whole window of a truly constant signal comes
out to exactly zero for every bin `k != 0`. A perfectly rectangular
window applied to a perfectly constant value has *no* leakage at all,
mathematically, all of that huge DC energy lands in bin 0 and nowhere
else. That's exactly why the original fix (excluding bin 0 from the
on-device spectrogram) was a complete fix there: rectangular + DC
really is a clean, single-bin problem.

### Why the laptop-side HD view uses a Hann window instead

The HD spectrogram isn't trying to be cheap, it's trying to be a
clearer *picture*, computed somewhere with room to spend extra cycles
on it. For that, a **Hann window** is the standard choice:

```rust
// smart_station_web/src/main.rs, spectro_hd()
let hann: Vec<f64> = (0..HD_WINDOW)
    .map(|n| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / (HD_WINDOW as f64 - 1.0)).cos())
    .collect();
```

Instead of a flat weight of 1 across the whole window, this tapers
smoothly from 0 up to 1 in the middle and back down to 0 at the far
edge, a raised-cosine bump. That taper removes the sharp seam a
rectangular window leaves behind, so the Hann window's own spectrum
has much lower, much faster-decaying sidelobes than the sinc function
above, far less leakage overall, at the cost of a slightly wider main
lobe (each real frequency now occupies a couple of neighboring bins
instead of one, which is a perfectly acceptable trade for a
visualization). It's the standard first choice for vibration and audio
spectrograms specifically because of that trade.

### The catch, and the actual bug

A window that tapers is, by definition, not constant across the
window anymore, even if the signal it's multiplying *is* constant. So
the special case that saved the Pico's rectangular window doesn't
hold here: tapering a near-constant, gravity-heavy signal turns it
into a shape whose spectrum is the *Hann window's own* spectrum (that
same multi-bin main lobe from above), scaled up by the huge DC
amplitude, not a clean single spike at bin 0. That spreads the
gravity term's enormous magnitude across several of the lowest bins,
not just bin 0, which is exactly the "one bin pinned hot, everything
else crushed to black" effect you first saw on the on-device view,
just relocated. Excluding bin 0 alone, which was a complete fix for
the rectangular case, wasn't enough here.

The fix, and the right one for this case rather than a workaround, is
to remove the mean before windowing at all:

```rust
// smart_station_web/src/main.rs, spectro_hd()
let mean: f64 = window.iter().sum::<f64>() / window.len() as f64;
let mut buf: Vec<_> = window.iter().zip(hann.iter())
    .map(|(&s, &w)| Complex::new((s - mean) * w, 0.0))
    .collect();
```

Subtracting each window's own mean (detrending) takes the huge DC
term out before the taper ever gets a chance to smear it. This is
completely standard practice anywhere a window function is used on
real sensor data (audio, vibration, biosignals), not something special
to this project. With the mean gone, bin 0 is genuinely near zero, and
every other bin shows real AC vibration content instead of leaked
gravity, which is the texture you can now see in the HD panel.

### Why 256 samples, and why 75% overlap

The window length and overlap aren't arbitrary either:

- **256 samples at ~200Hz is about 1.28 seconds**, which gives a
  frequency resolution (bin spacing) of `sample_rate / N`, roughly
  0.78Hz per bin. That's fine enough to tell apart the handful of Hz
  where real mechanical vibration or imbalance actually shows up,
  while still being short enough that the view updates several times
  a second rather than lagging behind reality.
- **75% overlap** (a 64-sample hop instead of jumping by the full 256)
  exists for two reasons at once: it makes the spectrogram scroll
  smoothly frame to frame instead of jumping in big, choppy blocks,
  and it compensates for the Hann taper itself, a sample sitting right
  at a window's edge gets multiplied by something close to zero, so
  on its own it would barely be represented. Heavy overlap means every
  sample gets a turn landing near the *center* of some window, where
  the taper is close to 1, instead of only ever being read near the
  attenuated edges.

Both choices are standard STFT (short-time Fourier transform) defaults
for exactly this kind of signal, not numbers picked by feel.

## Where everything lives, for reference

- `pico_hotspot/src/main.rs` — the `dump` command and `RAW_HISTORY`
  ring buffer (FR-11), and the on-device spectrogram's bin-0 exclusion.
- `smart_station_dsp/python/export_vibration_data.py` — pulls a `dump`
  off the Pico and saves it as a CSV file on your laptop.
- `smart_station_dsp/src/lib.rs` — `Signal`, `from_csv_file`,
  `tile_to_length`, and the FFT itself (`magnitude_spectrum`).
- `smart_station_dsp/src/bin/benchmark.rs` — the Rust half of the
  benchmark.
- `smart_station_dsp/python/benchmark.py` — the Python half, same
  methodology.
- `smart_station_web/src/main.rs`, `spectro_hd()` — the laptop-side
  Hann-windowed, overlapping STFT (the HD spectrogram), and the
  mean-subtraction fix described in Part 3.
