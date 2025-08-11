#![allow(unexpected_cfgs)] // Silence cfg warnings inside objc/cocoa macros

use block::ConcreteBlock;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize};
use objc::runtime::{Class, Object, Sel};
use objc::{class, declare::ClassDecl, msg_send, sel, sel_impl};
use std::borrow::Cow;
use std::ffi::{CStr, CString};

//
// ===================== Carbon HotKeys (FFI) =====================
//

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn RegisterEventHotKey(
        inHotKeyCode: u32,
        inHotKeyModifiers: u32,
        inHotKeyID: EventHotKeyID,
        inTarget: EventTargetRef,
        inOptions: u32,
        outRef: *mut EventHotKeyRef,
    ) -> i32;

    fn UnregisterEventHotKey(inHotKeyRef: EventHotKeyRef) -> i32;

    fn InstallEventHandler(
        inTarget: EventTargetRef,
        inHandler: EventHandlerUPP,
        inNumTypes: u32,
        inList: *const EventTypeSpec,
        inUserData: *mut std::ffi::c_void,
        outRef: *mut EventHandlerRef,
    ) -> i32;

    fn RemoveEventHandler(inHandlerRef: EventHandlerRef) -> i32;

    fn GetApplicationEventTarget() -> EventTargetRef;

    fn GetEventClass(inEvent: EventRef) -> u32;
    fn GetEventKind(inEvent: EventRef) -> u32;

    fn GetEventParameter(
        inEvent: EventRef,
        inName: u32,
        inDesiredType: u32,
        outActualType: *mut u32,
        inBufferSize: u32,
        outActualSize: *mut u32,
        outData: *mut std::ffi::c_void,
    ) -> i32;
}

type EventTargetRef = *mut std::ffi::c_void;
type EventHandlerRef = *mut std::ffi::c_void;
type EventRef = *mut std::ffi::c_void;
type EventHandlerUPP = extern "C" fn(EventHandlerCallRef, EventRef, *mut std::ffi::c_void) -> i32;
type EventHandlerCallRef = *mut std::ffi::c_void;
type EventHotKeyRef = *mut std::ffi::c_void;

#[repr(C)]
#[derive(Copy, Clone)]
struct EventTypeSpec {
    // Layout Carbon
    event_class: u32,
    event_kind: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EventHotKeyID {
    signature: u32,
    id: u32,
}

// Carbon constants
const NO_ERR: i32 = 0;
const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7962; // 'keyb'
const K_EVENT_HOTKEY_PRESSED: u32 = 6;
const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2D2D_2D2D; // '----'
const TYPE_EVENT_HOTKEY_ID: u32 = 0x686B_6964; // 'hkid'

// Modifiers
const CMD_KEY: u32 = 1 << 8;
const CONTROL_KEY: u32 = 1 << 12;

// ANSI keycodes
const KC_A: u32 = 0;
const KC_SEMICOLON: u32 = 41;
const KC_COMMA: u32 = 43;

// Hotkey signature: 'mhlt'
const SIG_MHLT: u32 = 0x6D68_6C74;
// IDs
const HKID_TOGGLE: u32 = 1;
const HKID_SETTINGS_COMMA: u32 = 2;
const HKID_SETTINGS_SEMI: u32 = 3;

//
// ======= CoreText / CoreGraphics / CoreFoundation (glyphs & paths) =======
//

type CTFontRef = *const std::ffi::c_void;
type CGPathRef = *const std::ffi::c_void;

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    fn CTFontCreateWithName(
        name: *const std::ffi::c_void,
        size: f64,
        matrix: *const std::ffi::c_void,
    ) -> CTFontRef;
    fn CTFontGetGlyphsForCharacters(
        font: CTFontRef,
        chars: *const u16,
        glyphs: *mut u16,
        count: isize,
    ) -> bool;
    fn CTFontCreatePathForGlyph(
        font: CTFontRef,
        glyph: u16,
        transform: *const std::ffi::c_void,
    ) -> CGPathRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPathRelease(path: CGPathRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(obj: *const std::ffi::c_void);
    fn CFAbsoluteTimeGetCurrent() -> f64;
}

// Para TCC Accesibilidad
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

//
// ===================== Defaults / Appearance =====================
//

const DEFAULT_DIAMETER: f64 = 38.5;
const DEFAULT_BORDER_WIDTH: f64 = 3.0;
const DEFAULT_COLOR: (f64, f64, f64, f64) = (1.0, 1.0, 1.0, 1.0);
const DEFAULT_FILL_TRANSPARENCY_PCT: f64 = 100.0; // 100% transparente

//
// ===================== NSUserDefaults keys =====================
//

const PREF_RADIUS: &str = "radius";
const PREF_BORDER: &str = "borderWidth";
const PREF_R: &str = "strokeR";
const PREF_G: &str = "strokeG";
const PREF_B: &str = "strokeB";
const PREF_A: &str = "strokeA";
const PREF_FILL_T: &str = "fillTransparencyPct";
const PREF_LANG: &str = "lang"; // 0 = en, 1 = es

//
// ===================== App =====================
//

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Solicita (si falta) el permiso de Accesibilidad con prompt del sistema
        ensure_accessibility_prompt();

        let app = NSApp();
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );

        // Una ventana por pantalla
        let screens: id = msg_send![class!(NSScreen), screens];
        let count: usize = msg_send![screens, count];
        if count == 0 {
            eprintln!("No screens available.");
            return;
        }

        let mut views: Vec<id> = Vec::with_capacity(count);
        for i in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: i];
            let (win, view) = make_window_for_screen(screen);
            let _: () = msg_send![win, orderFrontRegardless];
            views.push(view);
        }

        // Host
        let host_view = *views.first().unwrap();

        // Carga y sincroniza prefs
        load_preferences_into_view(host_view);
        sync_visual_prefs_to_all_views(host_view);

        // Timer ~60 FPS
        let _ = create_timer(host_view, sel!(update_cursor_multi), 0.016);

        // Hotkeys/monitores
        install_hotkeys(host_view);
        install_mouse_monitors(host_view);
        install_termination_observer(host_view);
        install_local_ctrl_a_monitor(host_view);

        start_hotkey_keepalive(host_view);
        install_wakeup_space_observers(host_view);

        app.run();
    }
}

/// Window level un poco por encima de menús emergentes/Dock
fn nspop_up_menu_window_level() -> i64 {
    201
}

fn overlay_window_level() -> i64 {
    nspop_up_menu_window_level() + 1
}

/// Posición global del puntero (coords Cocoa)
fn get_mouse_position_cocoa() -> (f64, f64) {
    unsafe {
        let cls = class!(NSEvent);
        let p: NSPoint = msg_send![cls, mouseLocation];
        (p.x, p.y)
    }
}

/// NSString* from &str
unsafe fn nsstring(s: &str) -> id {
    let cstr = CString::new(s).unwrap();
    let ns: id = msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
    ns
}

/// DisplayID (CGDirectDisplayID) estable para un NSScreen
unsafe fn display_id_for_screen(screen: id) -> u32 {
    let desc: id = msg_send![screen, deviceDescription];
    let key = nsstring("NSScreenNumber");
    let num: id = msg_send![desc, objectForKey: key];
    if num == nil {
        0
    } else {
        let v: u64 = msg_send![num, unsignedIntegerValue];
        v as u32
    }
}

//
// ===================== NSUserDefaults helpers =====================
//

unsafe fn prefs_get_double(key: &str, default: f64) -> f64 {
    let ud: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
    let k = nsstring(key);
    let obj: id = msg_send![ud, objectForKey: k];
    if obj == nil {
        default
    } else {
        let v: f64 = msg_send![ud, doubleForKey: k];
        v
    }
}

unsafe fn prefs_set_double(key: &str, val: f64) {
    let ud: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
    let k = nsstring(key);
    let _: () = msg_send![ud, setDouble: val forKey: k];
}

unsafe fn prefs_get_int(key: &str, default: i32) -> i32 {
    let ud: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
    let k = nsstring(key);
    let obj: id = msg_send![ud, objectForKey: k];
    if obj == nil {
        default
    } else {
        let v: i32 = msg_send![ud, integerForKey: k];
        v
    }
}

unsafe fn prefs_set_int(key: &str, val: i32) {
    let ud: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
    let k = nsstring(key);
    let _: () = msg_send![ud, setInteger: val forKey: k];
}

/// Load preferences into a view
unsafe fn load_preferences_into_view(view: id) {
    let radius = prefs_get_double(PREF_RADIUS, DEFAULT_DIAMETER / 2.0);
    let border = prefs_get_double(PREF_BORDER, DEFAULT_BORDER_WIDTH);
    let r = prefs_get_double(PREF_R, DEFAULT_COLOR.0);
    let g = prefs_get_double(PREF_G, DEFAULT_COLOR.1);
    let b = prefs_get_double(PREF_B, DEFAULT_COLOR.2);
    let a = prefs_get_double(PREF_A, DEFAULT_COLOR.3);
    let fill_t = prefs_get_double(PREF_FILL_T, DEFAULT_FILL_TRANSPARENCY_PCT);
    let lang = prefs_get_int(PREF_LANG, 0); // 0 en, 1 es

    (*view).set_ivar::<f64>("_radius", radius);
    (*view).set_ivar::<f64>("_borderWidth", border);
    (*view).set_ivar::<f64>("_strokeR", r);
    (*view).set_ivar::<f64>("_strokeG", g);
    (*view).set_ivar::<f64>("_strokeB", b);
    (*view).set_ivar::<f64>("_strokeA", a);
    (*view).set_ivar::<f64>("_fillTransparencyPct", clamp(fill_t, 0.0, 100.0));
    (*view).set_ivar::<i32>("_lang", if lang == 1 { 1 } else { 0 });
}

//
// ===================== Utilities / Localization =====================
//

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn color_to_hex(r: f64, g: f64, b: f64, a: f64) -> String {
    let ri = (clamp(r, 0.0, 1.0) * 255.0).round() as u8;
    let gi = (clamp(g, 0.0, 1.0) * 255.0).round() as u8;
    let bi = (clamp(b, 0.0, 1.0) * 255.0).round() as u8;
    let ai = (clamp(a, 0.0, 1.0) * 255.0).round() as u8;
    if ai >= 255 {
        format!("#{:02X}{:02X}{:02X}", ri, gi, bi)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", ri, gi, bi, ai)
    }
}

fn parse_hex_color(s: &str) -> Option<(f64, f64, f64, f64)> {
    let t = s.trim();
    let t = t.strip_prefix('#').unwrap_or(t);
    let hex = t.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let (r, g, b, a) = match hex.len() {
        6 => {
            let rv = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let gv = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let bv = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (rv, gv, bv, 255u8)
        }
        8 => {
            let rv = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let gv = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let bv = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let av = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (rv, gv, bv, av)
        }
        _ => return None,
    };
    Some((
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    ))
}

// Localization via Cow to avoid lifetime issues
fn tr_key(key: &str, es: bool) -> Cow<'static, str> {
    match (key, es) {
        ("Settings", true) => Cow::Borrowed("Configuración"),
        ("Settings", false) => Cow::Borrowed("Settings"),

        ("Language", true) => Cow::Borrowed("Idioma"),
        ("Language", false) => Cow::Borrowed("Language"),

        ("English", true) => Cow::Borrowed("Inglés"),
        ("English", false) => Cow::Borrowed("English"),

        ("Spanish", true) => Cow::Borrowed("Español"),
        ("Spanish", false) => Cow::Borrowed("Spanish"),

        ("Radius (px)", true) => Cow::Borrowed("Radio (px)"),
        ("Radius (px)", false) => Cow::Borrowed("Radius (px)"),

        ("Border (px)", true) => Cow::Borrowed("Grosor (px)"),
        ("Border (px)", false) => Cow::Borrowed("Border (px)"),

        ("Color", true) => Cow::Borrowed("Color"),
        ("Color", false) => Cow::Borrowed("Color"),

        ("Hex", true) => Cow::Borrowed("Hex"),
        ("Hex", false) => Cow::Borrowed("Hex"),

        ("Fill Transparency (%)", true) => Cow::Borrowed("Transparencia (%)"),
        ("Fill Transparency (%)", false) => Cow::Borrowed("Fill Transparency (%)"),

        ("Close", true) => Cow::Borrowed("Cerrar"),
        ("Close", false) => Cow::Borrowed("Close"),

        _ => Cow::Owned(key.to_string()),
    }
}

fn lang_is_es(view: id) -> bool {
    unsafe {
        let l = *(*view).get_ivar::<i32>("_lang");
        l == 1
    }
}

//
// ===================== Multi-monitor helpers =====================
//

/// Aplica un closure a todos los contentView de clase CustomViewMulti
unsafe fn apply_to_all_views<F: Fn(id)>(f: F) {
    let app: id = NSApp();
    let windows: id = msg_send![app, windows];
    let wcount: usize = msg_send![windows, count];

    let custom_cls = Class::get("CustomViewMulti").unwrap();

    for j in 0..wcount {
        let win: id = msg_send![windows, objectAtIndex: j];
        let view: id = msg_send![win, contentView];
        if view != nil {
            let is_custom: bool = msg_send![view, isKindOfClass: custom_cls];
            if is_custom {
                f(view);
            }
        }
    }
}

/// Copia prefs visuales de src a todas las vistas
unsafe fn sync_visual_prefs_to_all_views(src: id) {
    let radius = *(*src).get_ivar::<f64>("_radius");
    let border = *(*src).get_ivar::<f64>("_borderWidth");
    let r = *(*src).get_ivar::<f64>("_strokeR");
    let g = *(*src).get_ivar::<f64>("_strokeG");
    let b = *(*src).get_ivar::<f64>("_strokeB");
    let a = *(*src).get_ivar::<f64>("_strokeA");
    let fill_t = *(*src).get_ivar::<f64>("_fillTransparencyPct");
    let lang = *(*src).get_ivar::<i32>("_lang");

    apply_to_all_views(|v| {
        unsafe {
            (*v).set_ivar::<f64>("_radius", radius);
            (*v).set_ivar::<f64>("_borderWidth", border);
            (*v).set_ivar::<f64>("_strokeR", r);
            (*v).set_ivar::<f64>("_strokeG", g);
            (*v).set_ivar::<f64>("_strokeB", b);
            (*v).set_ivar::<f64>("_strokeA", a);
            (*v).set_ivar::<f64>("_fillTransparencyPct", fill_t);
            (*v).set_ivar::<i32>("_lang", lang);
        }
    });
}

//
// ===================== Settings window =====================
//

fn ensure_hotkey_menu(view: id) {
    unsafe {
        let installed = *(*view).get_ivar::<bool>("_menuInstalled");
        if installed {
            return;
        }

        let main_menu: id = msg_send![class!(NSMenu), new];
        let app_item: id = msg_send![class!(NSMenuItem), new];
        let _: () = msg_send![main_menu, addItem: app_item];

        let app_menu: id = msg_send![class!(NSMenu), new];
        let _: () = msg_send![app_item, setSubmenu: app_menu];

        let mi_toggle: id = msg_send![class!(NSMenuItem), alloc];
        let mi_toggle: id = msg_send![
            mi_toggle,
            initWithTitle: nsstring("Toggle Overlay")
            action: sel!(requestToggle)
            keyEquivalent: nsstring("a")
        ];
        let ctrl_mask: u64 = 1 << 18; // Control
        let _: () = msg_send![mi_toggle, setKeyEquivalentModifierMask: ctrl_mask];
        let _: () = msg_send![mi_toggle, setTarget: view];
        let _: () = msg_send![app_menu, addItem: mi_toggle];

        let app = NSApp();
        let _: () = msg_send![app, setMainMenu: main_menu];

        (*view).set_ivar::<bool>("_menuInstalled", true);
    }
}

fn open_settings_window(view: id) {
    unsafe {
        let existing: id = *(*view).get_ivar::<id>("_settingsWindow");
        if existing != nil {
            let _: () = msg_send![existing, makeKeyAndOrderFront: nil];
            return;
        }

        // Pasa a Regular para mostrar la ventana y enfoque
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];

        // Menú + hotkeys activos
        ensure_hotkey_menu(view);
        reinstall_hotkeys(view);

        // Mantén overlays arriba y refresca
        apply_to_all_views(|v| {
            unsafe {
                let overlay_win: id = msg_send![v, window];
                let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
                let _: () = msg_send![overlay_win, orderFrontRegardless];
                let _: () = msg_send![
                    v,
                    performSelectorOnMainThread: sel!(update_cursor_multi)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        });

        let es = lang_is_es(view);

        let style = NSWindowStyleMask::NSTitledWindowMask
            | NSWindowStyleMask::NSClosableWindowMask
            | NSWindowStyleMask::NSMiniaturizableWindowMask;
        let w = 520.0;
        let h = 330.0; // espacio extra selector idioma
        let settings = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)),
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );
        let _: () = msg_send![settings, setTitle: nsstring(tr_key("Settings", es).as_ref())];
        let _: () = msg_send![settings, center];

        let content: id = msg_send![settings, contentView];

        let radius: f64 = *(*view).get_ivar::<f64>("_radius");
        let border: f64 = *(*view).get_ivar::<f64>("_borderWidth");
        let r: f64 = *(*view).get_ivar::<f64>("_strokeR");
        let g: f64 = *(*view).get_ivar::<f64>("_strokeG");
        let b: f64 = *(*view).get_ivar::<f64>("_strokeB");
        let a: f64 = *(*view).get_ivar::<f64>("_strokeA");
        let fill_t: f64 = *(*view).get_ivar::<f64>("_fillTransparencyPct");
        let cur_lang: i32 = *(*view).get_ivar::<i32>("_lang");

        // Helper label
        let mk_label = |x, y, text: &str| -> id {
            let lbl: id = msg_send![class!(NSTextField), alloc];
            let lbl: id = msg_send![
                lbl,
                initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(180.0, 20.0))
            ];
            let _: () = msg_send![lbl, setBezeled: NO];
            let _: () = msg_send![lbl, setDrawsBackground: NO];
            let _: () = msg_send![lbl, setEditable: NO];
            let _: () = msg_send![lbl, setSelectable: NO];
            let _: () = msg_send![lbl, setStringValue: nsstring(text)];
            lbl
        };

        // Helper value-label (no interactivo)
        let mk_value_label = |x, y, w, h, val: &str| -> id {
            let tf: id = msg_send![class!(NSTextField), alloc];
            let tf: id =
                msg_send![tf, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))];
            let _: () = msg_send![tf, setBezeled: NO];
            let _: () = msg_send![tf, setDrawsBackground: NO];
            let _: () = msg_send![tf, setEditable: NO];
            let _: () = msg_send![tf, setSelectable: NO];
            let _: () = msg_send![tf, setStringValue: nsstring(val)];
            tf
        };

        // Idioma
        let label_lang = mk_label(20.0, h - 40.0, tr_key("Language", es).as_ref());
        let popup_lang: id = msg_send![class!(NSPopUpButton), alloc];
        let popup_lang: id = msg_send![
            popup_lang,
            initWithFrame: NSRect::new(NSPoint::new(160.0, h - 44.0), NSSize::new(160.0, 24.0))
        ];
        let _: () = msg_send![popup_lang, addItemWithTitle: nsstring(tr_key("English", es).as_ref())];
        let _: () = msg_send![popup_lang, addItemWithTitle: nsstring(tr_key("Spanish", es).as_ref())];
        let _: () = msg_send![popup_lang, selectItemAtIndex: (if cur_lang == 1 { 1 } else { 0 })];
        let _: () = msg_send![popup_lang, setTarget: view];
        let _: () = msg_send![popup_lang, setAction: sel!(langChanged:)];

        // Labels estáticos
        let label_radius = mk_label(20.0, h - 80.0, tr_key("Radius (px)", es).as_ref());
        let label_border = mk_label(20.0, h - 130.0, tr_key("Border (px)", es).as_ref());
        let label_color = mk_label(20.0, h - 180.0, tr_key("Color", es).as_ref());
        let label_hex = mk_label(220.0, h - 180.0, tr_key("Hex", es).as_ref());
        let _: () = msg_send![label_hex, sizeToFit];
        let label_fill_t = mk_label(20.0, h - 230.0, tr_key("Fill Transparency (%)", es).as_ref());

        // Value labels (no interactivos) + sliders (SIN marcas visuales)
        // Radius (valor como etiqueta)
        let field_radius = mk_value_label(160.0, h - 84.0, 60.0, 24.0, &format!("{:.0}", radius));
        let slider_radius: id = msg_send![class!(NSSlider), alloc];
        let slider_radius: id = msg_send![
            slider_radius,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 85.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_radius, setMinValue: 5.0f64];
        let _: () = msg_send![slider_radius, setMaxValue: 200.0f64];
        let _: () = msg_send![slider_radius, setDoubleValue: radius];
        let _: () = msg_send![slider_radius, setTarget: view];
        let _: () = msg_send![slider_radius, setAction: sel!(setRadius:)];
        let _: () = msg_send![slider_radius, setContinuous: YES];
        // (sin tick marks)

        // Border (valor como etiqueta)
        let field_border = mk_value_label(160.0, h - 134.0, 60.0, 24.0, &format!("{:.0}", border));
        let slider_border: id = msg_send![class!(NSSlider), alloc];
        let slider_border: id = msg_send![
            slider_border,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 135.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_border, setMinValue: 1.0f64];
        let _: () = msg_send![slider_border, setMaxValue: 20.0f64];
        let _: () = msg_send![slider_border, setDoubleValue: border];
        let _: () = msg_send![slider_border, setTarget: view];
        let _: () = msg_send![slider_border, setAction: sel!(setBorderWidth:)];
        let _: () = msg_send![slider_border, setContinuous: YES];
        // (sin tick marks)

        // Color + Hex (Hex se mantiene editable)
        let color_well: id = msg_send![class!(NSColorWell), alloc];
        let color_well: id = msg_send![
            color_well,
            initWithFrame: NSRect::new(NSPoint::new(160.0, h - 185.0), NSSize::new(50.0, 25.0))
        ];
        let ns_color = Class::get("NSColor").unwrap();
        let current_color: id =
            msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
        let _: () = msg_send![color_well, setColor: current_color];
        let _: () = msg_send![color_well, setTarget: view];
        let _: () = msg_send![color_well, setAction: sel!(colorChanged:)];

        let hex_str = color_to_hex(r, g, b, a);
        // Campo Hex a la derecha del label "Hex"
        let label_hex_frame: NSRect = msg_send![label_hex, frame];
        let padding: f64 = 8.0;
        let right_margin: f64 = 175.0;
        let field_x = label_hex_frame.origin.x + label_hex_frame.size.width + padding;
        let field_w = (w - right_margin) - field_x;

        let field_hex: id = msg_send![class!(NSTextField), alloc];
        let field_hex: id = msg_send![
            field_hex,
            initWithFrame: NSRect::new(NSPoint::new(field_x, h - 185.0), NSSize::new(field_w, 24.0))
        ];
        let _: () = msg_send![field_hex, setStringValue: nsstring(&hex_str)];
        the_hex_field_config(view, field_hex);

        // Fill transparency (valor como etiqueta)
        let field_fill_t = mk_value_label(160.0, h - 234.0, 60.0, 24.0, &format!("{:.0}", fill_t));
        let slider_fill_t: id = msg_send![class!(NSSlider), alloc];
        let slider_fill_t: id = msg_send![
            slider_fill_t,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 235.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_fill_t, setMinValue: 0.0f64];
        let _: () = msg_send![slider_fill_t, setMaxValue: 100.0f64];
        let _: () = msg_send![slider_fill_t, setDoubleValue: fill_t];
        let _: () = msg_send![slider_fill_t, setTarget: view];
        let _: () = msg_send![slider_fill_t, setAction: sel!(setFillTransparency:)];
        let _: () = msg_send![slider_fill_t, setContinuous: YES];
        // (sin tick marks)

        // Close
        let btn_close: id = msg_send![class!(NSButton), alloc];
        let btn_close: id = msg_send![
            btn_close,
            initWithFrame: NSRect::new(NSPoint::new(w - 100.0, 15.0), NSSize::new(80.0, 28.0))
        ];
        let _: () = msg_send![btn_close, setTitle: nsstring(tr_key("Close", es).as_ref())];
        let _: () = msg_send![btn_close, setTarget: view];
        let _: () = msg_send![btn_close, setAction: sel!(closeSettings:)];

        // Enter/Return activa “Cerrar”
        let _: () = msg_send![btn_close, setKeyEquivalent: nsstring("\r")];
        let cell: id = msg_send![btn_close, cell];
        let _: () = msg_send![settings, setDefaultButtonCell: cell];

        // Añadir subviews
        let _: () = msg_send![content, addSubview: label_lang];
        let _: () = msg_send![content, addSubview: popup_lang];

        let _: () = msg_send![content, addSubview: label_radius];
        let _: () = msg_send![content, addSubview: field_radius];
        let _: () = msg_send![content, addSubview: slider_radius];

        let _: () = msg_send![content, addSubview: label_border];
        let _: () = msg_send![content, addSubview: field_border];
        let _: () = msg_send![content, addSubview: slider_border];

        let _: () = msg_send![content, addSubview: label_color];
        let _: () = msg_send![content, addSubview: color_well];
        let _: () = msg_send![content, addSubview: label_hex];
        let _: () = msg_send![content, addSubview: field_hex];

        let _: () = msg_send![content, addSubview: label_fill_t];
        let _: () = msg_send![content, addSubview: field_fill_t];
        let _: () = msg_send![content, addSubview: slider_fill_t];

        let _: () = msg_send![content, addSubview: btn_close];

        // Guardar refs
        (*view).set_ivar::<id>("_settingsWindow", settings);
        (*view).set_ivar::<id>("_labelLang", label_lang);
        (*view).set_ivar::<id>("_popupLang", popup_lang);

        (*view).set_ivar::<id>("_labelRadius", label_radius);
        (*view).set_ivar::<id>("_fieldRadius", field_radius); // etiqueta
        (*view).set_ivar::<id>("_sliderRadius", slider_radius);

        (*view).set_ivar::<id>("_labelBorder", label_border);
        (*view).set_ivar::<id>("_fieldBorder", field_border); // etiqueta
        (*view).set_ivar::<id>("_sliderBorder", slider_border);

        (*view).set_ivar::<id>("_labelColor", label_color);
        (*view).set_ivar::<id>("_colorWell", color_well);

        (*view).set_ivar::<id>("_labelHex", label_hex);
        (*view).set_ivar::<id>("_fieldHex", field_hex); // editable

        (*view).set_ivar::<id>("_labelFillT", label_fill_t);
        (*view).set_ivar::<id>("_fieldFillT", field_fill_t); // etiqueta
        (*view).set_ivar::<id>("_sliderFillT", slider_fill_t);

        (*view).set_ivar::<id>("_btnClose", btn_close);

        // 👉 Foco por defecto en “Cerrar”
        let _: () = msg_send![settings, setInitialFirstResponder: btn_close];

        // Mostrar ventana
        let _: () = msg_send![settings, makeKeyAndOrderFront: nil];

        // Asegurar foco cuando ya es keyWindow
        let _: () = msg_send![settings, makeFirstResponder: btn_close];

        // 👉 ESC cierra la ventana de configuración (monitor local)
        const KEY_DOWN_MASK: u64 = 1 << 10;
        let settings_for_block = settings;
        let host_view = view;
        let esc_block = ConcreteBlock::new(move |event: id| -> id {
            unsafe {
                // Sólo si la ventana de configuración es la key window
                let app = NSApp();
                let key_win: id = msg_send![app, keyWindow];
                if key_win == settings_for_block {
                    let keycode: u16 = msg_send![event, keyCode];
                    if keycode == 53 {
                        // 53 = Escape
                        let _: () = msg_send![
                            host_view,
                            performSelectorOnMainThread: sel!(closeSettings:)
                            withObject: nil
                            waitUntilDone: NO
                        ];
                        return nil; // consumir el evento
                    }
                }
            }
            event
        })
            .copy();
        let esc_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
            handler: &*esc_block
        ];
        (*view).set_ivar::<id>("_settingsEscMonitor", esc_mon);
    }
}

unsafe fn the_hex_field_config(view: id, field_hex: id) {
    let _: () = msg_send![field_hex, setBezeled: YES];
    let _: () = msg_send![field_hex, setDrawsBackground: YES];
    let _: () = msg_send![field_hex, setEditable: YES];
    let _: () = msg_send![field_hex, setSelectable: YES];
    let _: () = msg_send![field_hex, setTarget: view];
    let _: () = msg_send![field_hex, setAction: sel!(hexChanged:)];
}

fn close_settings_window(view: id) {
    unsafe {
        // Quitar monitor de ESC si existiera
        let esc_mon: id = *(*view).get_ivar::<id>("_settingsEscMonitor");
        if esc_mon != nil {
            let _: () = msg_send![class!(NSEvent), removeMonitor: esc_mon];
            (*view).set_ivar::<id>("_settingsEscMonitor", nil);
        }

        let settings: id = *(*view).get_ivar::<id>("_settingsWindow");
        if settings != nil {
            let _: () = msg_send![settings, orderOut: nil];
            (*view).set_ivar::<id>("_settingsWindow", nil);
        }
        // De vuelta a Accessory
        let app = NSApp();
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );

        // Recoloca overlays y refresca
        apply_to_all_views(|v| {
            unsafe {
                let overlay_win: id = msg_send![v, window];
                let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
                let _: () = msg_send![overlay_win, orderFrontRegardless];
                let _: () = msg_send![
                    v,
                    performSelectorOnMainThread: sel!(update_cursor_multi)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        });

        // Reinstala hotkeys
        reinstall_hotkeys(view);
    }
}

//
// ===================== Multi-monitor view =====================
//

unsafe fn make_window_for_screen(screen: id) -> (id, id) {
    let frame: NSRect = msg_send![screen, frame];

    let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
        frame,
        NSWindowStyleMask::NSBorderlessWindowMask,
        NSBackingStoreType::NSBackingStoreBuffered,
        NO,
    );
    window.setOpaque_(NO);
    window.setBackgroundColor_(NSColor::clearColor(nil));
    window.setIgnoresMouseEvents_(YES);
    window.setAcceptsMouseMovedEvents_(YES); // aceptar movimientos
    window.setLevel_(overlay_window_level().into());
    window.setCollectionBehavior_(
        NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
    );

    let view: id =
        register_custom_view_class_and_create_view(window, frame.size.width, frame.size.height);
    (*view).set_ivar::<id>("_ownScreen", screen);
    // Guardar DisplayID estable
    let did = display_id_for_screen(screen);
    (*view).set_ivar::<u32>("_ownDisplayID", did);

    (window, view)
}

unsafe fn register_custom_view_class_and_create_view(window: id, width: f64, height: f64) -> id {
    let class_name = "CustomViewMulti";
    let custom_view_class = if let Some(cls) = Class::get(class_name) {
        cls
    } else {
        let superclass = Class::get("NSView").unwrap();
        let mut decl = ClassDecl::new(class_name, superclass).unwrap();

        // Base state
        decl.add_ivar::<f64>("_cursorXScreen");
        decl.add_ivar::<f64>("_cursorYScreen");
        decl.add_ivar::<bool>("_visible"); // visible por selección de pantalla
        decl.add_ivar::<bool>("_overlayEnabled"); // toggle global
        decl.add_ivar::<i32>("_displayMode"); // 0=circle, 1=L, 2=R
        decl.add_ivar::<id>("_ownScreen"); // NSScreen
        decl.add_ivar::<u32>("_ownDisplayID"); // DisplayID estable
        decl.add_ivar::<i32>("_lang"); // 0=en, 1=es

        // Visual parameters
        decl.add_ivar::<f64>("_radius");
        decl.add_ivar::<f64>("_borderWidth");
        decl.add_ivar::<f64>("_strokeR");
        decl.add_ivar::<f64>("_strokeG");
        decl.add_ivar::<f64>("_strokeB");
        decl.add_ivar::<f64>("_strokeA");
        decl.add_ivar::<f64>("_fillTransparencyPct"); // 0..100

        // Carbon refs
        decl.add_ivar::<*mut std::ffi::c_void>("_hkHandler");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkToggle");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkComma");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkSemi");

        // Keep-alive timer para hotkeys
        decl.add_ivar::<id>("_hkKeepAliveTimer");

        // Global mouse monitors
        decl.add_ivar::<id>("_monLeftDown");
        decl.add_ivar::<id>("_monLeftUp");
        decl.add_ivar::<id>("_monRightDown");
        decl.add_ivar::<id>("_monRightUp");
        decl.add_ivar::<id>("_monMove");

        // Settings UI refs
        decl.add_ivar::<id>("_settingsWindow");
        decl.add_ivar::<id>("_labelLang");
        decl.add_ivar::<id>("_popupLang");

        decl.add_ivar::<id>("_labelRadius");
        decl.add_ivar::<id>("_fieldRadius"); // ahora etiqueta
        decl.add_ivar::<id>("_sliderRadius");

        decl.add_ivar::<id>("_labelBorder");
        decl.add_ivar::<id>("_fieldBorder"); // ahora etiqueta
        decl.add_ivar::<id>("_sliderBorder");

        decl.add_ivar::<id>("_labelColor");
        decl.add_ivar::<id>("_colorWell");

        decl.add_ivar::<id>("_labelHex");
        decl.add_ivar::<id>("_fieldHex"); // sigue editable

        decl.add_ivar::<id>("_labelFillT");
        decl.add_ivar::<id>("_fieldFillT"); // ahora etiqueta
        decl.add_ivar::<id>("_sliderFillT");

        decl.add_ivar::<id>("_btnClose");

        // Monitor local (Ctrl+A)
        decl.add_ivar::<id>("_localKeyMonitor");

        // Debounce
        decl.add_ivar::<f64>("_lastToggleTs");

        // Menú instalado
        decl.add_ivar::<bool>("_menuInstalled");

        // Timer de refresco
        decl.add_ivar::<id>("_updateTimer");

        // Monitor local para ESC en settings
        decl.add_ivar::<id>("_settingsEscMonitor");

        // ====== Methods ======

        extern "C" fn update_cursor_multi(this: &mut Object, _cmd: Sel) {
            unsafe {
                let (x, y) = get_mouse_position_cocoa();
                let screens: id = msg_send![class!(NSScreen), screens];
                let count: usize = msg_send![screens, count];

                // Determina la pantalla bajo el cursor por DisplayID (estable)
                let mut target_id: u32 = 0;
                for i in 0..count {
                    let s: id = msg_send![screens, objectAtIndex: i];
                    let f: NSRect = msg_send![s, frame];
                    if x >= f.origin.x
                        && x <= f.origin.x + f.size.width
                        && y >= f.origin.y
                        && y <= f.origin.y + f.size.height
                    {
                        target_id = display_id_for_screen(s);
                        break;
                    }
                }

                let enabled = *this.get_ivar::<bool>("_overlayEnabled");

                apply_to_all_views(|v| {
                    unsafe {
                        *(*v).get_mut_ivar::<f64>("_cursorXScreen") = x;
                        *(*v).get_mut_ivar::<f64>("_cursorYScreen") = y;
                        let own_id = *(*v).get_ivar::<u32>("_ownDisplayID");
                        let vis = enabled && own_id == target_id && target_id != 0;
                        *(*v).get_mut_ivar::<bool>("_visible") = vis;
                        let _: () = msg_send![v, setNeedsDisplay: YES];
                        // Fuerza la ventana a repintar si es necesario (importante tras abrir config)
                        let win: id = msg_send![v, window];
                        let _: () = msg_send![win, displayIfNeeded];
                    }
                });
            }
        }

        extern "C" fn toggle_visibility(this: &mut Object, _cmd: Sel) {
            unsafe {
                let enabled = *this.get_ivar::<bool>("_overlayEnabled");
                let new_enabled = !enabled;

                apply_to_all_views(|v| {
                    unsafe {
                        *(*v).get_mut_ivar::<bool>("_overlayEnabled") = new_enabled;
                    }
                });

                if new_enabled {
                    let _: () = msg_send![
                        this,
                        performSelectorOnMainThread: sel!(update_cursor_multi)
                        withObject: nil
                        waitUntilDone: NO
                    ];
                } else {
                    apply_to_all_views(|v| {
                        unsafe {
                            *(*v).get_mut_ivar::<bool>("_visible") = false;
                            let _: () = msg_send![v, setNeedsDisplay: YES];
                            let win: id = msg_send![v, window];
                            let _: () = msg_send![win, displayIfNeeded];
                        }
                    });
                }
            }
        }

        // Debounced toggle para menú/teclas locales
        extern "C" fn request_toggle(this: &mut Object, _cmd: Sel) {
            unsafe {
                let now = CFAbsoluteTimeGetCurrent();
                let last = *this.get_ivar::<f64>("_lastToggleTs");
                if now - last < 0.15 {
                    return;
                }
                apply_to_all_views(|v| unsafe {
                    *(*v).get_mut_ivar::<f64>("_lastToggleTs") = now;
                });

                let _: () = msg_send![
                    this,
                    performSelectorOnMainThread: sel!(toggleVisibility)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        }

        // Hotkey keep-alive: re-install periódicamente
        extern "C" fn hotkey_keepalive(this: &mut Object, _cmd: Sel) {
            unsafe {
                reinstall_hotkeys(this as *mut _ as id);
            }
        }

        // ===== Settings actions (apply to ALL views) =====
        extern "C" fn set_radius(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = (v / 5.0).round() * 5.0;
                v = clamp(v, 5.0, 200.0);

                // actualiza etiqueta inmediata
                let field: id = *this.get_ivar("_fieldRadius");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_RADIUS, v);
                apply_to_all_views(|vv| unsafe { (*vv).set_ivar::<f64>("_radius", v) });
                apply_to_all_views(|vv| unsafe { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        // Ya no se usan (campos no editables), se dejan vacíos por compat
        extern "C" fn set_radius_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn set_border_width(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = clamp(v.round(), 1.0, 20.0); // step 1

                let field: id = *this.get_ivar("_fieldBorder");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_BORDER, v);
                apply_to_all_views(|vv| unsafe { (*vv).set_ivar::<f64>("_borderWidth", v) });
                apply_to_all_views(|vv| unsafe { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn set_border_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn set_fill_transparency(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = (v / 5.0).round() * 5.0; // step 5
                v = clamp(v, 0.0, 100.0);

                let field: id = *this.get_ivar("_fieldFillT");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_FILL_T, v);
                apply_to_all_views(|vv| unsafe { (*vv).set_ivar::<f64>("_fillTransparencyPct", v) });
                apply_to_all_views(|vv| unsafe { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn set_fill_transparency_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn color_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let color: id = msg_send![sender, color];
                let r: f64 = msg_send![color, redComponent];
                let g: f64 = msg_send![color, greenComponent];
                let b: f64 = msg_send![color, blueComponent];
                let a: f64 = msg_send![color, alphaComponent];

                prefs_set_double(PREF_R, r);
                prefs_set_double(PREF_G, g);
                prefs_set_double(PREF_B, b);
                prefs_set_double(PREF_A, a);

                apply_to_all_views(|vv| unsafe {
                    (*vv).set_ivar::<f64>("_strokeR", r);
                    (*vv).set_ivar::<f64>("_strokeG", g);
                    (*vv).set_ivar::<f64>("_strokeB", b);
                    (*vv).set_ivar::<f64>("_strokeA", a);
                });

                let hex_field: id = *this.get_ivar("_fieldHex");
                if hex_field != nil {
                    let s = color_to_hex(r, g, b, a);
                    let _: () = msg_send![hex_field, setStringValue: nsstring(&s)];
                }
                apply_to_all_views(|vv| unsafe { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn hex_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let s: id = msg_send![sender, stringValue];
                let cstr_ptr: *const std::os::raw::c_char = msg_send![s, UTF8String];
                if !cstr_ptr.is_null() {
                    let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
                    if let Some((r, g, b, a)) = parse_hex_color(&txt) {
                        prefs_set_double(PREF_R, r);
                        prefs_set_double(PREF_G, g);
                        prefs_set_double(PREF_B, b);
                        prefs_set_double(PREF_A, a);

                        apply_to_all_views(|vv| unsafe {
                            (*vv).set_ivar::<f64>("_strokeR", r);
                            (*vv).set_ivar::<f64>("_strokeG", g);
                            (*vv).set_ivar::<f64>("_strokeB", b);
                            (*vv).set_ivar::<f64>("_strokeA", a);
                        });

                        let ns_color = Class::get("NSColor").unwrap();
                        let col: id = msg_send![
                            ns_color,
                            colorWithCalibratedRed: r
                            green: g
                            blue: b
                            alpha: a
                        ];
                        let well: id = *this.get_ivar("_colorWell");
                        if well != nil {
                            let _: () = msg_send![well, setColor: col];
                        }
                        let norm = color_to_hex(r, g, b, a);
                        let _: () = msg_send![sender, setStringValue: nsstring(&norm)];
                        apply_to_all_views(|vv| unsafe { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
                    } else {
                        let r = *this.get_ivar::<f64>("_strokeR");
                        let g = *this.get_ivar::<f64>("_strokeG");
                        let b = *this.get_ivar::<f64>("_strokeB");
                        let a = *this.get_ivar::<f64>("_strokeA");
                        let norm = color_to_hex(r, g, b, a);
                        let _: () = msg_send![sender, setStringValue: nsstring(&norm)];
                    }
                }
            }
        }
        extern "C" fn close_settings(this: &mut Object, _cmd: Sel, _sender: id) {
            let view: id = this as *mut _ as id;
            close_settings_window(view);
        }

        // Cambio de idioma + ajuste de layout del campo Hex
        extern "C" fn lang_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let idx: i32 = msg_send![sender, indexOfSelectedItem];
                let new_lang = if idx == 1 { 1 } else { 0 };

                prefs_set_int(PREF_LANG, new_lang);
                apply_to_all_views(|v| unsafe { (*v).set_ivar::<i32>("_lang", new_lang) });

                let es = new_lang == 1;

                let settings: id = *this.get_ivar("_settingsWindow");
                if settings != nil {
                    let _: () =
                        msg_send![settings, setTitle: nsstring(tr_key("Settings", es).as_ref())];
                }

                let label_lang: id = *this.get_ivar("_labelLang");
                if label_lang != nil {
                    let _: () = msg_send![
                        label_lang,
                        setStringValue: nsstring(tr_key("Language", es).as_ref())
                    ];
                }

                let popup: id = *this.get_ivar("_popupLang");
                if popup != nil {
                    let _: () = msg_send![popup, removeAllItems];
                    let _: () = msg_send![
                        popup,
                        addItemWithTitle: nsstring(tr_key("English", es).as_ref())
                    ];
                    let _: () = msg_send![
                        popup,
                        addItemWithTitle: nsstring(tr_key("Spanish", es).as_ref())
                    ];
                    let _: () = msg_send![popup, selectItemAtIndex: (if es { 1 } else { 0 })];
                }

                let lr: id = *this.get_ivar("_labelRadius");
                if lr != nil {
                    let _: () =
                        msg_send![lr, setStringValue: nsstring(tr_key("Radius (px)", es).as_ref())];
                }
                let lb: id = *this.get_ivar("_labelBorder");
                if lb != nil {
                    let _: () =
                        msg_send![lb, setStringValue: nsstring(tr_key("Border (px)", es).as_ref())];
                }
                let lc: id = *this.get_ivar("_labelColor");
                if lc != nil {
                    let _: () =
                        msg_send![lc, setStringValue: nsstring(tr_key("Color", es).as_ref())];
                }
                let lhex: id = *this.get_ivar("_labelHex");
                if lhex != nil {
                    let _: () =
                        msg_send![lhex, setStringValue: nsstring(tr_key("Hex", es).as_ref())];
                    let _: () = msg_send![lhex, sizeToFit];

                    let field_hex: id = *this.get_ivar("_fieldHex");
                    if field_hex != nil && settings != nil {
                        let wframe: NSRect = msg_send![settings, frame];
                        let w = wframe.size.width;

                        let label_hex_frame: NSRect = msg_send![lhex, frame];
                        let padding: f64 = 8.0;
                        let right_margin: f64 = 175.0;
                        let field_x =
                            label_hex_frame.origin.x + label_hex_frame.size.width + padding;
                        let field_w = (w - right_margin) - field_x;

                        let mut fh_frame: NSRect = msg_send![field_hex, frame];
                        fh_frame.origin.x = field_x;
                        fh_frame.size.width = field_w;
                        let _: () = msg_send![field_hex, setFrame: fh_frame];
                    }
                }
                let lfill: id = *this.get_ivar("_labelFillT");
                if lfill != nil {
                    let _: () = msg_send![
                        lfill,
                        setStringValue: nsstring(tr_key("Fill Transparency (%)", es).as_ref())
                    ];
                }

                let btn: id = *this.get_ivar("_btnClose");
                if btn != nil {
                    let _: () = msg_send![btn, setTitle: nsstring(tr_key("Close", es).as_ref())];
                }
            }
        }

        // Delegate ya no se usa (campos no editables), lo dejamos vacío para compat
        extern "C" fn control_text_did_change(_this: &mut Object, _cmd: Sel, _notif: id) {}

        // ===== Drawing (circle or L/R letter) =====
        extern "C" fn draw_rect(this: &Object, _cmd: Sel, _rect: NSRect) {
            unsafe {
                let sx = *this.get_ivar::<f64>("_cursorXScreen");
                let sy = *this.get_ivar::<f64>("_cursorYScreen");
                let visible = *this.get_ivar::<bool>("_visible");
                if !visible {
                    return;
                }

                // Convert screen → window → view
                let screen_pt = NSPoint::new(sx, sy);
                let screen_rect = NSRect::new(screen_pt, NSSize::new(0.0, 0.0));
                let win: id = msg_send![this, window];
                let win_rect: NSRect = msg_send![win, convertRectFromScreen: screen_rect];
                let win_pt = win_rect.origin;
                let view_pt: NSPoint = msg_send![this, convertPoint: win_pt fromView: nil];

                let radius = *this.get_ivar::<f64>("_radius");
                let border_width = *this.get_ivar::<f64>("_borderWidth");
                let r = *this.get_ivar::<f64>("_strokeR");
                let g = *this.get_ivar::<f64>("_strokeG");
                let b = *this.get_ivar::<f64>("_strokeB");
                let a = *this.get_ivar::<f64>("_strokeA");
                let fill_t = *this.get_ivar::<f64>("_fillTransparencyPct");
                let mode = *this.get_ivar::<i32>("_displayMode");

                let ns_color = Class::get("NSColor").unwrap();

                if mode == 0 {
                    // Circle
                    let rect = NSRect::new(
                        NSPoint::new(view_pt.x - radius, view_pt.y - radius),
                        NSSize::new(radius * 2.0, radius * 2.0),
                    );
                    let ns_bezier = Class::get("NSBezierPath").unwrap();
                    let circle: id = msg_send![ns_bezier, bezierPathWithOvalInRect: rect];

                    // Fill
                    let fill_alpha = a * (1.0 - clamp(fill_t, 0.0, 100.0) / 100.0);
                    if fill_alpha > 0.0 {
                        let fill: id = msg_send![
                            ns_color,
                            colorWithCalibratedRed: r
                            green: g
                            blue: b
                            alpha: fill_alpha
                        ];
                        let _: () = msg_send![fill, set];
                        let _: () = msg_send![circle, fill];
                    }
                    // Stroke
                    let stroke: id =
                        msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                    let _: () = msg_send![stroke, set];
                    let _: () = msg_send![circle, setLineWidth: border_width];
                    let _: () = msg_send![circle, stroke];
                    return;
                }

                // Letter L/R
                let target_letter_height = 3.0 * radius; // 1.5 × diámetro
                let font_class = Class::get("NSFont").unwrap();
                let font: id = msg_send![font_class, boldSystemFontOfSize: target_letter_height];

                let font_name: id = msg_send![font, fontName];
                let ct_font: CTFontRef =
                    CTFontCreateWithName(font_name as *const _, target_letter_height, std::ptr::null());

                let ch_u16: u16 = if mode == 1 { 'L' as u16 } else { 'R' as u16 };
                let mut glyph: u16 = 0;
                let mapped = CTFontGetGlyphsForCharacters(
                    ct_font,
                    &ch_u16 as *const u16,
                    &mut glyph as *mut u16,
                    1,
                );
                if !mapped || glyph == 0 {
                    CFRelease(ct_font as *const _);
                    return;
                }
                let cg_path: CGPathRef = CTFontCreatePathForGlyph(ct_font, glyph, std::ptr::null());
                if cg_path.is_null() {
                    CFRelease(ct_font as *const _);
                    return;
                }

                let ns_bezier = Class::get("NSBezierPath").unwrap();
                let path: id = msg_send![ns_bezier, bezierPathWithCGPath: cg_path];

                let pbounds: NSRect = msg_send![path, bounds];
                let mid_x = pbounds.origin.x + pbounds.size.width / 2.0;
                let mid_y = pbounds.origin.y + pbounds.size.height / 2.0;

                let ns_affine = Class::get("NSAffineTransform").unwrap();
                let transform: id = msg_send![ns_affine, transform];
                let dx = view_pt.x - mid_x;
                let dy = view_pt.y - mid_y;
                let _: () = msg_send![transform, translateXBy: dx yBy: dy];
                let _: () = msg_send![path, transformUsingAffineTransform: transform];

                let _: () = msg_send![path, setLineJoinStyle: 1u64 /* round */];

                // Fill como el círculo
                let fill_alpha = a * (1.0 - clamp(fill_t, 0.0, 100.0) / 100.0);
                if fill_alpha > 0.0 {
                    let fill: id = msg_send![
                        ns_color,
                        colorWithCalibratedRed: r
                        green: g
                        blue: b
                        alpha: fill_alpha
                    ];
                    let _: () = msg_send![fill, set];
                    let _: () = msg_send![path, fill];
                }
                // Stroke
                let stroke: id =
                    msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                let _: () = msg_send![stroke, set];
                let _: () = msg_send![path, setLineWidth: border_width];
                let _: () = msg_send![path, stroke];

                CGPathRelease(cg_path);
                CFRelease(ct_font as *const _);
            }
        }

        // Register methods (incluye delegado en vivo vacío)
        decl.add_method(
            sel!(update_cursor_multi),
            update_cursor_multi as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(toggleVisibility),
            toggle_visibility as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(requestToggle),
            request_toggle as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(hotkeyKeepAlive),
            hotkey_keepalive as extern "C" fn(&mut Object, Sel),
        );

        decl.add_method(sel!(setRadius:), set_radius as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(setRadiusFromField:),
            set_radius_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setBorderWidth:),
            set_border_width as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setBorderFromField:),
            set_border_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setFillTransparency:),
            set_fill_transparency as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setFillTransparencyFromField:),
            set_fill_transparency_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(colorChanged:), color_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(hexChanged:), hex_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(closeSettings:),
            close_settings as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(langChanged:), lang_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(controlTextDidChange:),
            control_text_did_change as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(drawRect:), draw_rect as extern "C" fn(&Object, Sel, NSRect));

        decl.register()
    };

    let view: id = msg_send![custom_view_class, alloc];
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let view: id = msg_send![view, initWithFrame: frame];

    // Estado inicial
    (*view).set_ivar::<f64>("_cursorXScreen", 0.0);
    (*view).set_ivar::<f64>("_cursorYScreen", 0.0);
    (*view).set_ivar::<bool>("_visible", false);
    (*view).set_ivar::<bool>("_overlayEnabled", false);
    (*view).set_ivar::<i32>("_displayMode", 0);
    (*view).set_ivar::<i32>("_lang", 0);
    (*view).set_ivar::<u32>("_ownDisplayID", 0);

    // Visual defaults
    (*view).set_ivar::<f64>("_radius", DEFAULT_DIAMETER / 2.0);
    (*view).set_ivar::<f64>("_borderWidth", DEFAULT_BORDER_WIDTH);
    (*view).set_ivar::<f64>("_strokeR", DEFAULT_COLOR.0);
    (*view).set_ivar::<f64>("_strokeG", DEFAULT_COLOR.1);
    (*view).set_ivar::<f64>("_strokeB", DEFAULT_COLOR.2);
    (*view).set_ivar::<f64>("_strokeA", DEFAULT_COLOR.3);
    (*view).set_ivar::<f64>("_fillTransparencyPct", DEFAULT_FILL_TRANSPARENCY_PCT);

    // Carbon refs
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkSemi", std::ptr::null_mut());

    // Keep-alive timer ref
    (*view).set_ivar::<id>("_hkKeepAliveTimer", nil);

    // Mouse monitors
    (*view).set_ivar::<id>("_monLeftDown", nil);
    (*view).set_ivar::<id>("_monLeftUp", nil);
    (*view).set_ivar::<id>("_monRightDown", nil);
    (*view).set_ivar::<id>("_monRightUp", nil);
    (*view).set_ivar::<id>("_monMove", nil);

    // Settings UI refs
    (*view).set_ivar::<id>("_settingsWindow", nil);
    (*view).set_ivar::<id>("_labelLang", nil);
    (*view).set_ivar::<id>("_popupLang", nil);

    (*view).set_ivar::<id>("_labelRadius", nil);
    (*view).set_ivar::<id>("_fieldRadius", nil);
    (*view).set_ivar::<id>("_sliderRadius", nil);

    (*view).set_ivar::<id>("_labelBorder", nil);
    (*view).set_ivar::<id>("_fieldBorder", nil);
    (*view).set_ivar::<id>("_sliderBorder", nil);

    (*view).set_ivar::<id>("_labelColor", nil);
    (*view).set_ivar::<id>("_colorWell", nil);

    (*view).set_ivar::<id>("_labelHex", nil);
    (*view).set_ivar::<id>("_fieldHex", nil);

    (*view).set_ivar::<id>("_labelFillT", nil);
    (*view).set_ivar::<id>("_fieldFillT", nil);
    (*view).set_ivar::<id>("_sliderFillT", nil);

    (*view).set_ivar::<id>("_btnClose", nil);

    (*view).set_ivar::<id>("_localKeyMonitor", nil);
    (*view).set_ivar::<f64>("_lastToggleTs", 0.0);
    (*view).set_ivar::<bool>("_menuInstalled", false);

    // Timer de refresco
    (*view).set_ivar::<id>("_updateTimer", nil);

    // Monitor ESC settings
    (*view).set_ivar::<id>("_settingsEscMonitor", nil);

    let _: () = msg_send![window, setContentView: view];
    view
}

// Timer programado por AppKit (más fiable tras abrir paneles/modos de UI)
unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let prev: id = *(*target).get_ivar::<id>("_updateTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*target).set_ivar::<id>("_updateTimer", nil);
    }
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: interval
        target: target
        selector: selector
        userInfo: nil
        repeats: YES
    ];
    (*target).set_ivar::<id>("_updateTimer", timer);
    timer
}

//
// ===================== Hotkeys (Carbon) =====================
//

extern "C" fn hotkey_event_handler(
    _call_ref: EventHandlerCallRef,
    event: EventRef,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    unsafe {
        if GetEventClass(event) == K_EVENT_CLASS_KEYBOARD
            && GetEventKind(event) == K_EVENT_HOTKEY_PRESSED
        {
            let mut hot_id = EventHotKeyID { signature: 0, id: 0 };
            let status = GetEventParameter(
                event,
                K_EVENT_PARAM_DIRECT_OBJECT,
                TYPE_EVENT_HOTKEY_ID,
                std::ptr::null_mut(),
                std::mem::size_of::<EventHotKeyID>() as u32,
                std::ptr::null_mut(),
                &mut hot_id as *mut _ as *mut std::ffi::c_void,
            );
            if status == NO_ERR && hot_id.signature == SIG_MHLT {
                let view = user_data as id;
                match hot_id.id {
                    HKID_TOGGLE => {
                        // Toggle overlay (main thread)
                        let _: () = msg_send![
                            view,
                            performSelectorOnMainThread: sel!(requestToggle)
                            withObject: nil
                            waitUntilDone: NO
                        ];
                    }
                    HKID_SETTINGS_COMMA | HKID_SETTINGS_SEMI => {
                        let block = ConcreteBlock::new(move || {
                            open_settings_window(view);
                        })
                            .copy();
                        let main_queue: id = msg_send![class!(NSOperationQueue), mainQueue];
                        let _: () = msg_send![main_queue, addOperationWithBlock: &*block];
                    }
                    _ => {}
                }
            }
        }
        NO_ERR
    }
}

unsafe fn install_hotkeys(view: id) {
    // Handler Carbon
    let types = [EventTypeSpec {
        event_class: K_EVENT_CLASS_KEYBOARD,
        event_kind: K_EVENT_HOTKEY_PRESSED,
    }];
    let mut handler_ref: EventHandlerRef = std::ptr::null_mut();
    let status = InstallEventHandler(
        GetApplicationEventTarget(),
        hotkey_event_handler,
        types.len() as u32,
        types.as_ptr(),
        view as *mut std::ffi::c_void,
        &mut handler_ref,
    );
    if status != NO_ERR {
        eprintln!("❌ InstallEventHandler failed: {}", status);
        return;
    }
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", handler_ref);

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
                (*view).set_ivar::<*mut std::ffi::c_void>($slot, out_ref);
            }
        }};
    }

    // Ctrl + A (toggle)
    register_hotkey!(KC_A, CONTROL_KEY, HKID_TOGGLE, "_hkToggle");
    // ⌘ + ,  y ⌘ + ; → Settings
    register_hotkey!(KC_COMMA, CMD_KEY, HKID_SETTINGS_COMMA, "_hkComma");
    register_hotkey!(KC_SEMICOLON, CMD_KEY, HKID_SETTINGS_SEMI, "_hkSemi");
}

unsafe fn uninstall_hotkeys(view: id) {
    let hk_toggle: *mut std::ffi::c_void = *(*view).get_ivar("_hkToggle");
    let hk_comma: *mut std::ffi::c_void = *(*view).get_ivar("_hkComma");
    let hk_semi: *mut std::ffi::c_void = *(*view).get_ivar("_hkSemi");
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
    if !hk_handler.is_null() {
        let _ = RemoveEventHandler(hk_handler);
        (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    }
}

/// Re-install hotkeys safely
unsafe fn reinstall_hotkeys(view: id) {
    uninstall_hotkeys(view);
    install_hotkeys(view);
}

unsafe fn install_termination_observer(view: id) {
    // Al terminar app: limpiar Carbon
    let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
    let queue: id = nil; // main thread

    let block = ConcreteBlock::new(move |_note: id| {
        unsafe {
            uninstall_hotkeys(view);
        }
    })
        .copy();

    let name: id = msg_send![
        class!(NSString),
        stringWithUTF8String: b"NSApplicationWillTerminateNotification\0".as_ptr() as *const _
    ];
    let _: id =
        msg_send![center, addObserverForName: name object: nil queue: queue usingBlock: &*block];
}

//
// ===================== Hotkey keep-alive & wake/space observers =====================
//

/// Timer que periódicamente re-instala hotkeys
unsafe fn start_hotkey_keepalive(view: id) {
    // Limpiar anterior si existe
    let prev: id = *(*view).get_ivar::<id>("_hkKeepAliveTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*view).set_ivar::<id>("_hkKeepAliveTimer", nil);
    }

    // 60s; operación barata
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: 60.0f64
        target: view
        selector: sel!(hotkeyKeepAlive)
        userInfo: nil
        repeats: YES
    ];
    (*view).set_ivar::<id>("_hkKeepAliveTimer", timer);
}

/// Observa eventos del sistema que suelen “romper” Carbon y re-instala
unsafe fn install_wakeup_space_observers(view: id) {
    let ws: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let nc: id = msg_send![ws, notificationCenter];

    let add_obs = |name_cstr: &'static [u8]| {
        let name: id =
            msg_send![class!(NSString), stringWithUTF8String: name_cstr.as_ptr() as *const _];
        let block = ConcreteBlock::new(move |_note: id| unsafe {
            reinstall_hotkeys(view);
        })
            .copy();
        let _: id = msg_send![nc, addObserverForName: name object: nil queue: nil usingBlock: &*block];
    };

    // Wake from sleep
    add_obs(b"NSWorkspaceDidWakeNotification\0");
    // Session became active (unlock/login)
    add_obs(b"NSWorkspaceSessionDidBecomeActiveNotification\0");
    // Active Space changed (Mission Control / Spaces)
    add_obs(b"NSWorkspaceActiveSpaceDidChangeNotification\0");
}

//
// ===================== Local Ctrl+A monitor =====================
//

unsafe fn install_local_ctrl_a_monitor(view: id) {
    let existing: id = *(*view).get_ivar::<id>("_localKeyMonitor");
    if existing != nil {
        return;
    }

    const KEY_DOWN_MASK: u64 = 1 << 10;
    const CTRL_FLAG: u64 = 1 << 18;
    const KEYCODE_A: u16 = 0;

    let host = view;
    let block = ConcreteBlock::new(move |event: id| {
        unsafe {
            let keycode: u16 = msg_send![event, keyCode];
            let flags: u64 = msg_send![event, modifierFlags];
            if keycode == KEYCODE_A && (flags & CTRL_FLAG) != 0 {
                let _: () = msg_send![
                    host,
                    performSelectorOnMainThread: sel!(requestToggle)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        }
        event
    })
        .copy();

    let mon: id = msg_send![
        class!(NSEvent),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
        handler: &*block
    ];
    (*view).set_ivar::<id>("_localKeyMonitor", mon);
}

//
// ===================== Global mouse monitors =====================
//

unsafe fn install_mouse_monitors(view: id) {
    // NSEvent masks: leftDown=1<<1, leftUp=1<<2, rightDown=1<<3, rightUp=1<<4, mouseMoved=1<<6
    const LEFT_DOWN_MASK: u64 = 1 << 1;
    const LEFT_UP_MASK: u64 = 1 << 2;
    const RIGHT_DOWN_MASK: u64 = 1 << 3;
    const RIGHT_UP_MASK: u64 = 1 << 4;
    const MOUSE_MOVED_MASK: u64 = 1 << 6;

    let cls = class!(NSEvent);

    // LEFT DOWN -> L mode
    let h1 = ConcreteBlock::new(move |_e: id| unsafe {
        apply_to_all_views(|v| unsafe { *(*v).get_mut_ivar::<i32>("_displayMode") = 1 });
        apply_to_all_views(|v| unsafe { let _: () = msg_send![v, setNeedsDisplay: YES]; });
    })
        .copy();
    let mon_ld: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: LEFT_DOWN_MASK handler: &*h1];
    (*view).set_ivar::<id>("_monLeftDown", mon_ld);

    // LEFT UP -> circle
    let h2 = ConcreteBlock::new(move |_e: id| unsafe {
        apply_to_all_views(|v| unsafe { *(*v).get_mut_ivar::<i32>("_displayMode") = 0 });
        apply_to_all_views(|v| unsafe { let _: () = msg_send![v, setNeedsDisplay: YES]; });
    })
        .copy();
    let mon_lu: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: LEFT_UP_MASK handler: &*h2];
    (*view).set_ivar::<id>("_monLeftUp", mon_lu);

    // RIGHT DOWN -> R mode
    let h3 = ConcreteBlock::new(move |_e: id| unsafe {
        apply_to_all_views(|v| unsafe { *(*v).get_mut_ivar::<i32>("_displayMode") = 2 });
        apply_to_all_views(|v| unsafe { let _: () = msg_send![v, setNeedsDisplay: YES]; });
    })
        .copy();
    let mon_rd: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: RIGHT_DOWN_MASK handler: &*h3];
    (*view).set_ivar::<id>("_monRightDown", mon_rd);

    // RIGHT UP -> circle
    let h4 = ConcreteBlock::new(move |_e: id| unsafe {
        apply_to_all_views(|v| unsafe { *(*v).get_mut_ivar::<i32>("_displayMode") = 0 });
        apply_to_all_views(|v| unsafe { let _: () = msg_send![v, setNeedsDisplay: YES]; });
    })
        .copy();
    let mon_ru: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: RIGHT_UP_MASK handler: &*h4];
    (*view).set_ivar::<id>("_monRightUp", mon_ru);

    // mouseMoved → obliga a refrescar en el hilo principal
    let host = view;
    let hmove = ConcreteBlock::new(move |_e: id| unsafe {
        let _: () = msg_send![
            host,
            performSelectorOnMainThread: sel!(update_cursor_multi)
            withObject: nil
            waitUntilDone: NO
        ];
    })
        .copy();
    let mon_move: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: MOUSE_MOVED_MASK handler: &*hmove];
    (*view).set_ivar::<id>("_monMove", mon_move);
}

//
// ===================== TCC: solicitar Accesibilidad =====================
//

unsafe fn ensure_accessibility_prompt() {
    // NSDictionary { "kAXTrustedCheckOptionPrompt": true }
    let key: id = msg_send![
        class!(NSString),
        stringWithUTF8String: b"kAXTrustedCheckOptionPrompt\0".as_ptr() as *const _
    ];
    let val_true: id = msg_send![class!(NSNumber), numberWithBool: YES];
    let dict: id = msg_send![class!(NSDictionary), dictionaryWithObject: val_true forKey: key];

    let _trusted: bool = AXIsProcessTrustedWithOptions(dict as *const _);
    // Ignoramos el booleano: si no está confiado, esto dispara el prompt del sistema.
}
