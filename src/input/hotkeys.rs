//! Carbon hotkey management.
//!
//! This module handles registration, unregistration, and reinstallation
//! of global hotkeys using the Carbon Event Manager API.
//!
//! Note: The hotkey event handler remains in main.rs because it needs
//! to call UI functions (open_settings_window, confirm_and_maybe_quit).

use cocoa::base::id;

use crate::ffi::{
    EventHotKeyID, EventHotKeyRef, EventHandlerRef, EventTypeSpec,
    GetApplicationEventTarget, InstallEventHandler, RegisterEventHotKey,
    RemoveEventHandler, UnregisterEventHotKey,
    K_EVENT_CLASS_KEYBOARD, K_EVENT_HOTKEY_PRESSED,
    KC_A, KC_COMMA, KC_SEMICOLON, KC_X,
    CMD_KEY, CONTROL_KEY, SHIFT_KEY,
    HKID_TOGGLE, HKID_SETTINGS_COMMA, HKID_SETTINGS_SEMI, HKID_QUIT,
    SIG_MHLT, NO_ERR,
};

/// Type alias for the hotkey event handler function signature.
pub type HotkeyHandler = extern "C" fn(
    crate::ffi::EventHandlerCallRef,
    crate::ffi::EventRef,
    *mut std::ffi::c_void,
) -> i32;

/// Install Carbon hotkeys for the application.
///
/// Registers:
/// - Ctrl+A: Toggle overlay
/// - ⌘+,: Open Settings
/// - ⌘+;: Open Settings (ISO keyboards)
/// - Ctrl+Shift+X: Quit confirmation
///
/// # Safety
/// Must be called from main thread. The handler function pointer must remain valid.
pub unsafe fn install_hotkeys(view: id, handler: HotkeyHandler) {
    // Install Carbon handler for hotkey events
    let types = [EventTypeSpec {
        event_class: K_EVENT_CLASS_KEYBOARD,
        event_kind: K_EVENT_HOTKEY_PRESSED,
    }];
    let mut handler_ref: EventHandlerRef = std::ptr::null_mut();
    let status = InstallEventHandler(
        GetApplicationEventTarget(),
        handler,
        types.len() as u32,
        types.as_ptr(),
        view as *mut std::ffi::c_void,
        &mut handler_ref,
    );
    if status != NO_ERR {
        eprintln!("❌ InstallEventHandler failed: {}", status);
        return;
    }
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", handler_ref as *mut _);

    macro_rules! register_hotkey {
        ($keycode:expr, $mods:expr, $idconst:expr, $slot:literal) => {{
            let hk_id = EventHotKeyID {
                signature: SIG_MHLT,
                id: $idconst,
            };
            let mut out_ref: EventHotKeyRef = std::ptr::null_mut();
            let st = RegisterEventHotKey(
                $keycode as u32,
                $mods as u32,
                hk_id,
                GetApplicationEventTarget(),
                0,
                &mut out_ref,
            );
            if st != NO_ERR || out_ref.is_null() {
                eprintln!(
                    "❌ RegisterEventHotKey failed (code={}, mods={}, id={}): {}",
                    $keycode, $mods, $idconst, st
                );
            } else {
                (*view).set_ivar::<*mut std::ffi::c_void>($slot, out_ref as *mut _);
            }
        }};
    }

    // Ctrl + A (toggle)
    register_hotkey!(KC_A, CONTROL_KEY, HKID_TOGGLE, "_hkToggle");
    // ⌘ + ,  and ⌘ + ; → Settings
    register_hotkey!(KC_COMMA, CMD_KEY, HKID_SETTINGS_COMMA, "_hkComma");
    register_hotkey!(KC_SEMICOLON, CMD_KEY, HKID_SETTINGS_SEMI, "_hkSemi");
    // Ctrl + Shift + X → Quit confirmation
    register_hotkey!(KC_X, CONTROL_KEY | SHIFT_KEY, HKID_QUIT, "_hkQuit");
}

/// Uninstall all registered Carbon hotkeys.
///
/// # Safety
/// Must be called from main thread.
pub unsafe fn uninstall_hotkeys(view: id) {
    let hk_toggle: *mut std::ffi::c_void = *(*view).get_ivar("_hkToggle");
    let hk_comma: *mut std::ffi::c_void = *(*view).get_ivar("_hkComma");
    let hk_semi: *mut std::ffi::c_void = *(*view).get_ivar("_hkSemi");
    let hk_quit: *mut std::ffi::c_void = *(*view).get_ivar("_hkQuit");
    let hk_handler: *mut std::ffi::c_void = *(*view).get_ivar("_hkHandler");

    if !hk_toggle.is_null() {
        let _ = UnregisterEventHotKey(hk_toggle);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    }
    if !hk_comma.is_null() {
        let _ = UnregisterEventHotKey(hk_comma);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    }
    if !hk_semi.is_null() {
        let _ = UnregisterEventHotKey(hk_semi);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkSemi", std::ptr::null_mut());
    }
    if !hk_quit.is_null() {
        let _ = UnregisterEventHotKey(hk_quit);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkQuit", std::ptr::null_mut());
    }
    if !hk_handler.is_null() {
        let _ = RemoveEventHandler(hk_handler);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    }
}

/// Re-install hotkeys safely (unregister first to avoid leaks).
///
/// # Safety
/// Must be called from main thread.
pub unsafe fn reinstall_hotkeys(view: id, handler: HotkeyHandler) {
    uninstall_hotkeys(view);
    install_hotkeys(view, handler);
}
