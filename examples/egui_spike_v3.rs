//! Spike V3: Full macOS integration
//!
//! Tests:
//! 1. Fullscreen transparent overlay
//! 2. Global cursor position tracking
//! 3. Global hotkey (Ctrl+A toggle)
//! 4. Window level above Dock
//! 5. Multi-Space support (NSWindowCollectionBehavior)
//!
//! Run with: cargo run --example egui_spike_v3

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ============================================================================
// macOS-specific FFI
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    /// Window levels (from NSWindow.h)
    pub const NS_POP_UP_MENU_WINDOW_LEVEL: i64 = 101;

    /// Our overlay level - above popup menus but below screen saver
    pub const OVERLAY_WINDOW_LEVEL: i64 = NS_POP_UP_MENU_WINDOW_LEVEL + 1;

    /// NSWindowCollectionBehavior flags
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY: u64 = 1 << 4;

    /// Configure NSWindow for overlay behavior from NSView pointer
    ///
    /// # Safety
    /// Must be called from main thread with valid NSView pointer
    pub unsafe fn configure_overlay_window_from_view(ns_view_ptr: *mut std::ffi::c_void) {
        if ns_view_ptr.is_null() {
            eprintln!("Warning: NSView pointer is null");
            return;
        }

        let view = ns_view_ptr as *mut AnyObject;

        // Get NSWindow from NSView
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() {
            eprintln!("Warning: Could not get NSWindow from NSView");
            return;
        }

        // Set window level above Dock
        let _: () = msg_send![window, setLevel: OVERLAY_WINDOW_LEVEL];

        // Set collection behavior for multi-Space support
        let behavior = NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES
            | NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY
            | NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY;
        let _: () = msg_send![window, setCollectionBehavior: behavior];

        // Make window ignore mouse events (reinforces click-through)
        let _: () = msg_send![window, setIgnoresMouseEvents: true];

        println!(
            "✅ NSWindow configured: level={}, behavior={}",
            OVERLAY_WINDOW_LEVEL, behavior
        );
    }

    /// Get global mouse position using CoreGraphics
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreate(source: *const std::ffi::c_void) -> *const std::ffi::c_void;
        fn CGEventGetLocation(event: *const std::ffi::c_void) -> CGPoint;
        fn CFRelease(cf: *const std::ffi::c_void);
        fn CGMainDisplayID() -> u32;
        fn CGDisplayPixelsWide(display: u32) -> usize;
        fn CGDisplayPixelsHigh(display: u32) -> usize;
    }

    pub fn get_global_mouse_position() -> (f64, f64) {
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            if event.is_null() {
                return (0.0, 0.0);
            }
            let point = CGEventGetLocation(event);
            CFRelease(event);
            (point.x, point.y)
        }
    }

    pub fn get_screen_size() -> (f32, f32) {
        unsafe {
            let display = CGMainDisplayID();
            let width = CGDisplayPixelsWide(display) as f32;
            let height = CGDisplayPixelsHigh(display) as f32;
            (width, height)
        }
    }
}

#[cfg(target_os = "macos")]
use macos::{configure_overlay_window_from_view, get_global_mouse_position, get_screen_size};

// ============================================================================
// Main application
// ============================================================================

fn main() -> eframe::Result<()> {
    // Set up global hotkey manager
    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
    hotkey_manager
        .register(hotkey)
        .expect("Failed to register Ctrl+A hotkey");

    // Shared state
    let visible = Arc::new(AtomicBool::new(true));
    let visible_clone = visible.clone();
    let window_configured = Arc::new(AtomicBool::new(false));

    // Hotkey listener thread
    std::thread::spawn(move || loop {
        if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
            if event.id == hotkey.id() && event.state == HotKeyState::Pressed {
                let current = visible_clone.load(Ordering::SeqCst);
                visible_clone.store(!current, Ordering::SeqCst);
                println!(
                    "Ctrl+A → Visibility: {}",
                    if !current { "ON" } else { "OFF" }
                );
            }
        }
    });

    let (screen_width, screen_height) = get_screen_size();
    println!("Screen size: {}x{}", screen_width, screen_height);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumbus Spike V3")
            .with_position([0.0, 0.0])
            .with_inner_size([screen_width, screen_height])
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "Lumbus egui Spike V3",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            Ok(Box::new(LumbusSpikeV3 {
                visible: visible.clone(),
                window_configured: window_configured.clone(),
                circle_radius: 40.0,
                circle_color: egui::Color32::from_rgb(255, 204, 0),
                border_width: 3.0,
            }))
        }),
    )
}

struct LumbusSpikeV3 {
    visible: Arc<AtomicBool>,
    window_configured: Arc<AtomicBool>,
    circle_radius: f32,
    circle_color: egui::Color32,
    border_width: f32,
}

impl eframe::App for LumbusSpikeV3 {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Configure NSWindow on first frame (once we have access to it)
        #[cfg(target_os = "macos")]
        if !self.window_configured.load(Ordering::SeqCst) {
            if let Ok(handle) = frame.window_handle() {
                if let RawWindowHandle::AppKit(appkit_handle) = handle.as_raw() {
                    unsafe {
                        configure_overlay_window_from_view(appkit_handle.ns_view.as_ptr() as *mut _);
                    }
                    self.window_configured.store(true, Ordering::SeqCst);
                }
            }
        }

        // Check visibility
        if !self.visible.load(Ordering::SeqCst) {
            ctx.request_repaint();
            return;
        }

        // Get global mouse position
        let (mouse_x, mouse_y) = get_global_mouse_position();
        let mouse_pos = egui::pos2(mouse_x as f32, mouse_y as f32);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let painter = ui.painter();

                // Draw circle at cursor position
                painter.circle_stroke(
                    mouse_pos,
                    self.circle_radius,
                    egui::Stroke::new(self.border_width, self.circle_color),
                );

                // Center dot
                painter.circle_filled(mouse_pos, 3.0, self.circle_color);
            });

        ctx.request_repaint();
    }
}
