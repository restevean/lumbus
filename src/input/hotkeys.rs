//! Carbon hotkey management.
//!
//! This module handles registration, unregistration, and reinstallation
//! of global hotkeys using the Carbon Event Manager API.
//!
//! Note: The hotkey event handler remains in main.rs because it needs
//! to call UI functions (open_settings_window, confirm_and_maybe_quit).

use lumbus::ffi::bridge::{id, ObjectExt};

use lumbus::ffi::{
    EventHandlerRef, EventHotKeyID, EventHotKeyRef, EventTypeSpec, GetApplicationEventTarget,
    InstallEventHandler, RegisterEventHotKey, RemoveEventHandler, UnregisterEventHotKey, CMD_KEY,
    CONTROL_KEY, HKID_HELP, HKID_QUIT, HKID_SETTINGS_COMMA, HKID_TOGGLE, KC_A, KC_COMMA, KC_H,
    KC_X, K_EVENT_CLASS_KEYBOARD, K_EVENT_HOTKEY_PRESSED, NO_ERR, SHIFT_KEY, SIG_MHLT,
};

/// Type alias for the hotkey event handler function signature.
pub type HotkeyHandler = extern "C" fn(
    lumbus::ffi::EventHandlerCallRef,
    lumbus::ffi::EventRef,
    *mut std::ffi::c_void,
) -> i32;

/// Install Carbon hotkeys for the application.
///
/// Registers:
/// - Ctrl+A: Toggle overlay
/// - Cmd+,: Open Settings
/// - Cmd+Shift+H: Show Help
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
        eprintln!("InstallEventHandler failed: {}", status);
        return;
    }
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkHandler", handler_ref as *mut _);

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
                    "RegisterEventHotKey failed (code={}, mods={}, id={}): {}",
                    $keycode, $mods, $idconst, st
                );
            } else {
                (*view).store_ivar::<*mut std::ffi::c_void>($slot, out_ref as *mut _);
            }
        }};
    }

    // Ctrl + A (toggle)
    register_hotkey!(KC_A, CONTROL_KEY, HKID_TOGGLE, "_hkToggle");
    // Cmd + , → Settings
    register_hotkey!(KC_COMMA, CMD_KEY, HKID_SETTINGS_COMMA, "_hkComma");
    // Cmd + Shift + H → Help
    register_hotkey!(KC_H, CMD_KEY | SHIFT_KEY, HKID_HELP, "_hkHelp");
    // Ctrl + Shift + X → Quit confirmation
    register_hotkey!(KC_X, CONTROL_KEY | SHIFT_KEY, HKID_QUIT, "_hkQuit");
}

/// Uninstall all registered Carbon hotkeys.
///
/// # Safety
/// Must be called from main thread.
pub unsafe fn uninstall_hotkeys(view: id) {
    let hk_toggle: *mut std::ffi::c_void = *(*view).load_ivar("_hkToggle");
    let hk_comma: *mut std::ffi::c_void = *(*view).load_ivar("_hkComma");
    let hk_help: *mut std::ffi::c_void = *(*view).load_ivar("_hkHelp");
    let hk_quit: *mut std::ffi::c_void = *(*view).load_ivar("_hkQuit");
    let hk_handler: *mut std::ffi::c_void = *(*view).load_ivar("_hkHandler");

    if !hk_toggle.is_null() {
        let _ = UnregisterEventHotKey(hk_toggle);
        (*view).store_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    }
    if !hk_comma.is_null() {
        let _ = UnregisterEventHotKey(hk_comma);
        (*view).store_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    }
    if !hk_help.is_null() {
        let _ = UnregisterEventHotKey(hk_help);
        (*view).store_ivar::<*mut std::ffi::c_void>("_hkHelp", std::ptr::null_mut());
    }
    if !hk_quit.is_null() {
        let _ = UnregisterEventHotKey(hk_quit);
        (*view).store_ivar::<*mut std::ffi::c_void>("_hkQuit", std::ptr::null_mut());
    }
    if !hk_handler.is_null() {
        let _ = RemoveEventHandler(hk_handler);
        (*view).store_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
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
