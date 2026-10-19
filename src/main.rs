
use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tower_http::services::ServeDir;

const PICO_ADDR: &str = "192.168.4.1:6000";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

static LED_ON: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/led/{state}", post(set_led))
        .route("/api/simulate", post(simulate))
        .route("/api/reading", get(reading))
        .route("/api/motion", get(motion))
        .route("/api/status", get(status))
        .fallback_service(ServeDir::new("assets"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind to 127.0.0.1:3000");

    println!("SmartStation web app running at http://127.0.0.1:3000");
    println!("Make sure this machine is connected to the Pico's Wi-Fi hotspot");
    println!("for the LED controls / status check to reach it.");

    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    message: String,
    led_on: bool,
}

async fn set_led(Path(state): Path<String>) -> impl IntoResponse {
    let command = match state.as_str() {
        "on" => "led:on\n",
        "off" => "led:off\n",
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    ok: false,
                    message: "state must be 'on' or 'off'".into(),
                    led_on: LED_ON.load(Ordering::Relaxed),
                }),
            );
        }
    };

    match send_to_pico(command).await {
        Ok(reply) => {
            let ok = reply.starts_with("OK");
            if ok {
                LED_ON.store(state == "on", Ordering::Relaxed);
            }
            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_GATEWAY
                },
                Json(ApiResponse {
                    ok,
                    message: reply,
                    led_on: LED_ON.load(Ordering::Relaxed),
                }),
            )
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse {
                ok: false,
                message: format!("could not reach Pico: {e}"),
                led_on: LED_ON.load(Ordering::Relaxed),
            }),
        ),
    }
}

#[derive(Deserialize)]
struct SimulateRequest {
    temperature: f64,
    humidity: f64,
}

#[derive(Serialize)]
struct SimulateResponse {
    ok: bool,
    temp_level: String,
    humidity_level: String,
    vibration_alarm: bool,
    buzzer_on: bool,
    raw: String,
}

async fn simulate(Json(req): Json<SimulateRequest>) -> impl IntoResponse {
    let command = format!("sim:{:.1}:{:.1}\n", req.temperature, req.humidity);

    match send_to_pico(&command).await {
        Ok(reply) => {
            let ok = reply.starts_with("OK");
            let mut temp_level = "unknown".to_string();
            let mut humidity_level = "unknown".to_string();
            let mut vibration_alarm = false;
            let mut buzzer_on = false;

            if ok {

                if let Some(fields) = reply.strip_prefix("OK:") {
                    for field in fields.split(',') {
                        if let Some((key, value)) = field.split_once('=') {
                            match key {
                                "temp" => temp_level = value.to_string(),
                                "humidity" => humidity_level = value.to_string(),
                                "vibration_alarm" => vibration_alarm = value == "on",
                                "buzzer" => buzzer_on = value == "on",
                                _ => {}
                            }
                        }
                    }
                }
            }

            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_GATEWAY
                },
                Json(SimulateResponse {
                    ok,
                    temp_level,
                    humidity_level,
                    vibration_alarm,
                    buzzer_on,
                    raw: reply,
                }),
            )
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(SimulateResponse {
                ok: false,
                temp_level: "unknown".to_string(),
                humidity_level: "unknown".to_string(),
                vibration_alarm: false,
                buzzer_on: false,
                raw: format!("could not reach Pico: {e}"),
            }),
        ),
    }
}

#[derive(Serialize)]
struct ReadingResponse {
    ok: bool,
    temperature: Option<f64>,
    humidity: Option<f64>,
    pressure: Option<f64>,
    temp_level: String,
    humidity_level: String,
    vibration_alarm: bool,
    buzzer_on: bool,
    raw: String,
}

async fn reading() -> impl IntoResponse {
    match send_to_pico("read\n").await {
        Ok(reply) => {
            let ok = reply.starts_with("OK");
            let mut temperature = None;
            let mut humidity = None;
            let mut pressure = None;
            let mut temp_level = "unknown".to_string();
            let mut humidity_level = "unknown".to_string();
            let mut vibration_alarm = false;
            let mut buzzer_on = false;

            if ok {
                if let Some(fields) = reply.strip_prefix("OK:") {
                    for field in fields.split(',') {
                        if let Some((key, value)) = field.split_once('=') {
                            match key {
                                "temp" => temperature = value.parse::<f64>().ok(),
                                "humidity" => humidity = value.parse::<f64>().ok(),
                                "pressure" => pressure = value.parse::<f64>().ok(),
                                "temp_level" => temp_level = value.to_string(),
                                "humidity_level" => humidity_level = value.to_string(),
                                "vibration_alarm" => vibration_alarm = value == "on",
                                "buzzer" => buzzer_on = value == "on",
                                _ => {}
                            }
                        }
                    }
                }
            }

            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_GATEWAY
                },
                Json(ReadingResponse {
                    ok,
                    temperature,
                    humidity,
                    pressure,
                    temp_level,
                    humidity_level,
                    vibration_alarm,
                    buzzer_on,
                    raw: reply,
                }),
            )
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadingResponse {
                ok: false,
                temperature: None,
                humidity: None,
                pressure: None,
                temp_level: "unknown".to_string(),
                humidity_level: "unknown".to_string(),
                vibration_alarm: false,
                buzzer_on: false,
                raw: format!("could not reach Pico: {e}"),
            }),
        ),
    }
}

#[derive(Serialize)]
struct MotionResponse {
    ok: bool,
    accel_x: Option<f64>,
    accel_y: Option<f64>,
    accel_z: Option<f64>,
    gyro_x: Option<f64>,
    gyro_y: Option<f64>,
    gyro_z: Option<f64>,
    temp: Option<f64>,
    raw: String,
}

async fn motion() -> impl IntoResponse {
    match send_to_pico("mpu\n").await {
        Ok(reply) => {
            let ok = reply.starts_with("OK");
            let mut accel_x = None;
            let mut accel_y = None;
            let mut accel_z = None;
            let mut gyro_x = None;
            let mut gyro_y = None;
            let mut gyro_z = None;
            let mut temp = None;

            if ok {
                if let Some(fields) = reply.strip_prefix("OK:") {
                    for field in fields.split(',') {
                        if let Some((key, value)) = field.split_once('=') {
                            match key {
                                "accel_x" => accel_x = value.parse::<f64>().ok(),
                                "accel_y" => accel_y = value.parse::<f64>().ok(),
                                "accel_z" => accel_z = value.parse::<f64>().ok(),
                                "gyro_x" => gyro_x = value.parse::<f64>().ok(),
                                "gyro_y" => gyro_y = value.parse::<f64>().ok(),
                                "gyro_z" => gyro_z = value.parse::<f64>().ok(),
                                "temp" => temp = value.parse::<f64>().ok(),
                                _ => {}
                            }
                        }
                    }
                }
            }

            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_GATEWAY
                },
                Json(MotionResponse {
                    ok,
                    accel_x,
                    accel_y,
                    accel_z,
                    gyro_x,
                    gyro_y,
                    gyro_z,
                    temp,
                    raw: reply,
                }),
            )
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(MotionResponse {
                ok: false,
                accel_x: None,
                accel_y: None,
                accel_z: None,
                gyro_x: None,
                gyro_y: None,
                gyro_z: None,
                temp: None,
                raw: format!("could not reach Pico: {e}"),
            }),
        ),
    }
}

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
}

async fn status() -> impl IntoResponse {
    let connected = timeout(CONNECT_TIMEOUT, TcpStream::connect(PICO_ADDR))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false);

    Json(StatusResponse { connected })
}

async fn send_to_pico(command: &str) -> Result<String, String> {
    let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(PICO_ADDR))
        .await
        .map_err(|_| "connection timed out".to_string())?
        .map_err(|e| e.to_string())?;

    stream
        .write_all(command.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    let mut buf = [0u8; 256];
    let n = timeout(CONNECT_TIMEOUT, stream.read(&mut buf))
        .await
        .map_err(|_| "read timed out".to_string())?
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}
