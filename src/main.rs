
#![no_std]
#![no_main]

use core::cell::RefCell;

use bme280_rs::{AsyncBme280, Configuration, Oversampling, SensorMode};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{Ipv4Address, Ipv4Cidr, StackResources, tcp::TcpSocket};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{self, Config as I2cConfig};
use embassy_rp::peripherals::I2C0;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_async::i2c::I2c;
use embedded_io_async::Write;
use heapless::Vec;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use defmt::*;

mod irqs;
use irqs::Irqs;

const SOCK: usize = 4;

const AP_SSID: &str = "SmartStation_Setup";
const AP_PASSWORD: &str = "smartstation123";

const TEMP_GREEN_LOW: f32 = 18.0;
const TEMP_GREEN_HIGH: f32 = 28.0;
const TEMP_YELLOW_LOW: f32 = 10.0;
const TEMP_YELLOW_HIGH: f32 = 35.0;

const HUM_GREEN_LOW: f32 = 30.0;
const HUM_GREEN_HIGH: f32 = 60.0;
const HUM_YELLOW_LOW: f32 = 20.0;
const HUM_YELLOW_HIGH: f32 = 70.0;

static RESOURCES: StaticCell<StackResources<SOCK>> = StaticCell::<StackResources<SOCK>>::new();

static SIM_OVERRIDE: Signal<CriticalSectionRawMutex, (f32, f32)> = Signal::new();

#[derive(Clone, Copy, PartialEq)]
enum Level3 {
    Green,
    Yellow,
    Red,
}

impl Level3 {
    fn as_str(self) -> &'static str {
        match self {
            Level3::Green => "green",
            Level3::Yellow => "yellow",
            Level3::Red => "red",
        }
    }
}

fn classify_temp(t: f32) -> Level3 {
    if t < TEMP_YELLOW_LOW || t > TEMP_YELLOW_HIGH {
        Level3::Red
    } else if t < TEMP_GREEN_LOW || t > TEMP_GREEN_HIGH {
        Level3::Yellow
    } else {
        Level3::Green
    }
}

fn classify_humidity(h: f32) -> Level3 {
    if h < HUM_YELLOW_LOW || h > HUM_YELLOW_HIGH {
        Level3::Red
    } else if h < HUM_GREEN_LOW || h > HUM_GREEN_HIGH {
        Level3::Yellow
    } else {
        Level3::Green
    }
}

fn apply_level(level: Level3, green: &mut Output, yellow: &mut Output, red: &mut Output) {
    green.set_level((level == Level3::Green).into());
    yellow.set_level((level == Level3::Yellow).into());
    red.set_level((level == Level3::Red).into());
}

#[derive(Clone, Copy, Default)]
struct Reading {
    temperature: f32,
    humidity: f32,
    pressure: f32,
    have_reading: bool,
}

static LATEST: Mutex<CriticalSectionRawMutex, RefCell<Reading>> =
    Mutex::new(RefCell::new(Reading {
        temperature: 0.0,
        humidity: 0.0,
        pressure: 0.0,
        have_reading: false,
    }));

const MPU6050_ADDR: u8 = 0x68;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_ACCEL_XOUT_H: u8 = 0x3B;

#[derive(Clone, Copy, Default)]
struct MpuReading {
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,
    temp: f32,
    have_reading: bool,
}

static LATEST_MPU: Mutex<CriticalSectionRawMutex, RefCell<MpuReading>> =
    Mutex::new(RefCell::new(MpuReading {
        accel_x: 0.0,
        accel_y: 0.0,
        accel_z: 0.0,
        gyro_x: 0.0,
        gyro_y: 0.0,
        gyro_z: 0.0,
        temp: 0.0,
        have_reading: false,
    }));

static VIBRATION_ALARM: Mutex<CriticalSectionRawMutex, RefCell<bool>> =
    Mutex::new(RefCell::new(false));

fn apply_reading(
    temperature: f32,
    humidity: f32,
    pressure: Option<f32>,
    temp_green: &mut Output,
    temp_yellow: &mut Output,
    temp_red: &mut Output,
    hum_green: &mut Output,
    hum_yellow: &mut Output,
    hum_red: &mut Output,
    buzzer: &mut Output,
    motor: &mut Output,
) {
    let temp_level = classify_temp(temperature);
    let hum_level = classify_humidity(humidity);
    let vibration_alarm = VIBRATION_ALARM.lock(|cell| *cell.borrow());
    let alarm = temp_level == Level3::Red || hum_level == Level3::Red || vibration_alarm;

    apply_level(temp_level, temp_green, temp_yellow, temp_red);
    apply_level(hum_level, hum_green, hum_yellow, hum_red);
    buzzer.set_level(alarm.into());

    motor.set_level(alarm.into());

    let resolved_pressure = LATEST.lock(|cell| {
        let previous = *cell.borrow();
        pressure.unwrap_or(previous.pressure)
    });

    info!(
        "reading: {} C, {} % RH, {} Pa -> temp={}, humidity={}, vibration_alarm={}, buzzer={}",
        temperature,
        humidity,
        resolved_pressure,
        temp_level.as_str(),
        hum_level.as_str(),
        vibration_alarm,
        alarm
    );

    LATEST.lock(|cell| {
        *cell.borrow_mut() = Reading {
            temperature,
            humidity,
            pressure: resolved_pressure,
            have_reading: true,
        };
    });
}

#[embassy_executor::task]
async fn sensor_task(
    mut bme280: AsyncBme280<
        I2cDevice<'static, NoopRawMutex, i2c::I2c<'static, I2C0, i2c::Async>>,
        Delay,
    >,
    mut temp_green: Output<'static>,
    mut temp_yellow: Output<'static>,
    mut temp_red: Output<'static>,
    mut hum_green: Output<'static>,
    mut hum_yellow: Output<'static>,
    mut hum_red: Output<'static>,
    mut buzzer: Output<'static>,
    mut motor: Output<'static>,
) {
    loop {

        match select(
            Timer::after(Duration::from_millis(2500)),
            SIM_OVERRIDE.wait(),
        )
        .await
        {
            Either::First(()) => {
                let temperature = bme280.read_temperature().await;
                let humidity = bme280.read_humidity().await;
                let pressure = bme280.read_pressure().await;

                match (temperature, humidity, pressure) {
                    (Ok(Some(t)), Ok(Some(h)), Ok(Some(p))) => apply_reading(
                        t,
                        h,
                        Some(p),
                        &mut temp_green,
                        &mut temp_yellow,
                        &mut temp_red,
                        &mut hum_green,
                        &mut hum_yellow,
                        &mut hum_red,
                        &mut buzzer,
                        &mut motor,
                    ),
                    _ => warn!("BME280 read failed or a value was disabled, trying again"),
                }
            }
            Either::Second((temperature, humidity)) => apply_reading(
                temperature,
                humidity,
                None,
                &mut temp_green,
                &mut temp_yellow,
                &mut temp_red,
                &mut hum_green,
                &mut hum_yellow,
                &mut hum_red,
                &mut buzzer,
                &mut motor,
            ),
        }
    }
}

const VIBRATION_WINDOW: usize = 256;
const VIBRATION_SAMPLE_INTERVAL_MS: u64 = 5;

const CALIBRATION_WINDOWS: u32 = 30;

const VIBRATION_ENERGY_THRESHOLD: f32 = 77.0;

#[embassy_executor::task]
async fn mpu_task(
    mut i2c: I2cDevice<'static, NoopRawMutex, i2c::I2c<'static, I2C0, i2c::Async>>,
) {
    match i2c.write(MPU6050_ADDR, &[REG_PWR_MGMT_1, 0x00]).await {
        Ok(_) => info!("MPU6050 woken up"),
        Err(_) => warn!("MPU6050: failed to wake sensor - check wiring/address"),
    }
    Timer::after(Duration::from_millis(100)).await;

    info!(
        "Vibration calibration starting: leave everything still for ~15s, \
         then gently tap or spin the propeller by hand for the rest of the \
         ~38s calibration window. Motor/buzzer will NOT react to vibration \
         until calibration finishes."
    );

    let mut attempt: u32 = 0;
    let mut vibration_buffer = [0f32; VIBRATION_WINDOW];
    let mut buffer_index: usize = 0;
    let mut calibration_window: u32 = 0;
    let mut calibration_min: f32 = f32::MAX;
    let mut calibration_max: f32 = 0.0;

    loop {
        Timer::after(Duration::from_millis(VIBRATION_SAMPLE_INTERVAL_MS)).await;
        attempt += 1;

        let mut buf = [0u8; 14];
        match i2c
            .write_read(MPU6050_ADDR, &[REG_ACCEL_XOUT_H], &mut buf)
            .await
        {
            Ok(_) => {
                let accel_x_raw = i16::from_be_bytes([buf[0], buf[1]]);
                let accel_y_raw = i16::from_be_bytes([buf[2], buf[3]]);
                let accel_z_raw = i16::from_be_bytes([buf[4], buf[5]]);
                let temp_raw = i16::from_be_bytes([buf[6], buf[7]]);
                let gyro_x_raw = i16::from_be_bytes([buf[8], buf[9]]);
                let gyro_y_raw = i16::from_be_bytes([buf[10], buf[11]]);
                let gyro_z_raw = i16::from_be_bytes([buf[12], buf[13]]);

                let accel_z_g = accel_z_raw as f32 / 16384.0;

                let reading = MpuReading {
                    accel_x: accel_x_raw as f32 / 16384.0,
                    accel_y: accel_y_raw as f32 / 16384.0,
                    accel_z: accel_z_g,
                    gyro_x: gyro_x_raw as f32 / 131.0,
                    gyro_y: gyro_y_raw as f32 / 131.0,
                    gyro_z: gyro_z_raw as f32 / 131.0,
                    temp: (temp_raw as f32) / 340.0 + 36.53,
                    have_reading: true,
                };

                LATEST_MPU.lock(|cell| {
                    *cell.borrow_mut() = reading;
                });

                if attempt % 100 == 0 {
                    info!(
                        "MPU6050 reading {}: accel(g) x={} y={} z={} | gyro(deg/s) x={} y={} z={} | temp={} C",
                        attempt,
                        reading.accel_x,
                        reading.accel_y,
                        reading.accel_z,
                        reading.gyro_x,
                        reading.gyro_y,
                        reading.gyro_z,
                        reading.temp
                    );
                }

                vibration_buffer[buffer_index] = accel_z_g;
                buffer_index += 1;

                if buffer_index == VIBRATION_WINDOW {
                    buffer_index = 0;

                    let spectrum = microfft::real::rfft_256(&mut vibration_buffer);

                    spectrum[0].im = 0.0;

                    let ac_energy: f32 = spectrum.iter().skip(1).map(|c| c.norm_sqr()).sum();

                    if calibration_window < CALIBRATION_WINDOWS {
                        calibration_window += 1;
                        calibration_min = calibration_min.min(ac_energy);
                        calibration_max = calibration_max.max(ac_energy);

                        VIBRATION_ALARM.lock(|cell| {
                            *cell.borrow_mut() = false;
                        });

                        info!(
                            "[CALIBRATION {}/{}] AC energy = {} (running min={}, max={})",
                            calibration_window,
                            CALIBRATION_WINDOWS,
                            ac_energy,
                            calibration_min,
                            calibration_max
                        );

                        if calibration_window == CALIBRATION_WINDOWS {
                            let suggested = calibration_max * 1.5;
                            info!(
                                "[CALIBRATION COMPLETE] min={}, max={} -> try \
                                 VIBRATION_ENERGY_THRESHOLD = {} (roughly 1.5x \
                                 observed max), update the constant, and \
                                 re-flash. Normal operation starts now with \
                                 the current threshold ({}).",
                                calibration_min,
                                calibration_max,
                                suggested,
                                VIBRATION_ENERGY_THRESHOLD
                            );
                        }
                    } else {
                        let anomaly = ac_energy > VIBRATION_ENERGY_THRESHOLD;

                        VIBRATION_ALARM.lock(|cell| {
                            *cell.borrow_mut() = anomaly;
                        });

                        info!(
                            "Vibration FFT: AC energy = {} (threshold {}), anomaly = {}",
                            ac_energy, VIBRATION_ENERGY_THRESHOLD, anomaly
                        );
                    }
                }
            }
            Err(_) => {
                if attempt % 100 == 0 {
                    warn!("MPU6050 attempt {}: read failed", attempt);
                }
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    let (net_device, mut control) = embassy_lab_utils::init_wifi!(&spawner, peripherals).await;

    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 4, 1), 24),
        gateway: Some(Ipv4Address::new(192, 168, 4, 1)),
        dns_servers: Vec::new(),
    });

    let temp_green = Output::new(peripherals.PIN_12, Level::Low);
    let temp_yellow = Output::new(peripherals.PIN_13, Level::Low);
    let temp_red = Output::new(peripherals.PIN_14, Level::Low);
    let hum_green = Output::new(peripherals.PIN_9, Level::Low);
    let hum_yellow = Output::new(peripherals.PIN_10, Level::Low);
    let hum_red = Output::new(peripherals.PIN_11, Level::Low);
    let buzzer = Output::new(peripherals.PIN_15, Level::Low);

    let motor = Output::new(peripherals.PIN_18, Level::Low);

    static I2C_BUS: StaticCell<AsyncMutex<NoopRawMutex, i2c::I2c<'static, I2C0, i2c::Async>>> =
        StaticCell::new();
    let i2c0 = i2c::I2c::new_async(
        peripherals.I2C0,
        peripherals.PIN_5,
        peripherals.PIN_4,
        Irqs,
        I2cConfig::default(),
    );
    let i2c_bus = I2C_BUS.init(AsyncMutex::new(i2c0));

    let bme_i2c = I2cDevice::new(i2c_bus);
    let mut bme280 = AsyncBme280::new(bme_i2c, Delay);

    match bme280.init().await {
        Ok(_) => info!("BME280 initialized"),
        Err(_) => warn!("BME280 init failed - check wiring/address"),
    }
    match bme280
        .set_sampling_configuration(
            Configuration::default()
                .with_temperature_oversampling(Oversampling::Oversample1)
                .with_pressure_oversampling(Oversampling::Oversample1)
                .with_humidity_oversampling(Oversampling::Oversample1)
                .with_sensor_mode(SensorMode::Normal),
        )
        .await
    {
        Ok(_) => info!("BME280 sampling configuration set"),
        Err(_) => warn!("BME280 failed to set sampling configuration"),
    }

    spawner
        .spawn(sensor_task(
            bme280, temp_green, temp_yellow, temp_red, hum_green, hum_yellow, hum_red, buzzer,
            motor,
        ))
        .unwrap();

    let mpu_i2c = I2cDevice::new(i2c_bus);
    spawner.spawn(mpu_task(mpu_i2c)).unwrap();

    let stack = embassy_lab_utils::init_network_stack(&spawner, net_device, &RESOURCES, config);

    info!("Starting AP '{}'...", AP_SSID);
    control.start_ap_wpa2(AP_SSID, AP_PASSWORD, 5).await;
    info!("AP started. Connect to it, then reach the Pico at 192.168.4.1:6000");

    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(30)));

        info!("Listening on TCP:6000...");
        if let Err(e) = socket.accept(6000).await {
            warn!("accept error: {:?}", e);
            continue;
        }
        info!("Client connected: {:?}", socket.remote_endpoint());

        let mut buf = [0u8; 256];
        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    info!("connection closed by client");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            let msg = core::str::from_utf8(&buf[..n]).unwrap_or("").trim();
            info!("received: {}", msg);

            let parts: Vec<&str, 3> = msg.split(':').collect();
            let response: heapless::String<200> = match parts.as_slice() {
                ["read"] => {
                    let reading = LATEST.lock(|cell| *cell.borrow());
                    if reading.have_reading {
                        format_reading(reading.temperature, reading.humidity, reading.pressure)
                    } else {
                        heapless::String::try_from("ERR:no_reading_yet\n").unwrap()
                    }
                }
                ["sim", temp_str, hum_str] => {
                    match (temp_str.parse::<f32>(), hum_str.parse::<f32>()) {
                        (Ok(temp), Ok(hum)) => {
                            SIM_OVERRIDE.signal((temp, hum));
                            let pressure =
                                LATEST.lock(|cell| cell.borrow().pressure);
                            format_reading(temp, hum, pressure)
                        }
                        _ => heapless::String::try_from("ERR:bad_numbers\n").unwrap(),
                    }
                }
                ["mpu"] => {
                    let reading = LATEST_MPU.lock(|cell| *cell.borrow());
                    if reading.have_reading {
                        format_mpu_reading(&reading)
                    } else {
                        heapless::String::try_from("ERR:no_reading_yet\n").unwrap()
                    }
                }
                _ => heapless::String::try_from("ERR:unknown_command\n").unwrap(),
            };

            if let Err(e) = socket.write_all(response.as_bytes()).await {
                warn!("write error: {:?}", e);
                break;
            }
        }
    }
}

fn format_reading(temperature: f32, humidity: f32, pressure: f32) -> heapless::String<200> {
    let temp_level = classify_temp(temperature);
    let hum_level = classify_humidity(humidity);
    let vibration_alarm = VIBRATION_ALARM.lock(|cell| *cell.borrow());
    let alarm = temp_level == Level3::Red || hum_level == Level3::Red || vibration_alarm;

    let mut s = heapless::String::<200>::new();
    let _ = core::fmt::write(
        &mut s,
        format_args!(
            "OK:temp={:.1},humidity={:.1},pressure={:.1},temp_level={},humidity_level={},vibration_alarm={},buzzer={}\n",
            temperature,
            humidity,
            pressure,
            temp_level.as_str(),
            hum_level.as_str(),
            if vibration_alarm { "on" } else { "off" },
            if alarm { "on" } else { "off" }
        ),
    );
    s
}

fn format_mpu_reading(reading: &MpuReading) -> heapless::String<200> {
    let mut s = heapless::String::<200>::new();
    let _ = core::fmt::write(
        &mut s,
        format_args!(
            "OK:accel_x={:.3},accel_y={:.3},accel_z={:.3},gyro_x={:.2},gyro_y={:.2},gyro_z={:.2},temp={:.1}\n",
            reading.accel_x,
            reading.accel_y,
            reading.accel_z,
            reading.gyro_x,
            reading.gyro_y,
            reading.gyro_z,
            reading.temp
        ),
    );
    s
}
