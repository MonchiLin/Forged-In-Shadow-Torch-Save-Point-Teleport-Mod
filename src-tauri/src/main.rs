#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gamepad;
mod window_control;

use serde_json::json;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;
use tauri::Manager;
use window_control::{initialize_opacity, set_window_opacity, save_current_window_position};

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

#[tauri::command]
fn update_in_game_points(points_json: String) -> Result<(), String> {
    use std::fs;
    use std::path::PathBuf;

    // Get the path to in_game_points.json relative to the executable
    let mut config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    config_path.pop(); // Go up from src-tauri to project root
    config_path.push("src");
    config_path.push("config");
    config_path.push("in_game_points.json");

    println!("[Rust] Updating in_game_points.json at: {:?}", config_path);

    // Validate JSON before writing
    serde_json::from_str::<serde_json::Value>(&points_json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Write to file
    fs::write(&config_path, points_json)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    println!("[Rust] Successfully updated in_game_points.json");
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            initialize_opacity(&app_handle);
            #[cfg(windows)]
            gamepad::spawn(&app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            push_coord,
            scan_save_points,
            teleport_to_savepoint,
            set_window_opacity,
            update_in_game_points,
            save_current_window_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
