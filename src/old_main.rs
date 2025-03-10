extern crate cocoa;
extern crate core_graphics;
extern crate objc;

use cocoa::appkit::{
    NSApp, NSApplication, NSColor, NSPanel, NSWindow, NSWindowStyleMask, 
    NSWindowCollectionBehavior, NSBackingStoreType,
};
use cocoa::base::{id, nil, YES, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use core_graphics::display::CGDisplay;
use objc::runtime::{Class, Object, Sel};
use objc::{declare::ClassDecl, msg_send, sel, sel_impl};

fn main() {
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);

        // Configuración de la ventana flotante
        let displays = CGDisplay::active_displays().unwrap();
        let total_width: f64 = displays.iter().map(|&id| CGDisplay::new(id).bounds().size.width).sum();
        let max_height: f64 = displays.iter().map(|&id| CGDisplay::new(id).bounds().size.height).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        let window: id = msg_send![
            NSPanel::alloc(nil),
            initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(total_width, max_height))
            styleMask: NSWindowStyleMask::NSBorderlessWindowMask
            backing: NSBackingStoreType::NSBackingStoreBuffered
            defer: NO
        ];

        window.setOpaque_(NO);
        window.setBackgroundColor_(NSColor::clearColor(nil));
        window.setIgnoresMouseEvents_(YES);
        window.setLevel_(ns_floating_window_level()); // Nivel flotante alto

        // Comportamiento para mantener visible la ventana en todas las circunstancias
        window.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle
        );

        let view: id = register_custom_view_class(window);
        let _: id = create_timer(view, sel!(update_cursor), 0.016);

        app.run();
    }
}

/// Registrar la clase personalizada de la vista
unsafe fn register_custom_view_class(window: id) -> id {
    let superclass = Class::get("NSView").unwrap();
    let mut decl = ClassDecl::new("CustomView", superclass).unwrap();

    // Método que actualizará el cursor
    extern "C" fn update_cursor(this: &Object, _cmd: Sel) {
        let _cursor_pos = get_mouse_position();
        let _: () = unsafe { msg_send![this, setNeedsDisplay: YES] }; // Fuerza el redibujo
    }

    // Método para dibujar el círculo
    extern "C" fn draw_rect(_this: &Object, _cmd: Sel, _rect: NSRect) {
        // Aquí debes obtener la posición del cursor y dibujar el círculo como lo hacías antes.
    }

    // Añadir los métodos a la clase
    decl.add_method(sel!(update_cursor), update_cursor as extern "C" fn(&Object, Sel));
    decl.add_method(sel!(drawRect:), draw_rect as extern "C" fn(&Object, Sel, NSRect));

    let new_class = decl.register();

    // Crear la vista personalizada
    let view: id = msg_send![new_class, alloc];
    let view: id = msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0))];
    let _: () = msg_send![window, setContentView: view];
    let _: () = msg_send![window, makeKeyAndOrderFront: nil];

    view
}

/// Crear un temporizador de NSTimer con un intervalo específico
unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![timer_class, scheduledTimerWithTimeInterval: interval
                              target: target
                              selector: selector
                              userInfo: nil
                              repeats: YES];
    timer
}

/// Obtiene la posición actual del ratón en la pantalla
fn get_mouse_position() -> (f64, f64) {
    (0.0, 0.0) // Aquí puedes usar tu código para obtener la posición real del ratón
}

/// Definir la función de nivel flotante
fn ns_floating_window_level() -> i64 {
    1000 // Nivel flotante para que la ventana siempre esté al frente
}
