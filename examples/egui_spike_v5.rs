//! Spike V5: Test secondary viewports for multi-monitor
//!
//! Goal: Validate that we can create multiple egui windows,
//! one per monitor, each configured as an overlay.
//!
//! Run with: cargo run --example egui_spike_v5

use eframe::egui::{self, ViewportBuilder, ViewportId};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod macos {
    use core_graphics::display::{CGDisplay, CGGetActiveDisplayList};

    pub const OVERLAY_WINDOW_LEVEL: i64 = 102;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY: u64 = 1 << 4;

    extern "C" {
        fn objc_msgSend(
            obj: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            ...
        ) -> *mut std::ffi::c_void;
        fn sel_registerName(name: *const i8) -> *mut std::ffi::c_void;
    }

    pub unsafe fn configure_nswindow(ns_view_ptr: *mut std::ffi::c_void, screen_idx: usize) {
        if ns_view_ptr.is_null() {
            println!("[Screen {}] NSView is null!", screen_idx);
            return;
        }

        let sel_window = sel_registerName(b"window\0".as_ptr() as *const i8);
        let window = objc_msgSend(ns_view_ptr, sel_window);
        if window.is_null() {
            println!("[Screen {}] NSWindow is null!", screen_idx);
            return;
        }

        let sel_set_level = sel_registerName(b"setLevel:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_level, OVERLAY_WINDOW_LEVEL);

        let behavior = NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES
            | NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY
            | NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY;
        let sel_set_behavior = sel_registerName(b"setCollectionBehavior:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_behavior, behavior);

        let sel_set_ignores = sel_registerName(b"setIgnoresMouseEvents:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_ignores, 1i32);

        println!("[Screen {}] ✅ NSWindow configured", screen_idx);
    }

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
    }

    pub fn get_mouse_position() -> (f64, f64) {
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

    #[derive(Debug, Clone)]
    pub struct ScreenInfo {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    impl ScreenInfo {
        pub fn contains(&self, x: f64, y: f64) -> bool {
            x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
        }
        pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
            (x - self.x, y - self.y)
        }
    }

    pub fn get_screens() -> Vec<ScreenInfo> {
        let mut ids = vec![0u32; 16];
        let mut count = 0u32;
        unsafe {
            CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut count);
        }

        let mut screens = Vec::new();
        for i in 0..count as usize {
            let display = CGDisplay::new(ids[i]);
            let b = display.bounds();
            println!(
                "Screen {}: ({}, {}) {}x{}",
                i, b.origin.x, b.origin.y, b.size.width, b.size.height
            );
            screens.push(ScreenInfo {
                x: b.origin.x,
                y: b.origin.y,
                width: b.size.width,
                height: b.size.height,
            });
        }
        screens
    }
}

#[cfg(target_os = "macos")]
use macos::*;

fn main() -> eframe::Result<()> {
    let screens = get_screens();
    println!("Found {} screens", screens.len());

    if screens.is_empty() {
        eprintln!("No screens found!");
        return Ok(());
    }

    // Main window on screen 0
    let screen0 = &screens[0];
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Spike V5 - Screen 0")
            .with_position([screen0.x as f32, screen0.y as f32])
            .with_inner_size([screen0.width as f32, screen0.height as f32])
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    let screens_arc = Arc::new(screens);

    eframe::run_native(
        "Spike V5",
        opts,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            Ok(Box::new(App {
                screens: screens_arc.clone(),
                main_configured: false,
                secondary_configured: vec![false; screens_arc.len()],
            }))
        }),
    )
}

struct App {
    screens: Arc<Vec<ScreenInfo>>,
    main_configured: bool,
    secondary_configured: Vec<bool>,
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Configure main window (screen 0)
        if !self.main_configured {
            #[cfg(target_os = "macos")]
            if let Ok(h) = frame.window_handle() {
                if let RawWindowHandle::AppKit(ak) = h.as_raw() {
                    unsafe {
                        configure_nswindow(ak.ns_view.as_ptr() as *mut _, 0);
                    }
                }
            }
            self.main_configured = true;
        }

        let (mx, my) = get_mouse_position();

        // Find which screen has the cursor
        let active_screen = self
            .screens
            .iter()
            .position(|s| s.contains(mx, my))
            .unwrap_or(0);

        // Draw on main window (screen 0)
        let screen0 = &self.screens[0];
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if active_screen == 0 {
                    let (lx, ly) = screen0.to_local(mx, my);
                    let p = ui.painter();
                    let pos = egui::pos2(lx as f32, ly as f32);
                    p.circle_stroke(
                        pos,
                        40.0,
                        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 204, 0)),
                    );
                    p.circle_filled(pos, 3.0, egui::Color32::from_rgb(255, 204, 0));
                }
            });

        // Create secondary viewports for other screens
        for (idx, screen) in self.screens.iter().enumerate().skip(1) {
            let viewport_id = ViewportId::from_hash_of(format!("screen_{}", idx));
            let is_active = idx == active_screen;
            let local_pos = screen.to_local(mx, my);

            // Use show_viewport_immediate for more control
            ctx.show_viewport_immediate(
                viewport_id,
                ViewportBuilder::default()
                    .with_title(format!("Spike V5 - Screen {}", idx))
                    .with_position([screen.x as f32, screen.y as f32])
                    .with_inner_size([screen.width as f32, screen.height as f32])
                    .with_transparent(true)
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_mouse_passthrough(true),
                |ctx, _class| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            if is_active {
                                let p = ui.painter();
                                let pos = egui::pos2(local_pos.0 as f32, local_pos.1 as f32);
                                p.circle_stroke(
                                    pos,
                                    40.0,
                                    egui::Stroke::new(3.0, egui::Color32::GREEN),
                                );
                                p.circle_filled(pos, 3.0, egui::Color32::GREEN);
                            }
                        });
                },
            );
        }

        ctx.request_repaint();
    }
}
