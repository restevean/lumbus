//! Spike V4: Multi-monitor support
//!
//! Run with: cargo run --example egui_spike_v4

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod macos {
    use core_graphics::display::{CGDisplay, CGGetActiveDisplayList};

    pub const NS_POP_UP_MENU_WINDOW_LEVEL: i64 = 101;
    pub const OVERLAY_WINDOW_LEVEL: i64 = NS_POP_UP_MENU_WINDOW_LEVEL + 1;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
    pub const NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY: u64 = 1 << 4;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct NSRect {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    extern "C" {
        fn objc_msgSend(
            obj: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            ...
        ) -> *mut std::ffi::c_void;
        fn sel_registerName(name: *const i8) -> *mut std::ffi::c_void;
    }

    pub unsafe fn configure_overlay_window(ns_view_ptr: *mut std::ffi::c_void, bbox: &BoundingBox) {
        if ns_view_ptr.is_null() {
            return;
        }

        let sel_window = sel_registerName(b"window\0".as_ptr() as *const i8);
        let window = objc_msgSend(ns_view_ptr, sel_window);
        if window.is_null() {
            return;
        }

        // TEMPORARILY SKIP frame change to test if circle appears
        let _ = bbox; // suppress unused warning
        println!("   [DEBUG] Skipping setFrame - testing basic rendering");

        let sel_set_level = sel_registerName(b"setLevel:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_level, OVERLAY_WINDOW_LEVEL);

        let behavior = NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES
            | NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_AUXILIARY
            | NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY;
        let sel_set_behavior = sel_registerName(b"setCollectionBehavior:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_behavior, behavior);

        let sel_set_ignores = sel_registerName(b"setIgnoresMouseEvents:\0".as_ptr() as *const i8);
        objc_msgSend(window, sel_set_ignores, 1i32);

        println!("✅ NSWindow configured");
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
    pub struct BoundingBox {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    impl BoundingBox {
        pub fn to_local(&self, gx: f64, gy: f64) -> (f64, f64) {
            (gx - self.x, gy - self.y)
        }
    }

    pub fn get_bounding_box() -> BoundingBox {
        let mut ids = vec![0u32; 16];
        let mut count = 0u32;

        unsafe {
            CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut count);
        }

        if count == 0 {
            return BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            };
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for i in 0..count as usize {
            let display = CGDisplay::new(ids[i]);
            let b = display.bounds();
            println!(
                "Screen {}: origin=({}, {}), size={}x{}",
                i, b.origin.x, b.origin.y, b.size.width, b.size.height
            );
            min_x = min_x.min(b.origin.x);
            min_y = min_y.min(b.origin.y);
            max_x = max_x.max(b.origin.x + b.size.width);
            max_y = max_y.max(b.origin.y + b.size.height);
        }

        let bbox = BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        };
        println!(
            "BBox: ({}, {}) {}x{}",
            bbox.x, bbox.y, bbox.width, bbox.height
        );
        bbox
    }
}

#[cfg(target_os = "macos")]
use macos::*;

fn main() -> eframe::Result<()> {
    let bbox = get_bounding_box();

    let hk_mgr = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
    hk_mgr.register(hotkey).unwrap();

    let visible = Arc::new(AtomicBool::new(true));
    let vis_clone = visible.clone();

    std::thread::spawn(move || loop {
        if let Ok(ev) = GlobalHotKeyEvent::receiver().recv() {
            if ev.id == hotkey.id() && ev.state == HotKeyState::Pressed {
                let cur = vis_clone.load(Ordering::SeqCst);
                vis_clone.store(!cur, Ordering::SeqCst);
                println!("Ctrl+A → {}", if !cur { "ON" } else { "OFF" });
            }
        }
    });

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_position([bbox.x as f32, bbox.y as f32])
            .with_inner_size([bbox.width as f32, bbox.height as f32])
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "Spike V4",
        opts,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            Ok(Box::new(App {
                visible: visible.clone(),
                bbox: bbox.clone(),
                configured: false,
            }))
        }),
    )
}

struct App {
    visible: Arc<AtomicBool>,
    bbox: BoundingBox,
    configured: bool,
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.configured {
            #[cfg(target_os = "macos")]
            if let Ok(h) = frame.window_handle() {
                if let RawWindowHandle::AppKit(ak) = h.as_raw() {
                    unsafe {
                        configure_overlay_window(ak.ns_view.as_ptr() as *mut _, &self.bbox);
                    }
                }
            }
            self.configured = true;
        }

        if !self.visible.load(Ordering::SeqCst) {
            ctx.request_repaint();
            return;
        }

        let (mx, my) = get_mouse_position();
        let (lx, ly) = self.bbox.to_local(mx, my);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let p = ui.painter();
                let pos = egui::pos2(lx as f32, ly as f32);
                p.circle_stroke(
                    pos,
                    40.0,
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 204, 0)),
                );
                p.circle_filled(pos, 3.0, egui::Color32::from_rgb(255, 204, 0));
            });

        ctx.request_repaint();
    }
}
