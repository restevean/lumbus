extern crate cocoa;
extern crate core_graphics;
extern crate objc;

use cocoa::appkit::{
    NSApp, NSApplication, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use cocoa::base::{id, nil, YES, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::runtime::{Class, Object, Sel};
use objc::{declare::ClassDecl, msg_send, sel, sel_impl};

// Dimensiones del círculo
const CIRCLE_DIAMETER: f64 = 38.5;
const BORDER_WIDTH: f64 = 3.0;

// Almacenamos la posición del cursor
static mut CURSOR_POS: (f64, f64) = (0.0, 0.0);

fn main() {
    unsafe {
        // Inicializamos la aplicación
        let app = NSApp();
        app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);

        // Obtenemos las pantallas activas
        let displays = CGDisplay::active_displays().unwrap();
        let total_width: f64 = displays.iter().map(|&id| CGDisplay::new(id).bounds().size.width).sum();
        let max_height: f64 = displays.iter().map(|&id| CGDisplay::new(id).bounds().size.height).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        // Creamos la ventana sin bordes y del tamaño de todas las pantallas
        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(total_width, max_height)),
            NSWindowStyleMask::NSBorderlessWindowMask,
            cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );

        // Configuración de la ventana para que se muestre en todas las pantallas
        window.setOpaque_(NO);
        window.setBackgroundColor_(NSColor::clearColor(nil));
        window.setIgnoresMouseEvents_(YES);

        // Forzar el nivel de la ventana por encima de los menús contextuales y el dock
        window.setLevel_((nspop_up_menu_window_level() + 1).into());

        // Configuramos la ventana para que esté en todos los espacios, pantallas y en pantalla completa
        window.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
        );

        let view: id = register_custom_view_class(window);

        // Usamos NSTimer para actualizar la posición del cursor cada 16 ms
        let _: id = create_timer(view, sel!(update_cursor), 0.016);

        app.run(); // Ejecuta la aplicación
    }
}

/// Función para obtener el nivel de la ventana para menús contextuales
fn nspop_up_menu_window_level() -> i64 {
    201 // Ajustado para estar por encima de los menús contextuales
}

/// Registrar la clase personalizada de la vista
unsafe fn register_custom_view_class(window: id) -> id {
    let superclass = Class::get("NSView").unwrap();
    let mut decl = ClassDecl::new("CustomView", superclass).unwrap();

    // Método que actualizará el cursor
    extern "C" fn update_cursor(this: &Object, _cmd: Sel) {
        unsafe {
            let cursor_pos = get_mouse_position();
            CURSOR_POS = cursor_pos;
            let _: () = msg_send![this, setNeedsDisplay: YES]; // Fuerza el redibujo
        }
    }

    // Método para dibujar el círculo
    extern "C" fn draw_rect(_this: &Object, _cmd: Sel, _rect: NSRect) {
        unsafe {
            let (x, y) = CURSOR_POS;

            // Invertimos la coordenada Y para que el círculo siga correctamente el puntero
            let screen_height = CGDisplay::main().bounds().size.height;
            let adjusted_y = screen_height - y;  // Corregimos el movimiento en Y

            let radius = CIRCLE_DIAMETER / 2.0;
            let rect = NSRect::new(
                NSPoint::new(x - radius, adjusted_y - radius), // Usamos las coordenadas corregidas
                NSSize::new(CIRCLE_DIAMETER, CIRCLE_DIAMETER),
            );

            // Obtener la ruta de dibujo y el color
            let circle: id = msg_send![Class::get("NSBezierPath").unwrap(), bezierPathWithOvalInRect: rect];
            let _: () = msg_send![circle, setLineWidth: BORDER_WIDTH];
            let color: id = msg_send![Class::get("NSColor").unwrap(), whiteColor]; // Borde blanco
            let _: () = msg_send![color, set];
            let _: () = msg_send![circle, stroke];
        }
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
    let event_source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).unwrap();
    let event = CGEvent::new(event_source).unwrap();
    let location = event.location(); // Esto obtiene la posición global del cursor
    (location.x, location.y)
}
