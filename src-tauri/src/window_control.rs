use tauri::{AppHandle, Manager, WebviewWindow, LogicalSize, PhysicalPosition, PhysicalSize, Monitor};
use serde::{Deserialize, Serialize};

pub const DEFAULT_OPACITY: f64 = 0.7;
const MIN_OPACITY: f64 = 0.3;
const MAX_OPACITY: f64 = 1.0;
const DEFAULT_WINDOW_WIDTH: u32 = 1280;
const DEFAULT_WINDOW_HEIGHT: u32 = 900;
const MIN_WINDOW_WIDTH: u32 = 1280;
const MIN_WINDOW_HEIGHT: u32 = 900;

#[derive(Debug, Serialize, Deserialize)]
struct WindowPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn get_config_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    Ok(config_dir.join("window_geometry.json"))
}

fn save_window_geometry(app: &AppHandle, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let config_path = get_config_path(app)?;
    let geometry = WindowGeometry { x, y, width, height };
    let json = serde_json::to_string_pretty(&geometry)
        .map_err(|e| format!("Failed to serialize geometry: {}", e))?;

    std::fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write geometry file: {}", e))?;

    println!("[WindowControl] Saved window geometry: x={}, y={}, width={}, height={}", x, y, width, height);
    Ok(())
}

fn load_window_geometry(app: &AppHandle) -> Result<WindowGeometry, String> {
    let config_path = get_config_path(app)?;

    if !config_path.exists() {
        return Err("Geometry file does not exist".to_string());
    }

    let json = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read geometry file: {}", e))?;

    let geometry: WindowGeometry = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse geometry: {}", e))?;

    println!("[WindowControl] Loaded window geometry: x={}, y={}, width={}, height={}",
             geometry.x, geometry.y, geometry.width, geometry.height);
    Ok(geometry)
}

/// Check if a window position is within bounds of any available monitor
fn is_position_in_bounds(window: &WebviewWindow, x: i32, y: i32, width: u32, height: u32) -> bool {
    // Get all available monitors
    let monitors = match window.available_monitors() {
        Ok(monitors) => monitors,
        Err(e) => {
            eprintln!("[WindowControl] Failed to get available monitors: {}", e);
            return true; // If we can't check, assume it's valid
        }
    };

    if monitors.is_empty() {
        eprintln!("[WindowControl] No monitors available");
        return true; // If no monitors, assume it's valid
    }

    // Check if window is at least partially visible on any monitor
    for monitor in monitors {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();

        let monitor_x = monitor_pos.x;
        let monitor_y = monitor_pos.y;
        let monitor_w = monitor_size.width as i32;
        let monitor_h = monitor_size.height as i32;

        // Window bounds
        let win_right = x + width as i32;
        let win_bottom = y + height as i32;
        let monitor_right = monitor_x + monitor_w;
        let monitor_bottom = monitor_y + monitor_h;

        // Check if there's any overlap between window and monitor
        let has_overlap = x < monitor_right
            && win_right > monitor_x
            && y < monitor_bottom
            && win_bottom > monitor_y;

        if has_overlap {
            println!("[WindowControl] Window position ({}, {}) is within monitor bounds", x, y);
            return true;
        }
    }

    println!("[WindowControl] Window position ({}, {}) is OUT OF BOUNDS", x, y);
    false
}

/// Get the primary monitor or the first available monitor
fn get_primary_monitor(window: &WebviewWindow) -> Option<Monitor> {
    match window.primary_monitor() {
        Ok(Some(monitor)) => Some(monitor),
        _ => {
            // Fallback to first available monitor
            match window.available_monitors() {
                Ok(mut monitors) => monitors.drain(..).next(),
                Err(_) => None,
            }
        }
    }
}

#[cfg(windows)]
fn apply_window_opacity(window: &WebviewWindow, opacity: f64) -> Result<(), String> {
    use tauri::window::Color;
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA,
        WS_EX_LAYERED,
    };

    let clamped = opacity.clamp(MIN_OPACITY, MAX_OPACITY);
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let should_disable_layer = clamped >= 0.999;

    if should_disable_layer {
        window
            .set_background_color(Some(Color(12, 13, 17, 255)))
            .map_err(|error| error.to_string())?;
    } else {
        window
            .set_background_color(None)
            .map_err(|error| error.to_string())?;
    }

    unsafe {
        let mut ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if should_disable_layer {
            if ex_style & WS_EX_LAYERED.0 as i32 != 0 {
                ex_style &= !(WS_EX_LAYERED.0 as i32);
                SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
            }
            return Ok(());
        }

        if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
            ex_style |= WS_EX_LAYERED.0 as i32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
        }

        let alpha = (clamped * 255.0).round() as u8;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
            .map_err(|error| error.to_string())
    }
}

#[cfg(not(windows))]
fn apply_window_opacity(_window: &WebviewWindow, _opacity: f64) -> Result<(), String> {
    Ok(())
}

pub fn initialize_opacity(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // Enable window resizing
        if let Err(error) = window.set_resizable(true) {
            eprintln!("failed to enable window resizing: {error}");
        }

        // Set minimum window size
        let min_size = LogicalSize::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
        if let Err(error) = window.set_min_size(Some(min_size)) {
            eprintln!("failed to set minimum window size: {error}");
        }

        // Restore saved window geometry or use defaults
        if let Ok(geometry) = load_window_geometry(app) {
            // Restore size
            let size = PhysicalSize::new(geometry.width, geometry.height);
            if let Err(error) = window.set_size(size) {
                eprintln!("failed to restore window size: {error}");
            }

            // Check if the saved position is within bounds of any available monitor
            let is_valid = is_position_in_bounds(&window, geometry.x, geometry.y, geometry.width, geometry.height);

            if is_valid {
                // Restore position
                let pos = PhysicalPosition::new(geometry.x, geometry.y);
                if let Err(error) = window.set_position(pos) {
                    eprintln!("failed to restore window position: {error}");
                }
            } else {
                // Position is out of bounds - move to primary monitor
                println!("[WindowControl] Saved position is out of bounds, moving to primary monitor");

                if let Some(monitor) = get_primary_monitor(&window) {
                    let monitor_pos = monitor.position();
                    let monitor_size = monitor.size();

                    // Center the window on the primary monitor
                    let center_x = monitor_pos.x + (monitor_size.width as i32 - geometry.width as i32) / 2;
                    let center_y = monitor_pos.y + (monitor_size.height as i32 - geometry.height as i32) / 2;

                    let pos = PhysicalPosition::new(center_x, center_y);
                    if let Err(error) = window.set_position(pos) {
                        eprintln!("failed to set centered position: {error}");
                    } else {
                        println!("[WindowControl] Moved window to primary monitor center: x={}, y={}", center_x, center_y);
                    }
                } else {
                    eprintln!("[WindowControl] Could not get primary monitor");
                }
            }
        } else {
            // Use default size
            let size = LogicalSize::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
            if let Err(error) = window.set_size(size) {
                eprintln!("failed to set default window size: {error}");
            }
        }

        // Apply opacity (Windows only)
        #[cfg(windows)]
        if let Err(error) = apply_window_opacity(&window, DEFAULT_OPACITY) {
            eprintln!("failed to set default window opacity: {error}");
        }
    }
}

#[tauri::command]
pub fn set_window_opacity(app: AppHandle, opacity: f64) -> Result<bool, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not available".to_string())?;

    apply_window_opacity(&window, opacity).map(|_| cfg!(windows))
}

#[tauri::command]
pub fn save_current_window_position(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not available".to_string())?;

    let position = window
        .outer_position()
        .map_err(|e| format!("Failed to get window position: {}", e))?;

    let size = window
        .outer_size()
        .map_err(|e| format!("Failed to get window size: {}", e))?;

    save_window_geometry(&app, position.x, position.y, size.width, size.height)
}
