//! Application events for inter-module communication.
//!
//! These events represent high-level application actions that can be
//! published by any module and handled by the event dispatcher.
//! This module is pure Rust with no FFI dependencies, making it fully testable.

/// Application-level events for decoupled communication between modules.
///
/// Events flow from producers (hotkeys, UI, observers) through the EventBus
/// to the dispatcher, which executes the appropriate actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    // === Input Events ===
    /// Toggle overlay visibility (Ctrl+A)
    ToggleOverlay,

    /// Open settings window (Cmd+, or Cmd+;)
    OpenSettings,

    /// Request application quit with confirmation dialog (Ctrl+Shift+X)
    RequestQuit,

    /// Show About dialog
    ShowAbout,

    // === UI Lifecycle Events ===
    /// Settings window was closed by user
    SettingsClosed,

    /// Quit dialog was cancelled (user chose not to quit)
    QuitCancelled,

    // === System Events ===
    /// Hotkeys need to be reinstalled (after sleep/wake, space change, etc.)
    ReinstallHotkeys,
}

impl AppEvent {
    /// Returns true if this event should trigger hotkey reinstallation.
    ///
    /// After certain UI operations complete (settings closed, quit cancelled),
    /// we need to reinstall Carbon hotkeys to ensure they work properly.
    pub fn requires_hotkey_reinstall(&self) -> bool {
        matches!(
            self,
            AppEvent::SettingsClosed | AppEvent::QuitCancelled | AppEvent::ReinstallHotkeys
        )
    }

    /// Returns a human-readable description of the event for debugging.
    pub fn description(&self) -> &'static str {
        match self {
            AppEvent::ToggleOverlay => "Toggle overlay visibility",
            AppEvent::OpenSettings => "Open settings window",
            AppEvent::RequestQuit => "Request quit with confirmation",
            AppEvent::ShowAbout => "Show about dialog",
            AppEvent::SettingsClosed => "Settings window closed",
            AppEvent::QuitCancelled => "Quit cancelled by user",
            AppEvent::ReinstallHotkeys => "Reinstall hotkeys",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_reinstall_required_for_ui_close_events() {
        assert!(AppEvent::SettingsClosed.requires_hotkey_reinstall());
        assert!(AppEvent::QuitCancelled.requires_hotkey_reinstall());
        assert!(AppEvent::ReinstallHotkeys.requires_hotkey_reinstall());
    }

    #[test]
    fn test_hotkey_reinstall_not_required_for_action_events() {
        assert!(!AppEvent::ToggleOverlay.requires_hotkey_reinstall());
        assert!(!AppEvent::OpenSettings.requires_hotkey_reinstall());
        assert!(!AppEvent::RequestQuit.requires_hotkey_reinstall());
        assert!(!AppEvent::ShowAbout.requires_hotkey_reinstall());
    }

    #[test]
    fn test_event_equality() {
        assert_eq!(AppEvent::ToggleOverlay, AppEvent::ToggleOverlay);
        assert_ne!(AppEvent::ToggleOverlay, AppEvent::OpenSettings);
    }

    #[test]
    fn test_event_clone() {
        let event = AppEvent::OpenSettings;
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_event_debug() {
        let event = AppEvent::ToggleOverlay;
        let debug_str = format!("{:?}", event);
        assert_eq!(debug_str, "ToggleOverlay");
    }

    #[test]
    fn test_all_events_have_descriptions() {
        let events = [
            AppEvent::ToggleOverlay,
            AppEvent::OpenSettings,
            AppEvent::RequestQuit,
            AppEvent::ShowAbout,
            AppEvent::SettingsClosed,
            AppEvent::QuitCancelled,
            AppEvent::ReinstallHotkeys,
        ];

        for event in events {
            assert!(!event.description().is_empty());
        }
    }
}
