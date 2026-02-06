//! Status bar (menu bar) item with dropdown menu.
//!
//! Creates a clickable icon in the macOS menu bar with options:
//! - Settings (Ajustes)
//! - Help (Ayuda)
//! - About (Acerca de...)
//! - Quit (Salir)

use lumbus::ffi::bridge::{get_class, id, msg_send, nil, nsstring_id, sel, NSSize, YES};

use crate::app::lang_is_es;
use lumbus::tr_key;

/// Global reference to the status item (must be kept alive)
static mut STATUS_ITEM: id = std::ptr::null_mut();

/// Install the status bar item with menu.
///
/// # Safety
/// Must be called from main thread, after the app is initialized.
pub unsafe fn install_status_bar(view: id) {
    let status_bar: id = msg_send![get_class("NSStatusBar"), systemStatusBar];

    // NSVariableStatusItemLength = -1.0
    let status_item: id = msg_send![status_bar, statusItemWithLength: -1.0f64];

    // Keep a strong reference so it doesn't get deallocated
    let _: id = msg_send![status_item, retain];
    STATUS_ITEM = status_item;

    // Set the icon from StatusBarIcon.png in Resources
    let button: id = msg_send![status_item, button];
    if button != nil {
        let bundle: id = msg_send![get_class("NSBundle"), mainBundle];
        let resources_path: id = msg_send![bundle, resourcePath];

        // Build full path to StatusBarIcon.png
        let icon_filename = nsstring_id("StatusBarIcon.png");
        let icon_path: id = msg_send![
            resources_path,
            stringByAppendingPathComponent: icon_filename
        ];

        let icon: id = msg_send![get_class("NSImage"), alloc];
        let icon: id = msg_send![icon, initWithContentsOfFile: icon_path];

        if icon != nil {
            // Set size for menu bar (18x18 standard)
            let _: () = msg_send![icon, setSize: NSSize::new(18.0, 18.0)];
            // Make it template so it adapts to light/dark mode
            let _: () = msg_send![icon, setTemplate: YES];
            let _: () = msg_send![button, setImage: icon];
        } else {
            // Fallback: use text if icon not found
            let _: () = msg_send![button, setTitle: nsstring_id("MH")];
        }
    }

    // Create menu
    let menu = create_status_menu(view);
    let _: () = msg_send![status_item, setMenu: menu];
}

/// Create the dropdown menu for the status bar item.
unsafe fn create_status_menu(view: id) -> id {
    let es = lang_is_es(view);

    let menu: id = msg_send![get_class("NSMenu"), alloc];
    let menu: id = msg_send![menu, init];

    // Settings item
    let settings_title = tr_key("Settings", es);
    let settings_item: id = msg_send![get_class("NSMenuItem"), alloc];
    let settings_item: id = msg_send![
        settings_item,
        initWithTitle: nsstring_id(&settings_title),
        action: sel!(statusBarSettings:),
        keyEquivalent: nsstring_id(",")
    ];
    let _: () = msg_send![settings_item, setTarget: view];
    let _: () = msg_send![menu, addItem: settings_item];

    // Help item (Cmd+Shift+H)
    let help_title = tr_key("Help", es);
    let help_item: id = msg_send![get_class("NSMenuItem"), alloc];
    let help_item: id = msg_send![
        help_item,
        initWithTitle: nsstring_id(&help_title),
        action: sel!(statusBarHelp:),
        keyEquivalent: nsstring_id("H")
    ];
    // NSEventModifierFlagCommand (1 << 20) + NSEventModifierFlagShift (1 << 17)
    let _: () = msg_send![help_item, setKeyEquivalentModifierMask: (1u64 << 20) | (1u64 << 17)];
    let _: () = msg_send![help_item, setTarget: view];
    let _: () = msg_send![menu, addItem: help_item];

    // Separator
    let separator: id = msg_send![get_class("NSMenuItem"), separatorItem];
    let _: () = msg_send![menu, addItem: separator];

    // About item
    let about_title = if es { "Acerca de..." } else { "About..." };
    let about_item: id = msg_send![get_class("NSMenuItem"), alloc];
    let about_item: id = msg_send![
        about_item,
        initWithTitle: nsstring_id(about_title),
        action: sel!(statusBarAbout:),
        keyEquivalent: nsstring_id("")
    ];
    let _: () = msg_send![about_item, setTarget: view];
    let _: () = msg_send![menu, addItem: about_item];

    // Separator
    let separator2: id = msg_send![get_class("NSMenuItem"), separatorItem];
    let _: () = msg_send![menu, addItem: separator2];

    // Quit item (no shortcut - direct quit without confirmation)
    let quit_title = tr_key("Quit", es);
    let quit_item: id = msg_send![get_class("NSMenuItem"), alloc];
    let quit_item: id = msg_send![
        quit_item,
        initWithTitle: nsstring_id(&quit_title),
        action: sel!(statusBarQuit:),
        keyEquivalent: nsstring_id("")
    ];
    let _: () = msg_send![quit_item, setTarget: view];
    let _: () = msg_send![menu, addItem: quit_item];

    menu
}

/// Update the status bar menu language.
///
/// Call this when the language changes in settings.
pub unsafe fn update_status_bar_language(view: id) {
    if STATUS_ITEM == nil {
        return;
    }

    let menu = create_status_menu(view);
    let _: () = msg_send![STATUS_ITEM, setMenu: menu];
}
