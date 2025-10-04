#[cfg(windows)]
use gilrs::{Axis, Button, Event, EventType, Gilrs};
#[cfg(windows)]
use serde::Serialize;
#[cfg(windows)]
use serde_json;
#[cfg(windows)]
use std::time::Duration;
use tauri::Manager;

#[cfg(windows)]
const AXIS_THRESHOLD: f32 = 0.5;

#[cfg(windows)]
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
enum GamepadEventPayload {
    #[serde(rename = "button")]
    Button { button: String, state: String },
    #[serde(rename = "axis")]
    Axis { axis: String, direction: i8 },
}

#[cfg(windows)]
pub fn spawn(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(error) => {
                eprintln!("failed to initialise gamepad listener: {error}");
                return;
            }
        };
        let mut horizontal_dir: i8 = 0;

        loop {
            while let Some(Event { event, .. }) = gilrs.next_event() {
                match event {
                    EventType::ButtonPressed(button, _) => {
                        if let Some(name) = map_button(&button) {
                            emit_button(&handle, name, "pressed");
                        }
                    }
                    EventType::ButtonReleased(button, _) => {
                        if let Some(name) = map_button(&button) {
                            emit_button(&handle, name, "released");
                        }
                    }
                    EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                        let direction = if value > AXIS_THRESHOLD {
                            1
                        } else if value < -AXIS_THRESHOLD {
                            -1
                        } else {
                            0
                        };
                        if direction != horizontal_dir {
                            horizontal_dir = direction;
                            emit_axis(&handle, "left_x", direction);
                        }
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    });
}

#[cfg(not(windows))]
pub fn spawn(_app: &tauri::AppHandle) {}

#[cfg(windows)]
fn emit_button(handle: &tauri::AppHandle, button: &str, state: &str) {
    let payload = GamepadEventPayload::Button {
        button: button.to_string(),
        state: state.to_string(),
    };
    dispatch(handle, &payload);
}

#[cfg(windows)]
fn emit_axis(handle: &tauri::AppHandle, axis: &str, direction: i8) {
    let payload = GamepadEventPayload::Axis {
        axis: axis.to_string(),
        direction,
    };
    dispatch(handle, &payload);
}

#[cfg(windows)]
fn map_button(button: &Button) -> Option<&'static str> {
    match button {
        Button::South => Some("A"),
        Button::East => Some("B"),
        Button::DPadLeft => Some("DPAD_LEFT"),
        Button::DPadRight => Some("DPAD_RIGHT"),
        _ => None,
    }
}

#[cfg(windows)]
fn dispatch(handle: &tauri::AppHandle, payload: &GamepadEventPayload) {
    if let Some(window) = handle.get_webview_window("main") {
        if let Ok(json) = serde_json::to_string(payload) {
            let script = format!(
                "window.dispatchEvent(new CustomEvent('gamepad-event', {{ detail: {} }}));",
                json
            );
            if let Err(error) = window.eval(&script) {
                eprintln!("failed to dispatch gamepad event: {error}");
            }
        }
    }
}
