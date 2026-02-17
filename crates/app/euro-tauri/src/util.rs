use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

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
        "keya" => Some(Code::KeyA),
        "keyb" => Some(Code::KeyB),
        "keyc" => Some(Code::KeyC),
        "keyd" => Some(Code::KeyD),
        "keye" => Some(Code::KeyE),
        "keyf" => Some(Code::KeyF),
        "keyg" => Some(Code::KeyG),
        "keyh" => Some(Code::KeyH),
        "keyi" => Some(Code::KeyI),
        "keyj" => Some(Code::KeyJ),
        "keyk" => Some(Code::KeyK),
        "keyl" => Some(Code::KeyL),
        "keym" => Some(Code::KeyM),
        "keyn" => Some(Code::KeyN),
        "keyo" => Some(Code::KeyO),
        "keyp" => Some(Code::KeyP),
        "keyq" => Some(Code::KeyQ),
        "keyr" => Some(Code::KeyR),
        "keys" => Some(Code::KeyS),
        "keyt" => Some(Code::KeyT),
        "keyu" => Some(Code::KeyU),
        "keyv" => Some(Code::KeyV),
        "keyw" => Some(Code::KeyW),
        "keyx" => Some(Code::KeyX),
        "keyy" => Some(Code::KeyY),
        "keyz" => Some(Code::KeyZ),
        "digit0" => Some(Code::Digit0),
        "digit1" => Some(Code::Digit1),
        "digit2" => Some(Code::Digit2),
        "digit3" => Some(Code::Digit3),
        "digit4" => Some(Code::Digit4),
        "digit5" => Some(Code::Digit5),
        "digit6" => Some(Code::Digit6),
        "digit7" => Some(Code::Digit7),
        "digit8" => Some(Code::Digit8),
        "digit9" => Some(Code::Digit9),
        "numpad0" => Some(Code::Numpad0),
        "numpad1" => Some(Code::Numpad1),
        "numpad2" => Some(Code::Numpad2),
        "numpad3" => Some(Code::Numpad3),
        "numpad4" => Some(Code::Numpad4),
        "numpad5" => Some(Code::Numpad5),
        "numpad6" => Some(Code::Numpad6),
        "numpad7" => Some(Code::Numpad7),
        "numpad8" => Some(Code::Numpad8),
        "numpad9" => Some(Code::Numpad9),
        _ => None,
    }
}

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

pub fn get_db_path(app_handle: &tauri::AppHandle) -> String {
    let base_path = app_handle.path().app_data_dir().unwrap();
    std::fs::create_dir_all(&base_path).unwrap();
    let db_path = base_path.join("personal_database.sqlite");
    db_path.to_string_lossy().to_string()
}
