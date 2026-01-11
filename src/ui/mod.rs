//! User interface module.
//!
//! This module will contain UI components once decoupled:
//!
//! ## overlay/
//! - view_class.rs: CustomView class registration and ivars
//! - view_methods.rs: extern "C" methods for the view
//! - drawing.rs: Circle and letter drawing logic
//! - window.rs: make_window_for_screen
//!
//! ## settings/
//! - window.rs: open_settings_window, close_settings_window
//! - controls.rs: mk_label, mk_value_label, mk_slider helpers
//! - actions.rs: Slider/color callbacks
//!
//! ## dialogs/
//! - quit_dialog.rs: confirm_and_maybe_quit
//!
//! Currently these functions remain in main.rs due to tight coupling with:
//! - apply_to_all_views (global helper)
//! - lang_is_es (localization helper)
//! - reinstall_hotkeys (Carbon hotkey management)
//! - Direct ivar access patterns
//!
//! A deeper refactor would require:
//! 1. Event/callback system for decoupling
//! 2. Dependency injection patterns
//! 3. Shared state management abstraction

// Placeholder - UI functions will be moved here after architectural improvements
