use tauri::{AppHandle, Manager, WebviewWindow, LogicalSize};

pub const DEFAULT_OPACITY: f64 = 0.7;
const MIN_OPACITY: f64 = 0.3;
const MAX_OPACITY: f64 = 1.0;
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 900;

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
        // Set fixed window size using LogicalSize to handle DPI scaling
        let size = LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);
        if let Err(error) = window.set_size(size) {
            eprintln!("failed to set window size: {error}");
        }

        // Ensure window is not resizable (cross-platform)
        if let Err(error) = window.set_resizable(false) {
            eprintln!("failed to disable window resizing: {error}");
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
