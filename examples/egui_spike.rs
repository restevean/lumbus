//! Spike: Validate egui for Lumbus overlay
//!
//! This spike tests if egui/eframe can:
//! 1. Create a transparent, borderless, always-on-top window
//! 2. Draw a circle that follows the mouse cursor
//! 3. Allow click-through (ideally)
//!
//! Run with: cargo run --example egui_spike

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumbus Spike")
            .with_inner_size([400.0, 400.0])
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true), // Click-through!
        ..Default::default()
    };

    eframe::run_native(
        "Lumbus egui Spike",
        options,
        Box::new(|cc| {
            // Set transparent background
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            Ok(Box::new(LumbusSpike::default()))
        }),
    )
}

struct LumbusSpike {
    circle_radius: f32,
    circle_color: egui::Color32,
    border_width: f32,
}

impl Default for LumbusSpike {
    fn default() -> Self {
        Self {
            circle_radius: 40.0,
            circle_color: egui::Color32::from_rgb(255, 204, 0), // Yellow like current Lumbus
            border_width: 3.0,
        }
    }
}

impl eframe::App for LumbusSpike {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Fully transparent background
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Get mouse position
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(200.0, 200.0)));

        // Draw on the entire screen
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let painter = ui.painter();

                // Draw circle at mouse position
                // Stroke only (not filled) to match current Lumbus behavior
                painter.circle_stroke(
                    mouse_pos,
                    self.circle_radius,
                    egui::Stroke::new(self.border_width, self.circle_color),
                );

                // Draw a small dot at center to verify position
                painter.circle_filled(mouse_pos, 3.0, self.circle_color);
            });

        // Request continuous repaint to follow mouse smoothly
        ctx.request_repaint();
    }
}
