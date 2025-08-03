#![allow(unexpected_cfgs)] // Silencia warnings de cfg dentro de macros de objc/cocoa

use block::ConcreteBlock;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize};
use objc::runtime::{Class, Object, Sel};
use objc::{class, declare::ClassDecl, msg_send, sel, sel_impl};
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
    // snake_case para evitar warnings; layout idéntico
    event_class: u32,
    event_kind: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EventHotKeyID {
    signature: u32, // FourCC
    id: u32,        // identificador del hotkey
}

// Constantes Carbon
const NO_ERR: i32 = 0;
const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7962; // 'keyb'
const K_EVENT_HOTKEY_PRESSED: u32 = 6;
const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2D2D_2D2D; // '----'
const TYPE_EVENT_HOTKEY_ID: u32 = 0x686B_6964; // 'hkid'

// Modificadores
const CMD_KEY: u32 = 1 << 8;
const CONTROL_KEY: u32 = 1 << 12;

// Keycodes comunes (ANSI)
const KC_A: u32 = 0;
const KC_SEMICOLON: u32 = 41;
const KC_COMMA: u32 = 43;

// Firma hotkeys: 'mhlt'
const SIG_MHLT: u32 = 0x6D68_6C74;
// IDs
const HKID_TOGGLE: u32 = 1;
const HKID_SETTINGS_COMMA: u32 = 2;
const HKID_SETTINGS_SEMI: u32 = 3;

//
// ===================== Apariencia por defecto =====================
//

const DEFAULT_DIAMETER: f64 = 38.5;
const DEFAULT_BORDER_WIDTH: f64 = 3.0;
const DEFAULT_COLOR: (f64, f64, f64, f64) = (1.0, 1.0, 1.0, 1.0); // blanco

//
// ===================== Claves de preferencias (NSUserDefaults) =====================
//

const PREF_RADIUS: &str = "radius";
const PREF_BORDER: &str = "borderWidth";
const PREF_R: &str = "strokeR";
const PREF_G: &str = "strokeG";
const PREF_B: &str = "strokeB";
const PREF_A: &str = "strokeA";

//
// ===================== App =====================
//

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);

        let (min_x, min_y, union_width, union_height) = union_de_pantallas_cocoa();

        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(
                NSPoint::new(min_x, min_y),
                NSSize::new(union_width, union_height),
            ),
            NSWindowStyleMask::NSBorderlessWindowMask,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );
        window.setOpaque_(NO);
        window.setBackgroundColor_(NSColor::clearColor(nil));
        window.setIgnoresMouseEvents_(YES);
        window.setLevel_((nspop_up_menu_window_level() + 1).into());
        window.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
        );

        // Vista personalizada
        let view: id = register_custom_view_class_and_create_view(window, union_width, union_height);

        // Cargar preferencias (NSUserDefaults) a la vista
        load_preferences_into_view(view);

        // Timer ~60 FPS
        let _: id = create_timer(view, sel!(update_cursor), 0.016);

        // Hotkeys globales (sin beep)
        install_hotkeys(view);

        // Observar terminación para desregistrar hotkeys/handler
        install_termination_observer(view);

        let _: () = msg_send![window, orderFrontRegardless];

        app.run();
    }
}

/// Nivel por encima de menús contextuales (aproximado)
fn nspop_up_menu_window_level() -> i64 {
    201
}

/// Unión de pantallas (Cocoa)
unsafe fn union_de_pantallas_cocoa() -> (f64, f64, f64, f64) {
    let screens: id = msg_send![class!(NSScreen), screens];
    let count: usize = msg_send![screens, count];

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for i in 0..count {
        let screen: id = msg_send![screens, objectAtIndex: i];
        let frame: NSRect = msg_send![screen, frame];
        min_x = min_x.min(frame.origin.x);
        min_y = min_y.min(frame.origin.y);
        max_x = max_x.max(frame.origin.x + frame.size.width);
        max_y = max_y.max(frame.origin.y + frame.size.height);
    }

    let union_width = (max_x - min_x).max(1.0);
    let union_height = (max_y - min_y).max(1.0);
    (min_x, min_y, union_width, union_height)
}

/// Posición del ratón en coordenadas Cocoa
fn get_mouse_position_cocoa() -> (f64, f64) {
    unsafe {
        let cls = class!(NSEvent);
        let p: NSPoint = msg_send![cls, mouseLocation];
        (p.x, p.y)
    }
}

/// Helper: NSString* desde &str
unsafe fn nsstring(s: &str) -> id {
    let cstr = CString::new(s).unwrap();
    let ns: id = msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
    ns
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

/// Cargar preferencias al arrancar y volcarlas a la vista
unsafe fn load_preferences_into_view(view: id) {
    let radius = prefs_get_double(PREF_RADIUS, DEFAULT_DIAMETER / 2.0);
    let border = prefs_get_double(PREF_BORDER, DEFAULT_BORDER_WIDTH);
    let r = prefs_get_double(PREF_R, DEFAULT_COLOR.0);
    let g = prefs_get_double(PREF_G, DEFAULT_COLOR.1);
    let b = prefs_get_double(PREF_B, DEFAULT_COLOR.2);
    let a = prefs_get_double(PREF_A, DEFAULT_COLOR.3);

    (*view).set_ivar::<f64>("_radius", radius);
    (*view).set_ivar::<f64>("_borderWidth", border);
    (*view).set_ivar::<f64>("_strokeR", r);
    (*view).set_ivar::<f64>("_strokeG", g);
    (*view).set_ivar::<f64>("_strokeB", b);
    (*view).set_ivar::<f64>("_strokeA", a);
}

//
// ===================== Utilidades Color / Texto =====================
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

//
// ===================== Configuración (ventana) =====================
//

fn open_settings_window(view: id) {
    unsafe {
        let existing: id = *(*view).get_ivar::<id>("_settingsWindow");
        if existing != nil {
            let _: () = msg_send![existing, makeKeyAndOrderFront: nil];
            return;
        }

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];

        let style = NSWindowStyleMask::NSTitledWindowMask
            | NSWindowStyleMask::NSClosableWindowMask
            | NSWindowStyleMask::NSMiniaturizableWindowMask;
        let w = 460.0;
        let h = 240.0;
        let settings = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)),
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );
        let _: () = msg_send![settings, setTitle: nsstring("Configuración")];
        let _: () = msg_send![settings, center];

        let content: id = msg_send![settings, contentView];

        let radius: f64 = *(*view).get_ivar::<f64>("_radius");
        let border: f64 = *(*view).get_ivar::<f64>("_borderWidth");
        let r: f64 = *(*view).get_ivar::<f64>("_strokeR");
        let g: f64 = *(*view).get_ivar::<f64>("_strokeG");
        let b: f64 = *(*view).get_ivar::<f64>("_strokeB");
        let a: f64 = *(*view).get_ivar::<f64>("_strokeA");

        // Labels
        let label_radius: id = msg_send![class!(NSTextField), alloc];
        let label_radius: id = msg_send![
            label_radius,
            initWithFrame: NSRect::new(NSPoint::new(20.0, h - 50.0), NSSize::new(90.0, 20.0))
        ];
        let _: () = msg_send![label_radius, setBezeled: NO];
        let _: () = msg_send![label_radius, setDrawsBackground: NO];
        let _: () = msg_send![label_radius, setEditable: NO];
        let _: () = msg_send![label_radius, setSelectable: NO];
        let _: () = msg_send![label_radius, setStringValue: nsstring("Radio (px)")];

        let label_border: id = msg_send![class!(NSTextField), alloc];
        let label_border: id = msg_send![
            label_border,
            initWithFrame: NSRect::new(NSPoint::new(20.0, h - 100.0), NSSize::new(90.0, 20.0))
        ];
        let _: () = msg_send![label_border, setBezeled: NO];
        let _: () = msg_send![label_border, setDrawsBackground: NO];
        let _: () = msg_send![label_border, setEditable: NO];
        let _: () = msg_send![label_border, setSelectable: NO];
        let _: () = msg_send![label_border, setStringValue: nsstring("Grosor (px)")];

        let label_color: id = msg_send![class!(NSTextField), alloc];
        let label_color: id = msg_send![
            label_color,
            initWithFrame: NSRect::new(NSPoint::new(20.0, h - 150.0), NSSize::new(90.0, 20.0))
        ];
        let _: () = msg_send![label_color, setBezeled: NO];
        let _: () = msg_send![label_color, setDrawsBackground: NO];
        let _: () = msg_send![label_color, setEditable: NO];
        let _: () = msg_send![label_color, setSelectable: NO];
        let _: () = msg_send![label_color, setStringValue: nsstring("Color")];

        // ===== Campos numéricos y sliders sincronizados =====
        // Radio: campo + slider
        let field_radius: id = msg_send![class!(NSTextField), alloc];
        let field_radius: id = msg_send![
            field_radius,
            initWithFrame: NSRect::new(NSPoint::new(120.0, h - 54.0), NSSize::new(60.0, 24.0))
        ];
        let _: () = msg_send![field_radius, setStringValue: nsstring(&format!("{:.0}", radius))];
        let _: () = msg_send![field_radius, setBezeled: YES];
        let _: () = msg_send![field_radius, setEditable: YES];
        let _: () = msg_send![field_radius, setTarget: view];
        let _: () = msg_send![field_radius, setAction: sel!(setRadiusFromField:)];

        let slider_radius: id = msg_send![class!(NSSlider), alloc];
        let slider_radius: id = msg_send![
            slider_radius,
            initWithFrame: NSRect::new(NSPoint::new(190.0, h - 55.0), NSSize::new(240.0, 24.0))
        ];
        let _: () = msg_send![slider_radius, setMinValue: 5.0f64];
        let _: () = msg_send![slider_radius, setMaxValue: 200.0f64];
        let _: () = msg_send![slider_radius, setDoubleValue: radius];
        let _: () = msg_send![slider_radius, setTarget: view];
        let _: () = msg_send![slider_radius, setAction: sel!(setRadius:)];

        // Grosor: campo + slider
        let field_border: id = msg_send![class!(NSTextField), alloc];
        let field_border: id = msg_send![
            field_border,
            initWithFrame: NSRect::new(NSPoint::new(120.0, h - 104.0), NSSize::new(60.0, 24.0))
        ];
        let _: () = msg_send![field_border, setStringValue: nsstring(&format!("{:.0}", border))];
        let _: () = msg_send![field_border, setBezeled: YES];
        let _: () = msg_send![field_border, setEditable: YES];
        let _: () = msg_send![field_border, setTarget: view];
        let _: () = msg_send![field_border, setAction: sel!(setBorderFromField:)];

        let slider_border: id = msg_send![class!(NSSlider), alloc];
        let slider_border: id = msg_send![
            slider_border,
            initWithFrame: NSRect::new(NSPoint::new(190.0, h - 105.0), NSSize::new(240.0, 24.0))
        ];
        let _: () = msg_send![slider_border, setMinValue: 1.0f64];
        let _: () = msg_send![slider_border, setMaxValue: 20.0f64];
        let _: () = msg_send![slider_border, setDoubleValue: border];
        let _: () = msg_send![slider_border, setTarget: view];
        let _: () = msg_send![slider_border, setAction: sel!(setBorderWidth:)];

        // ColorWell
        let color_well: id = msg_send![class!(NSColorWell), alloc];
        let color_well: id = msg_send![
            color_well,
            initWithFrame: NSRect::new(NSPoint::new(120.0, h - 155.0), NSSize::new(50.0, 25.0))
        ];
        let ns_color = Class::get("NSColor").unwrap();
        let current_color: id =
            msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
        let _: () = msg_send![color_well, setColor: current_color];
        let _: () = msg_send![color_well, setTarget: view];
        let _: () = msg_send![color_well, setAction: sel!(colorChanged:)];

        // Hex label + field
        let label_hex: id = msg_send![class!(NSTextField), alloc];
        let label_hex: id = msg_send![
            label_hex,
            initWithFrame: NSRect::new(NSPoint::new(190.0, h - 150.0), NSSize::new(40.0, 20.0))
        ];
        let _: () = msg_send![label_hex, setBezeled: NO];
        let _: () = msg_send![label_hex, setDrawsBackground: NO];
        let _: () = msg_send![label_hex, setEditable: NO];
        let _: () = msg_send![label_hex, setSelectable: NO];
        let _: () = msg_send![label_hex, setStringValue: nsstring("Hex")];

        let hex_str = color_to_hex(r, g, b, a);
        let field_hex: id = msg_send![class!(NSTextField), alloc];
        let field_hex: id = msg_send![
            field_hex,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 154.0), NSSize::new(200.0, 24.0))
        ];
        let _: () = msg_send![field_hex, setStringValue: nsstring(&hex_str)];
        let _: () = msg_send![field_hex, setBezeled: YES];
        let _: () = msg_send![field_hex, setEditable: YES];
        let _: () = msg_send![field_hex, setTarget: view];
        let _: () = msg_send![field_hex, setAction: sel!(hexChanged:)];

        // Botón cerrar
        let btn_close: id = msg_send![class!(NSButton), alloc];
        let btn_close: id = msg_send![
            btn_close,
            initWithFrame: NSRect::new(NSPoint::new(w - 100.0, 15.0), NSSize::new(80.0, 28.0))
        ];
        let _: () = msg_send![btn_close, setTitle: nsstring("Cerrar")];
        let _: () = msg_send![btn_close, setTarget: view];
        let _: () = msg_send![btn_close, setAction: sel!(closeSettings:)];

        // Añadir subviews
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

        let _: () = msg_send![content, addSubview: btn_close];

        // Guardar punteros de controles para sincronización desde acciones
        (*view).set_ivar::<id>("_settingsWindow", settings);
        (*view).set_ivar::<id>("_fieldRadius", field_radius);
        (*view).set_ivar::<id>("_sliderRadius", slider_radius);
        (*view).set_ivar::<id>("_fieldBorder", field_border);
        (*view).set_ivar::<id>("_sliderBorder", slider_border);
        (*view).set_ivar::<id>("_colorWell", color_well);
        (*view).set_ivar::<id>("_fieldHex", field_hex);

        let _: () = msg_send![settings, makeKeyAndOrderFront: nil];
    }
}

fn close_settings_window(view: id) {
    unsafe {
        let settings: id = *(*view).get_ivar::<id>("_settingsWindow");
        if settings != nil {
            let _: () = msg_send![settings, orderOut: nil];
            (*view).set_ivar::<id>("_settingsWindow", nil);
        }
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);

        let overlay_win: id = msg_send![view, window];
        let _: () = msg_send![overlay_win, setLevel: (nspop_up_menu_window_level() + 1)];
        let _: () = msg_send![overlay_win, orderFrontRegardless];

        let _: () = msg_send![
            view,
            performSelectorOnMainThread: sel!(update_cursor)
            withObject: nil
            waitUntilDone: NO
        ];
    }
}

//
// ===================== Vista personalizada =====================
//

unsafe fn register_custom_view_class_and_create_view(window: id, width: f64, height: f64) -> id {
    let class_name = "CustomView";
    let custom_view_class = if let Some(cls) = Class::get(class_name) {
        cls
    } else {
        let superclass = Class::get("NSView").unwrap();
        let mut decl = ClassDecl::new(class_name, superclass).unwrap();

        // Estado
        decl.add_ivar::<f64>("_cursorXScreen");
        decl.add_ivar::<f64>("_cursorYScreen");
        decl.add_ivar::<bool>("_visible");
        // Parámetros
        decl.add_ivar::<f64>("_radius");
        decl.add_ivar::<f64>("_borderWidth");
        decl.add_ivar::<f64>("_strokeR");
        decl.add_ivar::<f64>("_strokeG");
        decl.add_ivar::<f64>("_strokeB");
        decl.add_ivar::<f64>("_strokeA");
        // Hotkeys/handler (refs Carbon)
        decl.add_ivar::<*mut std::ffi::c_void>("_hkHandler");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkToggle");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkComma");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkSemi");
        // Ventana settings + controles
        decl.add_ivar::<id>("_settingsWindow");
        decl.add_ivar::<id>("_fieldRadius");
        decl.add_ivar::<id>("_sliderRadius");
        decl.add_ivar::<id>("_fieldBorder");
        decl.add_ivar::<id>("_sliderBorder");
        decl.add_ivar::<id>("_colorWell");
        decl.add_ivar::<id>("_fieldHex");

        extern "C" fn update_cursor(this: &mut Object, _cmd: Sel) {
            unsafe {
                let (x, y) = get_mouse_position_cocoa();
                *this.get_mut_ivar::<f64>("_cursorXScreen") = x;
                *this.get_mut_ivar::<f64>("_cursorYScreen") = y;
                let _: () = msg_send![this, setNeedsDisplay: YES];
            }
        }

        extern "C" fn toggle_visibility(this: &mut Object, _cmd: Sel) {
            unsafe {
                let vis = *this.get_ivar::<bool>("_visible");
                *this.get_mut_ivar::<bool>("_visible") = !vis;
                if !vis {
                    let _: () = msg_send![
                        this,
                        performSelectorOnMainThread: sel!(update_cursor)
                        withObject: nil
                        waitUntilDone: NO
                    ];
                } else {
                    let _: () = msg_send![this, setNeedsDisplay: YES];
                }
            }
        }

        // ===== Acciones de configuración (con guardado y sincronización UI) =====

        extern "C" fn set_radius(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = clamp(v, 5.0, 200.0);
                *this.get_mut_ivar::<f64>("_radius") = v;
                prefs_set_double(PREF_RADIUS, v);
                // sync campo
                let field: id = *this.get_ivar("_fieldRadius");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }
                let _: () = msg_send![this, setNeedsDisplay: YES];
            }
        }

        extern "C" fn set_radius_from_field(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let s: id = msg_send![sender, stringValue];
                let cstr_ptr: *const std::os::raw::c_char = msg_send![s, UTF8String];
                if !cstr_ptr.is_null() {
                    let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
                    if let Ok(mut v) = txt.trim().parse::<f64>() {
                        v = clamp(v, 5.0, 200.0);
                        *this.get_mut_ivar::<f64>("_radius") = v;
                        prefs_set_double(PREF_RADIUS, v);
                        // sync slider
                        let slider: id = *this.get_ivar("_sliderRadius");
                        if slider != nil {
                            let _: () = msg_send![slider, setDoubleValue: v];
                        }
                        // normaliza texto
                        let _: () = msg_send![sender, setStringValue: nsstring(&format!("{:.0}", v))];
                        let _: () = msg_send![this, setNeedsDisplay: YES];
                    }
                }
            }
        }

        extern "C" fn set_border_width(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = clamp(v, 1.0, 20.0);
                *this.get_mut_ivar::<f64>("_borderWidth") = v;
                prefs_set_double(PREF_BORDER, v);
                // sync campo
                let field: id = *this.get_ivar("_fieldBorder");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }
                let _: () = msg_send![this, setNeedsDisplay: YES];
            }
        }

        extern "C" fn set_border_from_field(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let s: id = msg_send![sender, stringValue];
                let cstr_ptr: *const std::os::raw::c_char = msg_send![s, UTF8String];
                if !cstr_ptr.is_null() {
                    let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
                    if let Ok(mut v) = txt.trim().parse::<f64>() {
                        v = clamp(v, 1.0, 20.0);
                        *this.get_mut_ivar::<f64>("_borderWidth") = v;
                        prefs_set_double(PREF_BORDER, v);
                        // sync slider
                        let slider: id = *this.get_ivar("_sliderBorder");
                        if slider != nil {
                            let _: () = msg_send![slider, setDoubleValue: v];
                        }
                        // normaliza texto
                        let _: () = msg_send![sender, setStringValue: nsstring(&format!("{:.0}", v))];
                        let _: () = msg_send![this, setNeedsDisplay: YES];
                    }
                }
            }
        }

        extern "C" fn color_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let color: id = msg_send![sender, color];
                let r: f64 = msg_send![color, redComponent];
                let g: f64 = msg_send![color, greenComponent];
                let b: f64 = msg_send![color, blueComponent];
                let a: f64 = msg_send![color, alphaComponent];
                *this.get_mut_ivar::<f64>("_strokeR") = r;
                *this.get_mut_ivar::<f64>("_strokeG") = g;
                *this.get_mut_ivar::<f64>("_strokeB") = b;
                *this.get_mut_ivar::<f64>("_strokeA") = a;
                // Guardar
                prefs_set_double(PREF_R, r);
                prefs_set_double(PREF_G, g);
                prefs_set_double(PREF_B, b);
                prefs_set_double(PREF_A, a);
                // sync campo hex
                let hex_field: id = *this.get_ivar("_fieldHex");
                if hex_field != nil {
                    let s = color_to_hex(r, g, b, a);
                    let _: () = msg_send![hex_field, setStringValue: nsstring(&s)];
                }
                let _: () = msg_send![this, setNeedsDisplay: YES];
            }
        }

        extern "C" fn hex_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let s: id = msg_send![sender, stringValue];
                let cstr_ptr: *const std::os::raw::c_char = msg_send![s, UTF8String];
                if !cstr_ptr.is_null() {
                    let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
                    if let Some((r, g, b, a)) = parse_hex_color(&txt) {
                        *this.get_mut_ivar::<f64>("_strokeR") = r;
                        *this.get_mut_ivar::<f64>("_strokeG") = g;
                        *this.get_mut_ivar::<f64>("_strokeB") = b;
                        *this.get_mut_ivar::<f64>("_strokeA") = a;
                        // Guardar
                        prefs_set_double(PREF_R, r);
                        prefs_set_double(PREF_G, g);
                        prefs_set_double(PREF_B, b);
                        prefs_set_double(PREF_A, a);
                        // sync colorWell
                        let ns_color = Class::get("NSColor").unwrap();
                        let col: id =
                            msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                        let well: id = *this.get_ivar("_colorWell");
                        if well != nil {
                            let _: () = msg_send![well, setColor: col];
                        }
                        // normaliza texto
                        let norm = color_to_hex(r, g, b, a);
                        let _: () = msg_send![sender, setStringValue: nsstring(&norm)];
                        let _: () = msg_send![this, setNeedsDisplay: YES];
                    } else {
                        // entrada inválida: reponer valor actual
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

        extern "C" fn draw_rect(this: &Object, _cmd: Sel, _rect: NSRect) {
            unsafe {
                let sx = *this.get_ivar::<f64>("_cursorXScreen");
                let sy = *this.get_ivar::<f64>("_cursorYScreen");
                let visible = *this.get_ivar::<bool>("_visible");

                if visible {
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

                    let rect = NSRect::new(
                        NSPoint::new(view_pt.x - radius, view_pt.y - radius),
                        NSSize::new(radius * 2.0, radius * 2.0),
                    );

                    let ns_bezier = Class::get("NSBezierPath").unwrap();
                    let circle: id = msg_send![ns_bezier, bezierPathWithOvalInRect: rect];

                    let ns_color = Class::get("NSColor").unwrap();
                    let stroke: id =
                        msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                    let _: () = msg_send![stroke, set];
                    let _: () = msg_send![circle, setLineWidth: border_width];
                    let _: () = msg_send![circle, stroke];
                }
            }
        }

        // Registro de métodos
        decl.add_method(sel!(update_cursor), update_cursor as extern "C" fn(&mut Object, Sel));
        decl.add_method(
            sel!(toggleVisibility),
            toggle_visibility as extern "C" fn(&mut Object, Sel),
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
        decl.add_method(sel!(colorChanged:), color_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(hexChanged:), hex_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(closeSettings:), close_settings as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(drawRect:), draw_rect as extern "C" fn(&Object, Sel, NSRect));

        decl.register()
    };

    let view: id = msg_send![custom_view_class, alloc];
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let view: id = msg_send![view, initWithFrame: frame];

    (*view).set_ivar::<f64>("_cursorXScreen", 0.0);
    (*view).set_ivar::<f64>("_cursorYScreen", 0.0);
    (*view).set_ivar::<bool>("_visible", false);

    // Parámetros iniciales (serán sobrescritos por load_preferences_into_view)
    (*view).set_ivar::<f64>("_radius", DEFAULT_DIAMETER / 2.0);
    (*view).set_ivar::<f64>("_borderWidth", DEFAULT_BORDER_WIDTH);
    (*view).set_ivar::<f64>("_strokeR", DEFAULT_COLOR.0);
    (*view).set_ivar::<f64>("_strokeG", DEFAULT_COLOR.1);
    (*view).set_ivar::<f64>("_strokeB", DEFAULT_COLOR.2);
    (*view).set_ivar::<f64>("_strokeA", DEFAULT_COLOR.3);

    // Hotkeys/handler (refs Carbon)
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkSemi", std::ptr::null_mut());

    (*view).set_ivar::<id>("_settingsWindow", nil);
    (*view).set_ivar::<id>("_fieldRadius", nil);
    (*view).set_ivar::<id>("_sliderRadius", nil);
    (*view).set_ivar::<id>("_fieldBorder", nil);
    (*view).set_ivar::<id>("_sliderBorder", nil);
    (*view).set_ivar::<id>("_colorWell", nil);
    (*view).set_ivar::<id>("_fieldHex", nil);

    let _: () = msg_send![window, setContentView: view];
    view
}

unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: interval
        target: target
        selector: selector
        userInfo: nil
        repeats: YES
    ];
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
                        let _: () = msg_send![
                            view,
                            performSelectorOnMainThread: sel!(toggleVisibility)
                            withObject: nil
                            waitUntilDone: NO
                        ];
                    }
                    HKID_SETTINGS_COMMA | HKID_SETTINGS_SEMI => {
                        let block =
                            ConcreteBlock::new(move || {
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
    // Instalar handler de eventos
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
        view as *mut std::ffi::c_void, // user_data -> view
        &mut handler_ref,
    );
    if status != NO_ERR {
        eprintln!("❌ InstallEventHandler falló: {}", status);
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
                    "❌ RegisterEventHotKey fallo (code={}, mods={}, id={}): {}",
                    $keycode, $mods, $idconst, st
                );
            } else {
                (*view).set_ivar::<*mut std::ffi::c_void>($slot, out_ref);
            }
        }};
    }

    // Ctrl + A (toggle)
    register_hotkey!(KC_A, CONTROL_KEY, HKID_TOGGLE, "_hkToggle");
    // ⌘ + ,  y ⌘ + ; (compat ISO/ANSI) → Configuración
    register_hotkey!(KC_COMMA, CMD_KEY, HKID_SETTINGS_COMMA, "_hkComma");
    register_hotkey!(KC_SEMICOLON, CMD_KEY, HKID_SETTINGS_SEMI, "_hkSemi");
}

/// Desregistra hotkeys y handler Carbon (se llama en terminación)
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

/// Observador de terminación de la app: limpia hotkeys/handler
unsafe fn install_termination_observer(view: id) {
    let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
    let queue: id = nil; // main thread

    let block = ConcreteBlock::new(move |_note: id| {
        unsafe { uninstall_hotkeys(view); }
    })
        .copy();

    let name: id = msg_send![
        class!(NSString),
        stringWithUTF8String: b"NSApplicationWillTerminateNotification\0".as_ptr() as *const _
    ];
    let _: id =
        msg_send![center, addObserverForName: name object: nil queue: queue usingBlock: &*block];
}
