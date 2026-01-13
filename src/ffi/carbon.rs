//! FFI bindings for Carbon Event Manager (hotkeys).
//!
//! This module provides the low-level Carbon API declarations needed
//! for registering and handling global hotkeys on macOS.

// === Types ===

pub type EventTargetRef = *mut std::ffi::c_void;
pub type EventHandlerRef = *mut std::ffi::c_void;
pub type EventRef = *mut std::ffi::c_void;
pub type EventHandlerUPP =
    extern "C" fn(EventHandlerCallRef, EventRef, *mut std::ffi::c_void) -> i32;
pub type EventHandlerCallRef = *mut std::ffi::c_void;
pub type EventHotKeyRef = *mut std::ffi::c_void;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EventHotKeyID {
    pub signature: u32,
    pub id: u32,
}

// === Constants ===

pub const NO_ERR: i32 = 0;
pub const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7962; // 'keyb'
pub const K_EVENT_HOTKEY_PRESSED: u32 = 6;
pub const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2D2D_2D2D; // '----'
pub const TYPE_EVENT_HOTKEY_ID: u32 = 0x686B_6964; // 'hkid'

// Modifiers
pub const CMD_KEY: u32 = 1 << 8;
pub const SHIFT_KEY: u32 = 1 << 9;
pub const CONTROL_KEY: u32 = 1 << 12;

// ANSI keycodes
pub const KC_A: u32 = 0;
pub const KC_X: u32 = 7;
pub const KC_SEMICOLON: u32 = 41;
pub const KC_COMMA: u32 = 43;
pub const KC_H: u32 = 4;        // H key for Help hotkey

// Hotkey signature: 'mhlt'
pub const SIG_MHLT: u32 = 0x6D68_6C74;

// Hotkey IDs
pub const HKID_TOGGLE: u32 = 1;
pub const HKID_SETTINGS_COMMA: u32 = 2;
pub const HKID_SETTINGS_SEMI: u32 = 3;
pub const HKID_QUIT: u32 = 4;
pub const HKID_HELP: u32 = 5;         // Cmd+Shift+H

// === FFI Declarations ===

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn RegisterEventHotKey(
        inHotKeyCode: u32,
        inHotKeyModifiers: u32,
        inHotKeyID: EventHotKeyID,
        inTarget: EventTargetRef,
        inOptions: u32,
        outRef: *mut EventHotKeyRef,
    ) -> i32;

    pub fn UnregisterEventHotKey(inHotKeyRef: EventHotKeyRef) -> i32;

    pub fn InstallEventHandler(
        inTarget: EventTargetRef,
        inHandler: EventHandlerUPP,
        inNumTypes: u32,
        inList: *const EventTypeSpec,
        inUserData: *mut std::ffi::c_void,
        outRef: *mut EventHandlerRef,
    ) -> i32;

    pub fn RemoveEventHandler(inHandlerRef: EventHandlerRef) -> i32;

    pub fn GetApplicationEventTarget() -> EventTargetRef;

    pub fn GetEventClass(inEvent: EventRef) -> u32;
    pub fn GetEventKind(inEvent: EventRef) -> u32;

    pub fn GetEventParameter(
        inEvent: EventRef,
        inName: u32,
        inDesiredType: u32,
        outActualType: *mut u32,
        inBufferSize: u32,
        outActualSize: *mut u32,
        outData: *mut std::ffi::c_void,
    ) -> i32;
}
