# SmartStation - Technical Progress Report

Digital Twin monitoring system, built in Rust. This document is a complete
record of the hardware, the software stack, and the debugging history to
date - including everything that failed along the way, not just what
worked, because the failures are as informative as the successes.

---

## 1. Hardware Inventory

### Boards
- **2x Raspberry Pi Pico 2 W** (RP2350, wireless). One flashed with
  `debugprobe_on_pico2.uf2` and permanently repurposed as an SWD
  debug probe/programmer. The other is the target board running all
  application firmware.
  <img width="728" height="654" alt="image" src="https://github.com/user-attachments/assets/78f6237b-204c-44a7-92e5-1582e461a9d4" />

- **2x full-size breadboards**, one hosting the two Picos and the debug
  wiring, one hosting the sensor and LED circuitry.

### Sensors tried
- **DHT22 (AM2302) temperature/humidity sensor** - the sensor currently
  in active use. Confirmed genuine, working hardware (validated via a
  raw protocol byte-dump early in the project, and again via a second,
  independent DHT22 unit behaving identically to the first).
  <img width="800" height="533" alt="image" src="https://github.com/user-attachments/assets/dc00dec1-c429-468c-a3ed-512a5bc4009c" />

- **BME280 temperature/humidity/pressure module (I2C)** - tried as an
  alternative sensor. Diagnosed as having a **dead temperature-sensing
  element**: the humidity channel worked and responded correctly to
  real physical stimuli (breath, finger contact), but the temperature
  channel returned a fixed, checksum-valid, out-of-range value
  (125.0 &deg;C) on every single read, thousands of times, including
  under direct thermal stimulus. Root cause was hardware, not wiring
  or code. Abandoned in favor of returning to the DHT22.
  <img width="807" height="758" alt="image" src="https://github.com/user-attachments/assets/6510d5cc-7bd3-41e7-a8fa-cf68aa8afd16" />

- **BME280 #2 (temperature/humidity/pressure, I2C)** - a second,
  independent unit, purchased after the DHT22-vs-Wi-Fi conflict was
  conclusively root-caused (Section 3.9) and I2C was identified as the
  structurally correct replacement. **Genuine, working hardware** -
  confirmed via physically plausible, correctly drifting readings
  (temperature, humidity, and pressure all responding to real ambient
  conditions), and now the project's live temperature/humidity/pressure
  source. See Section 3.12.

  - **MPU-6050 (GY-521) accelerometer/gyroscope, I2C** - the vibration
  sensor the FFT/DSP pipeline is actually built around. Confirmed
  genuine, working hardware via `WHO_AM_I` register readback and real
  physical response to motion (tilting/shaking). See Section 3.11.
  <img width="600" height="407" alt="image" src="https://github.com/user-attachments/assets/ffecbd50-fd46-4fc1-bd56-a8ad751f6cb1" />

### Display tried
- **2.4" ILI9341 SPI TFT display** - wired and driven correctly at the
  software/SPI level (confirmed via a raw display-ID register read over
  MISO). The backlight never lit under any wiring configuration,
  including direct battery power isolated from the rest of the circuit.
  Diagnosed as a **dead display unit** (backlight and/or panel).
  Abandoned; not currently part of the system.
  <img width="1048" height="909" alt="image" src="https://github.com/user-attachments/assets/2f98857a-3931-45eb-bcd2-afa865fbc7bd" />


### Indicators / actuators
- **6x LEDs** (2 sets of red/yellow/green), one set for temperature
  status, one set for humidity status, each wired through its own
  series resistor (~220&ndash;1k &Omega;).
- **1x active buzzer**, shared alarm output, fires when either
  temperature or humidity reaches the red level.

### Passive components
- **10nF ceramic capacitor** - added across the DHT22's VCC/GND pins as
  a high-frequency decoupling attempt.
- **100&micro;F electrolytic capacitor** - added in parallel with the
  ceramic, as a larger-capacity power-rail smoothing attempt, correct
  polarity confirmed.
- **Aluminum foil**, grounded to target GND via a jumper wire, tested as
  an improvised RF shield between the two boards. Abandoned; not currently part of the system.



## 1.5 Pin Reference (current, final wiring)
 
Every physical connection currently in use on the target Pico 2 W, as of
the working system described in this document. GPIO numbers are the
`GPxx` labels; physical pin numbers are the pin position counting around
the board, as printed on most Pico 2 W pinout diagrams.
 
### Debug probe &harr; target (SWD)
 
| Signal | Debug-probe Pico pin | Target Pico pin |
|---|---|---|
| SWCLK | GP2 | SWCLK |
| SWDIO | GP3 | SWDIO |
| GND | GND | GND |
 
### Power
 
| Signal | From | To | Physical pin (target) |
|---|---|---|---|
| Target power | Laptop USB &rarr; target's own USB port | Target VSYS | - |
 
(Both boards are powered independently via their own USB cables from
the laptop - not daisy-chained through the debug probe. See Section 3.9
for why a fully separate power source was tested and ruled out as a
factor.)
 
### I2C0 bus - shared by both sensors
 
One physical bus, two devices, distinguished by I2C address (BME280 =
`0x76`, MPU-6050 = `0x68`) via the shared-bus pattern described in
Section 2.
 
| Signal | Target Pico pin | Physical pin # |
|---|---|---|
| SCL | GP5 | 7 |
| SDA | GP4 | 6 |
 
**BME280:**
 
| BME280 pin | Target Pico pin | Physical pin # |
|---|---|---|
| VIN | 3V3(OUT) | 36 |
| GND | any GND | 38 |
| SCL | GP5 | 7 |
| SDA | GP4 | 6 |
 
**MPU-6050 (GY-521):**
 
| MPU6050 pin | Target Pico pin | Physical pin # |
|---|---|---|
| VCC | 3V3(OUT) | 36 |
| GND | any GND | 38 |
| SCL | GP5 | 7 |
| SDA | GP4 | 6 |
 
### Status LEDs (6x) + buzzer
 
| Signal | Target Pico pin | Physical pin # |
|---|---|---|
| Temperature - GREEN | GP12 | 16 |
| Temperature - YELLOW | GP13 | 17 |
| Temperature - RED | GP14 | 19 |
| Humidity - GREEN | GP9 | 12 |
| Humidity - YELLOW | GP10 | 14 |
| Humidity - RED | GP11 | 15 |
| Buzzer (shared alarm) | GP15 | 20 |
 
Each LED has its own series resistor (~220&ndash;1k &Omega;) between the
GPIO pin and the LED, cathode to GND. The buzzer's `+` goes to GP15,
`-` to GND.
 

---

## 2. Software Stack

### Firmware (target Pico 2 W)
- **Language:** Rust, `no_std` / `no_main` embedded target
  (`thumbv8m.main-none-eabihf`)
- **HAL/async runtime:** `embassy-rp` + `embassy-executor` +
  `embassy-time` + `embassy-net`, pinned to a specific git commit of the
  `embassy-rs/embassy` monorepo (`rev = 2e7a2b6`) for internal version
  consistency across the whole dependency graph.
- **Wi-Fi:** `cyw43` + `cyw43-pio`, driving the Pico W's onboard
  CYW43439 chip. The Pico runs as its own **Wi-Fi access point**
  (`SmartStation_Setup`), not as a client on an existing network.
- **Networking:** `embassy-net` with `smoltcp`, a raw TCP server on
  port 6000 implementing a small plain-text command protocol
  (`read`, `sim:<temp>:<humidity>`).
- **Sensor drivers, final design:** two real I2C sensors sharing one
  physical I2C0 bus (GP4=SDA, GP5=SCL), via the official `embassy`
  shared-bus pattern (`embassy-embedded-hal`'s
  `shared_bus::asynch::i2c::I2cDevice`, one `I2c` instance behind an
  async `Mutex`, each sensor gets its own handle): a **BME280**
  (`bme280-rs` crate, address `0x76`) for temperature/humidity/pressure,
  and an **MPU-6050** (hand-rolled register-level driver, address
  `0x68`) for acceleration/gyroscope. This replaced an earlier
  PIO-driven DHT22 approach entirely - see Section 3.9 for why.
- **Debug/logging:** `defmt` + `defmt-rtt`, viewed live via
  `probe-rs run` over the SWD debug probe.
- **Flashing/debugging tool:** `probe-rs`, talking to the debug-probe
  Pico over SWD.
  
### Web application (runs on the laptop, not the Pico)
- **Language/framework:** Rust, `axum` web server + `tokio` async
  runtime.
- **Pages:**
  - Home - project overview and Digital Twin theory explanation, plus
    a live "system connected / not reachable" status badge.
  - Control - manual LED on/off test controls (superseded once real
    sensor + classification logic was added).
  - Sensor Simulation - manual temperature/humidity input plus 9
    one-click presets covering every green/yellow/red combination, for
    testing the LED/buzzer logic without needing real sensor data.
  - Live Reading - polls the Pico for the current real sensor reading
    every 2 seconds and displays it.
- **Transport:** the web app acts as a bridge - it receives HTTP
  requests from the browser and forwards them as plain-text commands
  over a fresh TCP connection to the Pico's AP (`192.168.4.1:6000`)
  for each request.

### Reference/tooling
- Course-provided `embassy-lab-utils` crate (Wi-Fi init helpers) and a
  solved lab exercise (`exercise7_server.rs`) were used as the proven
  starting point for the AP + TCP server pattern.
- `probe-rs`, `elf2uf2`-style flashing considered as an alternative to
  bypass the debug session entirely (not ultimately used).

---

## 3. Debugging Journey

This is organized roughly chronologically, by the problem being chased
at each stage.

### 3.1 Initial SWD probe verification
- **Win:** confirmed the debug-probe Pico could flash and monitor the
  target Pico correctly, using a minimal blink + RTT logging test
  firmware (`rp235x-hal`-based, no Wi-Fi).
- **Fail &rarr; fix:** first build failed because `memory.x` was written
  using the older RP2040-style BOOT2 bootloader layout instead of the
  RP2350's metadata-block boot format. The chip's boot ROM silently
  rejected the flashed image (flash succeeded, nothing ever ran).
  Fixed by using the official `rp235x-hal` example `memory.x`.
- **Fail &rarr; fix:** a `CorePeripherals::take()` call used the wrong
  path (`pac::CorePeripherals` instead of `cortex_m::Peripherals`), and
  a missing `Clock` trait import broke `.freq()` calls. Both were
  one-line fixes.

### 3.2 First real DHT22 read (proof the sensor itself works)
- **Win:** using `rp235x-hal`'s `InOutPin` (true hardware open-drain
  emulation via output-enable override) and a hand-written bit-bang
  decoder, achieved a clean, correct DHT22 read - verified by dumping
  the raw 5 protocol bytes over RTT and confirming the checksum by
  hand, and by confirming the humidity reading changed in response to
  breathing on the sensor. This became the reference "known good"
  result for everything that followed.

### 3.3 ILI9341 display
- **Fail:** backlight never lit, under any wiring, including bare
  battery power directly on the backlight pins with nothing else
  connected. SPI-level communication was confirmed working via a raw
  display-ID register read. Diagnosed as dead hardware, not a wiring
  or software issue. Abandoned.

### 3.4 Wi-Fi hotspot + TCP + LED "carcase"
- **Win:** built the AP + TCP server + single test LED, based on the
  course's solved exercise. Debugged an early failure that turned out
  to be the laptop's Wi-Fi adapter not receiving a DHCP lease from the
  Pico's AP (the firmware doesn't run a DHCP server) - fixed by
  assigning the adapter a manual static IP.
- Expanded to a full 6-LED (temperature + humidity, red/yellow/green)
  plus buzzer system, driven first by simulated `sim:` values from the
  web app, to validate the classification/LED/buzzer logic
  independent of any real sensor.

### 3.5 BME280 attempt
- **Fail:** extensive debugging (I2C address bug fixed, an I2C bus
  scanner written to test in isolation) eventually converged on: the
  sensor's temperature channel is genuinely dead hardware. Humidity
  channel and I2C communication both worked correctly. Documented and
  abandoned in favor of returning to the already-proven DHT22.

### 3.6 DHT22 in the real (Wi-Fi-enabled) firmware - the long middle stretch
This was the most extensive debugging phase. A hand-rolled, CPU-driven
bit-bang DHT22 reader (ported from the working `rp235x-hal` version to
`embassy-rp`'s async `Flex` GPIO type) failed **100% of the time** once
Wi-Fi was active, despite being provably correct in isolation.
Hypotheses tested, in order, each ruled out in turn:

1. **Buffer/logging bugs in the diagnostics themselves** - found and
   fixed two real bugs (a trace buffer sized too small, and diagnostic
   logging calls happening inside the timing-critical section,
   corrupting the very measurements being taken). Fixing these
   revealed the underlying signal was clean for the first several bits
   every time, then reliably corrupted partway through.
2. **Wi-Fi stealing CPU time from the sensor read** - tested by
   running the sensor-read task on a genuine hardware
   interrupt-priority executor (`InterruptExecutor`), higher priority
   than Wi-Fi's own tasks. No change. **Ruled out.**
3. **RTT/debug-probe polling interference** - tested by stripping
   nearly all logging down to one line per attempt. No change.
   **Ruled out.**
4. **High-frequency electrical noise** - tested with a 10nF ceramic
   decoupling capacitor directly across the sensor's VCC/GND. No
   change. **Ruled out.**
5. **Low-frequency power-rail sag from Wi-Fi TX current spikes** -
   tested with a 100&micro;F electrolytic capacitor in parallel with the
   ceramic. No change. **Ruled out.**
6. **Bad physical sensor/wiring** - tested with two independent
   physical DHT22 units and multiple different GPIO pins/breadboard
   rows. Both units failed identically. **Ruled out.**

At this point, isolating the exact same CPU bit-bang code into a
**separate project with Wi-Fi entirely removed** showed it still failed
identically - proving the bug was not Wi-Fi-related at all, but a
genuine flaw in the CPU-driven bit-bang approach itself.

### 3.7 Switch to `embedded-dht-rs` + `OutputOpenDrain`
- **Win:** replacing the hand-rolled bit-bang with the community
  `embedded-dht-rs` crate, paired with `embassy-rp`'s dedicated
  `OutputOpenDrain` pin type (true hardware open-drain, rather than
  manually toggling a pin between output and input modes), fixed the
  CPU-driven approach completely - 18/18 consecutive successful reads
  with real, plausible, changing values, in isolation without Wi-Fi.
- **Fail:** folded into the real firmware with Wi-Fi active, this
  approach also failed 100% of the time. Adding retry logic did not
  help - Wi-Fi's background activity corrupted every single attempt,
  even across multiple quick retries.

### 3.8 Move to PIO (hardware-level sensor reading)
Given CPU scheduling had been conclusively ruled out (even at the
highest possible software priority) as the mechanism, the read was
moved entirely into **PIO** - a small, independent hardware block on
the RP2350 that samples GPIO pins with cycle-accurate timing, with no
CPU or scheduler involvement at all once started.

- Used **PIO1** specifically (Wi-Fi's `cyw43-pio` driver uses PIO0 -
  separate hardware blocks, no sharing).
- Ported a known-working DHT protocol PIO program (matching the design
  of the established `pico_dht` reference implementation) into
  `embassy-rp`'s PIO API.
- **Fail &rarr; fix (compile-time):** the `pio_asm!` macro's export path
  differed from documentation for the specific pinned crate version in
  use; resolved by importing it from the `pio` crate's re-export.
- **Fail &rarr; fix (design):** the PIO program as first written had no
  timeout - if the sensor didn't respond, a `wait` instruction blocked
  forever, hanging the whole read task silently. Fixed by wrapping
  reads in an async timeout, with an explicit state-machine reset
  (which re-jumps to the program's start) on timeout.
- **Fail &rarr; fix (timing tuning):** the bit-sampling delay needed
  empirical tuning (25 &rarr; 15 &rarr; 40 microseconds after a bit's
  rising edge) to reliably land between the sensor's real
  0-bit (~21&ndash;27&micro;s) and 1-bit (~69&ndash;73&micro;s) pulse
  widths. A hardware limit (PIO delay fields cap at 31 cycles) required
  slowing the PIO clock divider (1&micro;s/cycle &rarr; 2&micro;s/cycle) to
  reach a 40&micro;s real-world delay within that limit.
- **Win:** in isolation (no Wi-Fi), this produced reliable real
  readings - roughly every other 2.5-second cycle succeeding cleanly,
  zero checksum errors across dozens of reads, with a small added
  recovery delay after any failed attempt.
- **Fail (current, unresolved):** folded into the real firmware with
  Wi-Fi active, this **still fails 100% of the time**, and in a
  strictly worse way than the CPU-driven version - every failure is a
  full timeout with zero partial/checksum-mismatch reads at all,
  suggesting the sensor itself may not be responding, not just that
  the reading of it is being corrupted.

### 3.9 Current hypothesis and mitigation attempts (unresolved)
Since PIO removes CPU scheduling as a possible cause entirely, and the
failure is total rather than partial, the leading hypothesis has shifted
from CPU contention to **physical interference between the active
Wi-Fi radio and the sensor's single-wire signal** - either radiated
(through the air) or conducted (through a shared power supply).

- **Grounded foil RF shield** between the two boards: tested, no
  change. Reduces confidence in the radiated-noise hypothesis
  specifically (though the shield's exact positioning relative to the
  Wi-Fi antenna was not independently verified).
- **`cyw43` power-management mode** (`PowerManagementMode::SuperSave`,
  reducing how often the radio actively transmits) added as the next
  test - result pending. Caveat noted at the time: power-save modes
  are more typically a station/client-side feature, and may have
  limited effect on a device operating as its own access point, since
  APs generally must keep transmitting beacon frames on a fixed
  schedule regardless of power mode.
- **Not yet tested:** fully separate power supplies for the two boards
  (to isolate conducted power-rail noise from a shared USB source as a
  variable).


### 3.11 MPU-6050 integration
- **Win, immediate and complete:** wired up on I2C0, confirmed via
  `WHO_AM_I` register readback (`0x68`, the correct value) and real
  physical response - acceleration and gyroscope values changing
  correctly and returning to rest when the sensor was tilted/shaken.
  Expanded to read the sensor's full data block in one burst: all 3
  accelerometer axes, all 3 gyroscope axes, and the sensor's internal
  temperature, converted to real physical units (g, degrees/second,
  &deg;C) rather than raw register counts.
- **Win, decisive for the whole investigation:** folded into the real
  Wi-Fi-enabled firmware and left running continuously alongside every
  single DHT22 mitigation test in Section 3.9 (AP/station switching,
  power-save mode, AP close/reopen cycling, separate power supply). It
  never failed once - hundreds of consecutive successful reads across
  every test, unaffected by any of it. This is what made the
  single-wire-vs-clocked-protocol explanation concrete rather than
  theoretical: same board, same Wi-Fi, same disruptive conditions, and
  the only variable that mattered was which protocol the sensor used.
### 3.12 BME280 #2 integration and the final I2C bus-sharing lesson
- **Win:** the new BME280 unit, tested standalone (no Wi-Fi) first,
  produced clean, physically plausible, correctly drifting temperature/
  humidity/pressure readings from the very first attempt - confirming
  genuine working hardware this time.
- **Fail &rarr; fix (architecture):** the first attempt to fold it into
  the real firmware put it on a second, independent I2C bus (I2C1, a
  different GPIO pair from the MPU6050's I2C0), reasoning that separate
  hardware buses would avoid any possible interaction between the two
  sensors. This was the wrong instinct: BME280 failed to initialize on
  I2C1 every time, while the exact same BME280 code had just worked
  perfectly on I2C0 in isolation, and the MPU6050 kept working
  flawlessly on I2C0 the whole time. The wiring was double-checked and
  correct.
- **Fix:** rather than keep chasing the second bus, both sensors were
  moved onto the *same*, already-proven I2C0 bus, sharing it correctly
  via the official `embassy` shared-bus pattern
  (`embassy-embedded-hal`'s `shared_bus::asynch::i2c::I2cDevice`: one
  `I2c` driver instance behind an async `Mutex`, each sensor given its
  own handle into it, distinguished only by I2C address - BME280 at
  `0x76`, MPU6050 at `0x68`). This is precisely what I2C's bus design is
  for, and it worked immediately: both sensors initialize and read
  correctly, continuously, with Wi-Fi fully active.
- **Result:** the full Digital Twin loop is now genuinely closed with
  real data end to end - real BME280 and MPU6050 readings drive real
  LED/buzzer state and a real live dashboard, simultaneously with Wi-Fi
  running normally (no AP-cycling workaround needed at all, since I2C
  never required one).
---
---

## 4. Current Status Summary
 
| Component | Status |
|---|---|
| SWD debug probe setup | Working, reliable |
| Wi-Fi AP + TCP server + web app bridge | Working, reliable |
| LED (6x) + buzzer classification logic | Working, reliable, now driven by real sensor data |
| BME280 (temperature/humidity/pressure) | **Working, reliable, real data, alongside Wi-Fi** |
| MPU-6050 (acceleration/gyroscope) | **Working, reliable, real data, alongside Wi-Fi** |
| DHT22 sensor | Retired - see Section 3.9 for the full, conclusive root-cause finding |
| BME280 #1 (first unit) | Abandoned - confirmed dead temperature channel (hardware fault) |
| ILI9341 display | Abandoned - confirmed dead backlight/panel |
| Web dashboard (single consolidated view, blue palette) | Working, reliable, real live data |
 
The Digital Twin loop is now genuinely closed end to end: real sensor
data (BME280 + MPU6050) drives real classification logic, which drives
real physical LEDs/buzzer and a real live dashboard, continuously,
simultaneously with Wi-Fi running normally. The DHT22-vs-Wi-Fi
investigation that occupied most of Section 3 concluded with a real,
specific, protocol-level root cause (Section 3.9) and a structural fix
(switching to clocked I2C sensors) rather than a workaround - which is
itself a genuine, citable finding about single-wire sensor protocols
and concurrent radio hardware on this class of board.
 
 
