//! Type aliases for Cocoa object IDs.
//!
//! These aliases provide semantic meaning to the generic `AnyObject` pointer type,
//! making function signatures more readable without runtime overhead.
//!
//! Migration note: In objc2, `cocoa::base::id` is replaced by `*mut AnyObject`
//! or typed pointers like `*mut NSView`. We keep these aliases for gradual migration.

#![allow(dead_code)]

use objc2::runtime::AnyObject;

/// Raw pointer to any Objective-C object (equivalent to old `cocoa::base::id`).
/// Prefer using typed pointers (e.g., `*mut NSView`) when the type is known.
pub type Id = *mut AnyObject;

/// A reference to an NSView or subclass.
pub type ViewId = Id;

/// A reference to an NSWindow.
pub type WindowId = Id;

/// A reference to an NSColor.
pub type ColorId = Id;

/// A reference to an NSBezierPath.
pub type BezierPathId = Id;

/// A reference to an NSAffineTransform.
pub type TransformId = Id;

/// A reference to an NSFont.
pub type FontId = Id;

/// A reference to an NSScreen.
pub type ScreenId = Id;

/// A reference to an NSMenu.
pub type MenuId = Id;

/// A reference to an NSMenuItem.
pub type MenuItemId = Id;

/// A reference to an NSStatusItem.
pub type StatusItemId = Id;

/// A reference to an NSImage.
pub type ImageId = Id;

/// A reference to an NSButton.
pub type ButtonId = Id;

/// A reference to an NSTextField.
pub type TextFieldId = Id;

/// A reference to an NSSlider.
pub type SliderId = Id;

/// A reference to an NSColorWell.
pub type ColorWellId = Id;

/// A reference to an NSPopUpButton.
pub type PopUpButtonId = Id;

/// A reference to an NSTimer.
pub type TimerId = Id;

/// Generic Cocoa object reference (for cases where specific type is unknown).
pub type ObjectId = Id;

/// Null pointer constant for Objective-C (equivalent to old `cocoa::base::nil`).
pub const NIL: Id = std::ptr::null_mut();
