use eur_screen_position::ActiveMonitor;
use eur_settings::Hotkey;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
use tracing::info;

/// Convert string modifiers to Tauri Modifiers
#[allow(unused)]
pub fn string_modifiers_to_tauri(modifiers: &[String]) -> Option<Modifiers> {
    let mut tauri_modifiers = Modifiers::empty();

    for modifier in modifiers {
        match modifier.to_lowercase().as_str() {
            "ctrl" | "control" => tauri_modifiers |= Modifiers::CONTROL,
            "alt" => tauri_modifiers |= Modifiers::ALT,
            "shift" => tauri_modifiers |= Modifiers::SHIFT,
            "meta" | "cmd" | "super" => tauri_modifiers |= Modifiers::SUPER,
            _ => return None, // Invalid modifier
        }
    }

    if tauri_modifiers.is_empty() {
        None
    } else {
        Some(tauri_modifiers)
    }
}

/// Convert string key to Tauri Code
#[allow(unused)]
pub fn string_key_to_tauri_code(key: &str) -> Option<Code> {
    match key.to_lowercase().as_str() {
        "space" => Some(Code::Space),
        "enter" | "return" => Some(Code::Enter),
        "tab" => Some(Code::Tab),
        "escape" | "esc" => Some(Code::Escape),
        "backspace" => Some(Code::Backspace),
        "delete" | "del" => Some(Code::Delete),
        "home" => Some(Code::Home),
        "end" => Some(Code::End),
        "pageup" => Some(Code::PageUp),
        "pagedown" => Some(Code::PageDown),
        "arrowup" | "up" => Some(Code::ArrowUp),
        "arrowdown" | "down" => Some(Code::ArrowDown),
        "arrowleft" | "left" => Some(Code::ArrowLeft),
        "arrowright" | "right" => Some(Code::ArrowRight),
        "f1" => Some(Code::F1),
        "f2" => Some(Code::F2),
        "f3" => Some(Code::F3),
        "f4" => Some(Code::F4),
        "f5" => Some(Code::F5),
        "f6" => Some(Code::F6),
        "f7" => Some(Code::F7),
        "f8" => Some(Code::F8),
        "f9" => Some(Code::F9),
        "f10" => Some(Code::F10),
        "f11" => Some(Code::F11),
        "f12" => Some(Code::F12),
        // Single character keys
        key if key.len() == 1 => {
            let ch = key.chars().next().unwrap();
            match ch {
                'a' => Some(Code::KeyA),
                'b' => Some(Code::KeyB),
                'c' => Some(Code::KeyC),
                'd' => Some(Code::KeyD),
                'e' => Some(Code::KeyE),
                'f' => Some(Code::KeyF),
                'g' => Some(Code::KeyG),
                'h' => Some(Code::KeyH),
                'i' => Some(Code::KeyI),
                'j' => Some(Code::KeyJ),
                'k' => Some(Code::KeyK),
                'l' => Some(Code::KeyL),
                'm' => Some(Code::KeyM),
                'n' => Some(Code::KeyN),
                'o' => Some(Code::KeyO),
                'p' => Some(Code::KeyP),
                'q' => Some(Code::KeyQ),
                'r' => Some(Code::KeyR),
                's' => Some(Code::KeyS),
                't' => Some(Code::KeyT),
                'u' => Some(Code::KeyU),
                'v' => Some(Code::KeyV),
                'w' => Some(Code::KeyW),
                'x' => Some(Code::KeyX),
                'y' => Some(Code::KeyY),
                'z' => Some(Code::KeyZ),
                '0' => Some(Code::Digit0),
                '1' => Some(Code::Digit1),
                '2' => Some(Code::Digit2),
                '3' => Some(Code::Digit3),
                '4' => Some(Code::Digit4),
                '5' => Some(Code::Digit5),
                '6' => Some(Code::Digit6),
                '7' => Some(Code::Digit7),
                '8' => Some(Code::Digit8),
                '9' => Some(Code::Digit9),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Convert user hotkey to Tauri shortcut
#[allow(unused)]
pub fn user_hotkey_to_shortcut(hotkey: &eur_user::Hotkey) -> Option<Shortcut> {
    let key_code = string_key_to_tauri_code(&hotkey.key)?;
    let modifiers = string_modifiers_to_tauri(&hotkey.modifiers);
    Some(Shortcut::new(modifiers, key_code))
}

/// Get default shortcut for the current OS
#[allow(unused)]
pub fn get_default_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    return Shortcut::new(Some(Modifiers::ALT), Code::Space);

    #[cfg(target_os = "windows")]
    return Shortcut::new(Some(Modifiers::CONTROL), Code::Space);

    #[cfg(target_os = "linux")]
    return Shortcut::new(Some(Modifiers::SUPER), Code::Space);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Shortcut::new(Some(Modifiers::CONTROL), Code::Space);
}

/// Get the launcher shortcut from user settings or default
#[allow(unused)]
pub fn get_launcher_shortcut(app_handle: &tauri::AppHandle) -> Shortcut {
    if let Some(user_controller) = app_handle.try_state::<eur_user::Controller>()
        && let Ok(Some(user)) = user_controller.get_user()
        && !user.hotkeys.open_launcher.key.is_empty()
        && let Some(shortcut) = user_hotkey_to_shortcut(&user.hotkeys.open_launcher)
    {
        info!("Using custom launcher shortcut: {:?}", shortcut);
        return shortcut;
    }

    let default = get_default_shortcut();
    info!("Using default launcher shortcut: {:?}", default);
    default
}

pub fn position_hover_window(hover_window: &tauri::WebviewWindow) {
    let active_monitor = ActiveMonitor::default();
    let (hover_x, hover_y) = active_monitor.calculate_position_for_percentage(
        tauri::PhysicalSize::new(50, 50),
        1.0,
        0.75,
    );
    let _ = hover_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: hover_x,
        y: hover_y,
    }));

    let _ = hover_window.set_size(tauri::PhysicalSize::new(50, 50));
}

pub fn convert_hotkey_to_shortcut(hotkey: Hotkey) -> Shortcut {
    let key_code = string_key_to_tauri_code(&hotkey.key).expect("Invalid key");
    let modifiers = string_modifiers_to_tauri(&hotkey.modifiers);
    Shortcut::new(modifiers, key_code)
}
