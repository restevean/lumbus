//! Type aliases for Cocoa object IDs.
//!
//! These aliases provide semantic meaning to the generic `id` type,
//! making function signatures more readable without runtime overhead.
//!
//! Note: These are available for gradual adoption across the codebase.

#![allow(dead_code)]

use cocoa::base::id;

/// A reference to an NSView or subclass.
pub type ViewId = id;

/// A reference to an NSWindow.
pub type WindowId = id;

/// A reference to an NSColor.
pub type ColorId = id;

/// A reference to an NSBezierPath.
pub type BezierPathId = id;

/// A reference to an NSAffineTransform.
pub type TransformId = id;

/// A reference to an NSFont.
pub type FontId = id;

/// A reference to an NSScreen.
pub type ScreenId = id;

/// A reference to an NSMenu.
pub type MenuId = id;

/// A reference to an NSMenuItem.
pub type MenuItemId = id;

/// A reference to an NSStatusItem.
pub type StatusItemId = id;

/// A reference to an NSImage.
pub type ImageId = id;

/// A reference to an NSButton.
pub type ButtonId = id;

/// A reference to an NSTextField.
pub type TextFieldId = id;

/// A reference to an NSSlider.
pub type SliderId = id;

/// A reference to an NSColorWell.
pub type ColorWellId = id;

/// A reference to an NSPopUpButton.
pub type PopUpButtonId = id;

/// A reference to an NSTimer.
pub type TimerId = id;

/// Generic Cocoa object reference (for cases where specific type is unknown).
pub type ObjectId = id;
