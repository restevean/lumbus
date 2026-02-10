//! Type definitions and re-exports for Windows FFI.
//!
//! This module re-exports commonly used Windows types from the `windows` crate
//! and defines application-specific constants.

// Re-export windows-rs types we use frequently
pub use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
pub use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, Ellipse, EndPaint, GetStockObject,
    SelectObject, SetBkMode, HBRUSH, HDC, HGDIOBJ, HPEN, NULL_BRUSH, PAINTSTRUCT, PS_SOLID,
    TRANSPARENT,
};
pub use windows::Win32::System::LibraryLoader::GetModuleHandleW;
pub use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
pub use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    LoadCursorW, PostQuitMessage, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HMENU, HWND_TOPMOST,
    IDC_ARROW, LWA_ALPHA, LWA_COLORKEY, MSG, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CREATE, WM_DESTROY,
    WM_HOTKEY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

// Application-specific hotkey IDs
pub const HOTKEY_TOGGLE: i32 = 1;
pub const HOTKEY_SETTINGS: i32 = 2;
pub const HOTKEY_HELP: i32 = 3;
pub const HOTKEY_QUIT: i32 = 4;

// Timer ID for cursor tracking
pub const TIMER_CURSOR: usize = 1;
pub const TIMER_INTERVAL_MS: u32 = 16; // ~60 FPS

// Transparency color key (magenta, commonly used for transparency)
pub const TRANSPARENT_COLOR: u32 = 0x00FF00FF; // RGB(255, 0, 255)
