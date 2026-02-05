//! Spike V2: Full validation for Lumbus migration
//!
//! Tests:
//! 1. Fullscreen transparent overlay
//! 2. Global cursor position tracking (via core-graphics)
//! 3. Global hotkey (Ctrl+A to toggle visibility)
//!
//! Run with: cargo run --example egui_spike_v2

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Core Graphics FFI for global cursor position
mod cg {
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGEventCreate(source: *const std::ffi::c_void) -> *const std::ffi::c_void;
        pub fn CGEventGetLocation(event: *const std::ffi::c_void) -> CGPoint;
        pub fn CFRelease(cf: *const std::ffi::c_void);
    }

    /// Get global mouse position using CoreGraphics.
    /// Returns (x, y) in screen coordinates (origin top-left).
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
}

fn main() -> eframe::Result<()> {
    // Set up global hotkey manager
    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    // Register Ctrl+A hotkey
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
    hotkey_manager
        .register(hotkey)
        .expect("Failed to register Ctrl+A hotkey");

    // Shared visibility state
    let visible = Arc::new(AtomicBool::new(true));
    let visible_clone = visible.clone();

    // Spawn thread to listen for hotkey events
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                // Only react to key PRESS, not release
                if event.id == hotkey.id() && event.state == HotKeyState::Pressed {
                    // Toggle visibility
                    let current = visible_clone.load(Ordering::SeqCst);
                    visible_clone.store(!current, Ordering::SeqCst);
                    println!(
                        "Hotkey Ctrl+A! Visibility: {}",
                        if !current { "ON" } else { "OFF" }
                    );
                }
            }
        }
    });

    // Get primary screen size for fullscreen window
    let (screen_width, screen_height) = get_screen_size();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumbus Spike V2")
            .with_position([0.0, 0.0])
            .with_inner_size([screen_width, screen_height])
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "Lumbus egui Spike V2",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            Ok(Box::new(LumbusSpikeV2 {
                visible: visible.clone(),
                circle_radius: 40.0,
                circle_color: egui::Color32::from_rgb(255, 204, 0),
                border_width: 3.0,
            }))
        }),
    )
}

/// Get screen size using CoreGraphics
fn get_screen_size() -> (f32, f32) {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayPixelsWide(display: u32) -> usize;
        fn CGDisplayPixelsHigh(display: u32) -> usize;
    }

    unsafe {
        let display = CGMainDisplayID();
        let width = CGDisplayPixelsWide(display) as f32;
        let height = CGDisplayPixelsHigh(display) as f32;
        (width, height)
    }
}

struct LumbusSpikeV2 {
    visible: Arc<AtomicBool>,
    circle_radius: f32,
    circle_color: egui::Color32,
    border_width: f32,
}

impl eframe::App for LumbusSpikeV2 {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check visibility (controlled by hotkey)
        if !self.visible.load(Ordering::SeqCst) {
            ctx.request_repaint();
            return;
        }

        // Get GLOBAL mouse position
        let (mouse_x, mouse_y) = cg::get_global_mouse_position();
        let mouse_pos = egui::pos2(mouse_x as f32, mouse_y as f32);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let painter = ui.painter();

                // Draw circle at global cursor position
                painter.circle_stroke(
                    mouse_pos,
                    self.circle_radius,
                    egui::Stroke::new(self.border_width, self.circle_color),
                );

                // Small center dot
                painter.circle_filled(mouse_pos, 3.0, self.circle_color);
            });

        // Continuous repaint for smooth tracking
        ctx.request_repaint();
    }
}
