#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gamepad;
mod window_control;

use serde_json::json;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use window_control::{initialize_opacity, set_window_opacity};

const VISIBILITY_TOGGLE: &str = "Ctrl+Shift+Space";

#[tauri::command]
fn push_coord(x: f64, y: f64, label: Option<String>) -> Result<(), String> {
    let stream = TcpStream::connect_timeout(
        &"127.0.0.1:61234".parse().map_err(|e| format!("Invalid address: {}", e))?,
        Duration::from_millis(100),
    );

    let mut stream = match stream {
        Ok(s) => s,
        Err(_) => {
            // Silently fail if Lua listener is not running
            return Ok(());
        }
    };

    // Set write timeout to avoid blocking
    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));

    let payload = json!({
      "x": x,
      "y": y,
      "label": label
    });
    stream
        .write_all(payload.to_string().as_bytes())
        .map_err(|error| error.to_string())?;
    stream
        .write_all(
            b"
",
        )
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    Ok(())
}

fn send_lua_command(cmd: &str) -> Result<String, String> {
    use std::env;
    use std::fs;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use system temp directory with project subdirectory
    let temp_dir = env::temp_dir();
    let project_dir = temp_dir.join("Forged-In-Shadow-Torch-Save-Point-Teleport-Mod");

    // Create directory if it doesn't exist
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    let cmd_file = project_dir.join("cmd.txt");
    let resp_file = project_dir.join("resp.txt");

    // Clean up old files
    let _ = fs::remove_file(&cmd_file);
    let _ = fs::remove_file(&resp_file);

    // Generate timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Write command with timestamp
    let command = format!("{} {}", cmd, timestamp);
    fs::write(&cmd_file, &command)
        .map_err(|e| format!("Failed to write command file: {}", e))?;
    println!("[Rust] Command file written: {} (timestamp: {})", cmd, timestamp);

    // Wait for response (poll for up to 5 seconds)
    for i in 0..50 {
        thread::sleep(Duration::from_millis(100));

        if let Ok(response) = fs::read_to_string(&resp_file) {
            println!("[Rust] Got response: {}", response);

            // Verify timestamp in response
            if response.contains(&timestamp.to_string()) || !response.contains("TIMESTAMP:") {
                let _ = fs::remove_file(&resp_file);
                let _ = fs::remove_file(&cmd_file);

                // Extract actual response (remove timestamp if present)
                let actual_response = if let Some(pos) = response.find("TIMESTAMP:") {
                    response[..pos].trim()
                } else {
                    response.trim()
                };

                return Ok(actual_response.to_string());
            } else {
                println!("[Rust] Ignoring response with mismatched timestamp");
            }
        }

        if i % 10 == 0 {
            println!("[Rust] Still waiting for response... ({}s)", i / 10);
        }
    }

    // Cleanup on timeout
    let _ = fs::remove_file(&cmd_file);
    let _ = fs::remove_file(&resp_file);
    Err("Timeout waiting for Lua response".to_string())
}

#[tauri::command]
fn scan_save_points() -> Result<String, String> {
    println!("[Rust] Sending SCAN command via file...");
    send_lua_command("SCAN")
}

#[tauri::command]
fn teleport_to_savepoint(savepoint_name: String) -> Result<String, String> {
    println!("[Rust] Sending TPNAME command via file: {}", savepoint_name);
    let cmd = format!("TPNAME {}", savepoint_name);
    send_lua_command(&cmd)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle();
            initialize_opacity(&app_handle);
            #[cfg(windows)]
            gamepad::spawn(&app_handle);

            app.global_shortcut().on_shortcut(
                VISIBILITY_TOGGLE,
                move |handle, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = handle.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    let _ = window.hide();
                                    dispatch_visibility(&window, false);
                                }
                                Ok(false) => {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    dispatch_visibility(&window, true);
                                }
                                Err(error) => {
                                    eprintln!("failed to read window visibility: {error}");
                                }
                            }
                        }
                    }
                },
            )?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            push_coord,
            scan_save_points,
            teleport_to_savepoint,
            set_window_opacity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dispatch_visibility(window: &WebviewWindow, visible: bool) {
    let script = format!(
        "window.dispatchEvent(new CustomEvent('viewer-visibility', {{ detail: {} }}));",
        visible
    );
    if let Err(error) = window.eval(&script) {
        eprintln!("failed to dispatch visibility event: {error}");
    }
}
