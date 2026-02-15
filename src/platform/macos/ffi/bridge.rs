//! Compatibility bridge for cocoa/objc → objc2 migration.
//!
//! This module provides type aliases and re-exports that allow existing code
//! to work with minimal changes during the migration from the deprecated
//! `cocoa`/`objc`/`block` crates to the `objc2` ecosystem.
//!
//! **Migration guide**:
//! 1. Replace `use cocoa::base::{id, nil, YES, NO}` with `use crate::platform::macos::ffi::bridge::*`
//! 2. Replace `use objc::{class, msg_send, sel, sel_impl}` with `use crate::platform::macos::ffi::bridge::*`
//! 3. Replace `use block::ConcreteBlock` with `use crate::platform::macos::ffi::bridge::*`
//!
//! After migration is complete, this module can be removed and direct imports used.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

// ============================================================================
// Core objc2 re-exports
// ============================================================================

pub use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};
pub use objc2::{class, msg_send, sel, ClassType};

// ============================================================================
// Type aliases for backward compatibility
// ============================================================================

/// Objective-C object pointer (replaces `cocoa::base::id`).
///
/// In objc2, prefer using typed pointers like `&NSView` or `Retained<NSString>`
/// when the type is known. Use `id` only for truly dynamic/unknown types.
pub type id = *mut AnyObject;

/// Null pointer constant (replaces `cocoa::base::nil`).
pub const nil: id = std::ptr::null_mut();

/// Boolean YES (replaces `cocoa::base::YES`).
/// This is the Objective-C BOOL type (u8), not Rust bool.
pub const YES: Bool = Bool::YES;

/// Boolean NO (replaces `cocoa::base::NO`).
/// This is the Objective-C BOOL type (u8), not Rust bool.
pub const NO: Bool = Bool::NO;

/// Re-export Bool for explicit usage
pub use objc2::runtime::Bool as ObjcBool;

// ============================================================================
// Foundation types (geometry)
// ============================================================================

// NSPoint, NSRect, NSSize are available with NSGeometry + objc2-core-foundation features
pub use objc2_foundation::{NSPoint, NSRect, NSSize};

// ============================================================================
// Foundation classes
// ============================================================================

pub use objc2_foundation::{NSArray, NSDictionary, NSString};

// ============================================================================
// AppKit classes
// ============================================================================

pub use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSScreen, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};

// ============================================================================
// Block support (replaces `block` crate)
// ============================================================================

pub use block2::{Block, RcBlock, StackBlock};

// ============================================================================
// Memory management
// ============================================================================

pub use objc2::rc::Retained;

// ============================================================================
// Helper functions
// ============================================================================

/// Get the shared NSApplication instance.
///
/// Replaces `cocoa::appkit::NSApp()`.
#[inline]
#[allow(non_snake_case)]
pub fn NSApp() -> id {
    unsafe { msg_send![NSApplication::class(), sharedApplication] }
}

/// Create an NSString from a Rust string slice.
///
/// Returns a retained NSString. For raw pointer, use `.as_ptr() as id`.
#[inline]
pub fn nsstring(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

/// Create an NSString and return as raw id pointer.
///
/// Useful for passing to msg_send! that expects id.
/// The returned pointer is retained - caller must manage memory.
#[inline]
pub fn nsstring_id(s: &str) -> id {
    let ns = NSString::from_str(s);
    Retained::into_raw(ns) as id
}

// ============================================================================
// Runtime helpers
// ============================================================================

/// Get a class by name, panicking if not found.
///
/// Replaces `objc::runtime::Class::get("ClassName").unwrap()`.
#[inline]
pub fn get_class(name: &str) -> &'static AnyClass {
    // Convert to CStr for AnyClass::get
    let c_name = std::ffi::CString::new(name).expect("Invalid class name");
    AnyClass::get(&c_name).unwrap_or_else(|| panic!("Class '{}' not found", name))
}

// ============================================================================
// Object trait extensions for ivar access
// ============================================================================

use objc2::encode::Encode;

/// Extension trait for accessing instance variables on AnyObject.
///
/// Uses `Ivar::load`/`Ivar::load_mut` internally (the non-deprecated API).
/// Method names are intentionally different from the deprecated AnyObject methods
/// to avoid ambiguity.
pub trait ObjectExt {
    /// Load a reference to an instance variable.
    ///
    /// # Safety
    /// - The ivar must exist and be of type T
    /// - Must be called from the main thread for UI objects
    unsafe fn load_ivar<T: Encode>(&self, name: &str) -> &T;

    /// Load a mutable reference to an instance variable.
    ///
    /// # Safety
    /// - The ivar must exist and be of type T
    /// - Must be called from the main thread for UI objects
    unsafe fn load_ivar_mut<T: Encode>(&mut self, name: &str) -> &mut T;

    /// Store a value in an instance variable.
    ///
    /// # Safety
    /// - The ivar must exist and be of type T
    /// - Must be called from the main thread for UI objects
    unsafe fn store_ivar<T: Encode>(&mut self, name: &str, value: T);
}

impl ObjectExt for AnyObject {
    unsafe fn load_ivar<T: Encode>(&self, name: &str) -> &T {
        let cls = self.class();
        let c_name = std::ffi::CString::new(name).unwrap();
        let ivar = cls
            .instance_variable(&c_name)
            .unwrap_or_else(|| panic!("ivar '{}' not found", name));
        ivar.load::<T>(self)
    }

    unsafe fn load_ivar_mut<T: Encode>(&mut self, name: &str) -> &mut T {
        let cls = self.class();
        let c_name = std::ffi::CString::new(name).unwrap();
        let ivar = cls
            .instance_variable(&c_name)
            .unwrap_or_else(|| panic!("ivar '{}' not found", name));
        ivar.load_mut::<T>(self)
    }

    unsafe fn store_ivar<T: Encode>(&mut self, name: &str, value: T) {
        let cls = self.class();
        let c_name = std::ffi::CString::new(name).unwrap();
        let ivar = cls
            .instance_variable(&c_name)
            .unwrap_or_else(|| panic!("ivar '{}' not found", name));
        *ivar.load_mut::<T>(self) = value;
    }
}

// ============================================================================
// Boolean ivar helpers (workaround for bool not implementing Encode)
// ============================================================================

/// Read a boolean ivar stored as `bool` in the object.
///
/// This is a workaround because `bool` doesn't implement `Encode` in objc2.
/// We read it as raw bytes since `bool` in Rust is typically 1 byte.
///
/// # Safety
/// The ivar must exist and be of type bool.
pub unsafe fn get_bool_ivar(obj: id, name: &str) -> bool {
    let obj_ref = &*obj;
    let cls = obj_ref.class();
    let c_name = std::ffi::CString::new(name).unwrap();
    let ivar = cls
        .instance_variable(&c_name)
        .unwrap_or_else(|| panic!("ivar '{}' not found", name));
    // Read as u8 (same size as bool) and convert
    let val: u8 = *ivar.load::<u8>(obj_ref);
    val != 0
}

/// Write a boolean ivar stored as `bool` in the object.
///
/// # Safety
/// The ivar must exist and be of type bool.
pub unsafe fn set_bool_ivar(obj: id, name: &str, val: bool) {
    let obj_ref = &mut *obj;
    let cls = obj_ref.class();
    let c_name = std::ffi::CString::new(name).unwrap();
    let ivar = cls
        .instance_variable(&c_name)
        .unwrap_or_else(|| panic!("ivar '{}' not found", name));
    // Write as u8
    *ivar.load_mut::<u8>(obj_ref) = if val { 1 } else { 0 };
}

// ============================================================================
// NSAutoreleasePool replacement
// ============================================================================

/// Run a closure within an autorelease pool.
///
/// Replaces the pattern:
/// ```text
/// let _pool = NSAutoreleasePool::new(nil);
/// // ... code ...
/// ```
///
/// With:
/// ```text
/// autoreleasepool(|| {
///     // ... code ...
/// });
/// ```
#[inline]
pub fn autoreleasepool<R, F: FnOnce() -> R>(f: F) -> R {
    // objc2 doesn't have a direct autoreleasepool function in the public API,
    // so we create one manually using the runtime
    unsafe {
        let pool: id = msg_send![get_class("NSAutoreleasePool"), new];
        let result = f();
        let _: () = msg_send![pool, drain];
        result
    }
}
