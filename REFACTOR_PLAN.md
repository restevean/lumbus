# Plan de Refactorización: mouse_highlighter

> **Estado**: EN PROGRESO
> **Última actualización**: 2026-01-11
> **Rama**: `refactor/modular-architecture`

---

## Resumen Ejecutivo

**Problema**: `main.rs` tiene 2174 líneas con 7 responsabilidades mezcladas, violando SRP. La "mega-función" `register_custom_view_class_and_create_view` tiene 590 líneas definiendo toda la clase Objective-C inline.

**Solución**: Arquitectura modular por capas (`ffi/`, `model/`, `ui/`, `input/`) reduciendo `main.rs` a ~100 líneas y maximizando testabilidad.

**Principios aplicados**:
- **SOLID**: Especialmente SRP (cada módulo una responsabilidad) y DIP (model no depende de FFI)
- **TDD**: Tests primero para `model/`
- **Pragmatismo**: No sobre-abstraer el runtime Obj-C

---

## Contexto del Proyecto (para recuperación de contexto)

### ¿Qué es mouse_highlighter?
Aplicación macOS nativa en Rust que:
- Dibuja un círculo transparente alrededor del cursor
- Muestra "L" o "R" al hacer click izquierdo/derecho
- Funciona en múltiples monitores
- Configurable via ventana de Settings (radio, color, transparencia, idioma)
- Hotkeys: Ctrl+A (toggle), ⌘+, (settings), Ctrl+Shift+X (quit)

### Stack técnico
- Rust con crates `cocoa`, `objc`, `block`
- FFI directo a Carbon (hotkeys), CoreText (glyphs), CoreGraphics
- NSUserDefaults para persistencia
- No hay wrappers de alto nivel - todo es `unsafe` y `msg_send!`

### Estado actual de archivos
| Archivo | Líneas | Descripción |
|---------|--------|-------------|
| `src/main.rs` | 2174 | Monolito con todo |
| `src/lib.rs` | ~100 | Helpers puros (clamp, color_to_hex, parse_hex_color, tr_key) |
| `tests/helpers.rs` | ~117 | 16 tests para lib.rs (100% coverage) |

---

## Estructura Final Propuesta

```
src/
├── main.rs                      # (~80-100 líneas) Entry point mínimo
├── lib.rs                       # (existente) Helpers puros
│
├── ffi/                         # FFI bindings encapsulados
│   ├── mod.rs                   # Re-exports públicos
│   ├── carbon.rs                # Carbon Event Manager (hotkeys)
│   ├── coretext.rs              # CoreText (glyph rendering)
│   ├── coregraphics.rs          # CoreGraphics/CoreFoundation
│   ├── accessibility.rs         # ApplicationServices (AX permisos)
│   └── cocoa_utils.rs           # nsstring(), display_id, helpers
│
├── model/                       # Lógica pura testeable (SIN FFI)
│   ├── mod.rs
│   ├── constants.rs             # Defaults, keys, modifiers, keycodes
│   ├── app_state.rs             # OverlayState struct
│   └── preferences.rs           # Load/save centralizado
│
├── ui/                          # Interfaz de usuario
│   ├── mod.rs
│   ├── overlay/
│   │   ├── mod.rs
│   │   ├── view_class.rs        # Registro de clase Obj-C (solo ivars)
│   │   ├── view_methods.rs      # Métodos extern "C"
│   │   ├── drawing.rs           # Lógica de draw_rect
│   │   └── window.rs            # make_window_for_screen
│   ├── settings/
│   │   ├── mod.rs
│   │   ├── window.rs            # Construcción de ventana
│   │   ├── controls.rs          # Builders: mk_label, mk_slider, etc.
│   │   └── actions.rs           # Callbacks: set_radius, color_changed...
│   └── dialogs/
│       ├── mod.rs
│       └── quit_dialog.rs       # Confirmación de salida
│
└── input/                       # Manejo de input
    ├── mod.rs
    ├── hotkeys.rs               # Carbon hotkeys
    ├── mouse_monitors.rs        # NSEvent mouse monitors
    ├── keyboard_monitors.rs     # Local key monitors
    └── observers.rs             # Wake/space/termination observers

tests/
├── helpers.rs                   # (existente) Tests para lib.rs
└── model_tests.rs               # (nuevo) Tests para model/
```

---

## Progreso General

| Fase | Estado | Descripción |
|------|--------|-------------|
| Fase 1 | ✅ COMPLETADO | Preparación y rama |
| Fase 2 | ✅ COMPLETADO | Extraer FFI bindings |
| Fase 3 | ⬜ PENDIENTE | Extraer model + tests |
| Fase 4 | ⬜ PENDIENTE | Extraer input handlers |
| Fase 5 | ⬜ PENDIENTE | Modularizar UI |
| Fase 6 | ⬜ PENDIENTE | Cleanup final |

**Leyenda**: ⬜ Pendiente | 🔄 En progreso | ✅ Completado | ❌ Bloqueado

---

## Fase 1: Preparación y rama

**Objetivo**: Crear la rama y estructura de directorios base sin modificar código.

### Tareas
- [x] 1.1 Crear rama `refactor/modular-architecture` desde `main`
- [x] 1.2 Crear directorios vacíos:
  - [x] `src/ffi/`
  - [x] `src/model/`
  - [x] `src/ui/overlay/`
  - [x] `src/ui/settings/`
  - [x] `src/ui/dialogs/`
  - [x] `src/input/`
- [x] 1.3 Crear archivos `mod.rs` vacíos en cada directorio
- [x] 1.4 Verificar `cargo build` compila sin cambios

### Verificación
```bash
cargo build --release
# Debe compilar sin errores ni warnings nuevos
```

### Commit sugerido
```
feat: create directory structure for modular architecture

- Add empty module directories (ffi, model, ui, input)
- No functional changes yet
```

---

## Fase 2: Extraer FFI bindings

**Objetivo**: Mover TODOS los `extern "C"` declarations y tipos FFI a `src/ffi/`.

**Riesgo**: Bajo - solo mueve declaraciones, no cambia lógica.

### 2.1 Crear `src/ffi/carbon.rs`

**Líneas origen en main.rs**: 20-106 (~86 líneas)

**Contenido a mover**:
```rust
// Tipos
#[repr(C)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

#[repr(C)]
pub struct EventHotKeyID {
    pub signature: u32,
    pub id: u32,
}

pub type EventHotKeyRef = *mut std::ffi::c_void;
pub type EventHandlerRef = *mut std::ffi::c_void;
pub type EventRef = *mut std::ffi::c_void;
pub type EventTargetRef = *mut std::ffi::c_void;

// Constantes
pub const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7962; // 'keyb'
pub const K_EVENT_HOTKEY_PRESSED: u32 = 5;
pub const CMD_KEY: u32 = 1 << 8;
pub const SHIFT_KEY: u32 = 1 << 9;
pub const CONTROL_KEY: u32 = 1 << 12;

pub const KC_A: u32 = 0x00;
pub const KC_X: u32 = 0x07;
pub const KC_COMMA: u32 = 0x2B;
pub const KC_SEMICOLON: u32 = 0x29;

pub const HKID_TOGGLE: u32 = 1;
pub const HKID_SETTINGS_COMMA: u32 = 2;
pub const HKID_SETTINGS_SEMI: u32 = 3;
pub const HKID_QUIT: u32 = 4;

// FFI declarations
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn RegisterEventHotKey(
        inHotKeyCode: u32,
        inHotKeyModifiers: u32,
        inHotKeyID: EventHotKeyID,
        inTarget: EventTargetRef,
        inOptions: u32,
        outRef: *mut EventHotKeyRef,
    ) -> i32;

    pub fn UnregisterEventHotKey(inHotKey: EventHotKeyRef) -> i32;

    pub fn GetApplicationEventTarget() -> EventTargetRef;

    pub fn InstallEventHandler(
        inTarget: EventTargetRef,
        inHandler: extern "C" fn(
            EventHandlerRef,
            EventRef,
            *mut std::ffi::c_void,
        ) -> i32,
        inNumTypes: u32,
        inList: *const EventTypeSpec,
        inUserData: *mut std::ffi::c_void,
        outRef: *mut EventHandlerRef,
    ) -> i32;

    pub fn RemoveEventHandler(inHandlerRef: EventHandlerRef) -> i32;

    pub fn GetEventClass(inEvent: EventRef) -> u32;
    pub fn GetEventKind(inEvent: EventRef) -> u32;
    pub fn GetEventParameter(
        inEvent: EventRef,
        inName: u32,
        inDesiredType: u32,
        outActualType: *mut u32,
        inBufferSize: u32,
        outActualSize: *mut u32,
        outData: *mut EventHotKeyID,
    ) -> i32;
}

pub const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2D2D2D2D; // '----'
pub const TYPE_EVENT_HOTKEY_ID: u32 = 0x686B6964;        // 'hkid'
```

- [x] 2.1.1 Crear archivo `src/ffi/carbon.rs`
- [x] 2.1.2 Mover tipos y constantes de Carbon
- [x] 2.1.3 Mover extern "C" declarations
- [x] 2.1.4 Añadir `pub use` en `src/ffi/mod.rs`

---

### 2.2 Crear `src/ffi/coretext.rs`

**Líneas origen en main.rs**: 109-130 (~22 líneas)

**Contenido a mover**:
```rust
use cocoa::base::id;
use core_foundation::base::CFTypeRef;

pub type CTFontRef = CFTypeRef;
pub type CGPathRef = *mut std::ffi::c_void;

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    pub fn CTFontCreateWithName(
        name: id,           // CFStringRef
        size: f64,          // CGFloat
        matrix: *const (),  // CGAffineTransform*
    ) -> CTFontRef;

    pub fn CTFontGetGlyphsForCharacters(
        font: CTFontRef,
        characters: *const u16,
        glyphs: *mut u16,
        count: isize,
    ) -> bool;

    pub fn CTFontCreatePathForGlyph(
        font: CTFontRef,
        glyph: u16,
        matrix: *const (),
    ) -> CGPathRef;
}
```

- [x] 2.2.1 Crear archivo `src/ffi/coretext.rs`
- [x] 2.2.2 Mover tipos y funciones de CoreText

---

### 2.3 Crear `src/ffi/coregraphics.rs`

**Líneas origen en main.rs**: 131-155 (~25 líneas)

**Contenido a mover**:
```rust
use core_foundation::base::CFTypeRef;

pub type CGPathRef = *mut std::ffi::c_void;
pub type CFDictionaryRef = *mut std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGPathRelease(path: CGPathRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFAbsoluteTimeGetCurrent() -> f64;

    pub fn CFDictionaryCreate(
        allocator: *const (),
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        numValues: isize,
        keyCallBacks: *const (),
        valueCallBacks: *const (),
    ) -> CFDictionaryRef;
}

pub const K_CF_ALLOCATOR_DEFAULT: *const () = std::ptr::null();
pub const K_CF_TYPE_DICTIONARY_KEY_CALLBACKS: *const () = std::ptr::null();
pub const K_CF_TYPE_DICTIONARY_VALUE_CALLBACKS: *const () = std::ptr::null();
```

- [x] 2.3.1 Crear archivo `src/ffi/coregraphics.rs`
- [x] 2.3.2 Mover tipos y funciones de CoreGraphics/CoreFoundation

---

### 2.4 Crear `src/ffi/accessibility.rs`

**Líneas origen en main.rs**: 156-163 (~8 líneas)

**Contenido a mover**:
```rust
use super::coregraphics::CFDictionaryRef;
use core_foundation::base::CFTypeRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

extern "C" {
    pub static kAXTrustedCheckOptionPrompt: CFTypeRef;
}
```

- [x] 2.4.1 Crear archivo `src/ffi/accessibility.rs`
- [x] 2.4.2 Mover función y constante de Accessibility

---

### 2.5 Crear `src/ffi/cocoa_utils.rs`

**Líneas origen en main.rs**: 261-278 + 243-258 (~35 líneas)

**Contenido a mover**:
```rust
use cocoa::base::{id, nil};
use cocoa::foundation::{NSString, NSPoint};
use objc::runtime::Object;
use objc::msg_send;

/// Convierte &str a NSString*
pub unsafe fn nsstring(s: &str) -> id {
    NSString::alloc(nil).init_str(s)
}

/// Obtiene el CGDirectDisplayID estable para una NSScreen
pub unsafe fn display_id_for_screen(screen: id) -> u32 {
    let desc: id = msg_send![screen, deviceDescription];
    let key = nsstring("NSScreenNumber");
    let val: id = msg_send![desc, objectForKey: key];
    let display_id: u32 = msg_send![val, unsignedIntValue];
    display_id
}

/// Obtiene la posición global del mouse
pub unsafe fn get_mouse_position_cocoa() -> (f64, f64) {
    let loc: NSPoint = msg_send![class!(NSEvent), mouseLocation];
    (loc.x, loc.y)
}

/// Nivel de ventana para overlay (encima de popup menus)
pub fn overlay_window_level() -> i64 {
    // NSPopUpMenuWindowLevel + 1
    101 + 1
}
```

- [x] 2.5.1 Crear archivo `src/ffi/cocoa_utils.rs`
- [x] 2.5.2 Mover helpers de Cocoa

---

### 2.6 Crear `src/ffi/mod.rs`

**Contenido**:
```rust
//! FFI bindings para frameworks de macOS.
//!
//! Este módulo encapsula todas las declaraciones `extern "C"` y tipos
//! necesarios para interactuar con Carbon, CoreText, CoreGraphics y Cocoa.

pub mod carbon;
pub mod coretext;
pub mod coregraphics;
pub mod accessibility;
pub mod cocoa_utils;

// Re-exports convenientes
pub use carbon::*;
pub use coretext::{CTFontRef, CTFontCreateWithName, CTFontGetGlyphsForCharacters, CTFontCreatePathForGlyph};
pub use coregraphics::{CGPathRef, CGPathRelease, CFRelease, CFAbsoluteTimeGetCurrent};
pub use accessibility::{AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt};
pub use cocoa_utils::{nsstring, display_id_for_screen, get_mouse_position_cocoa, overlay_window_level};
```

- [x] 2.6.1 Crear archivo `src/ffi/mod.rs`
- [x] 2.6.2 Añadir re-exports

---

### 2.7 Actualizar main.rs

- [x] 2.7.1 Añadir `mod ffi;` al inicio de main.rs
- [x] 2.7.2 Reemplazar declaraciones locales por `use crate::ffi::*;`
- [x] 2.7.3 Eliminar código duplicado de main.rs

### Verificación Fase 2
```bash
cargo build --release  # Debe compilar sin errores
cargo run --release    # Verificar que la app funciona
```

- [x] Build compila
- [x] App abre correctamente
- [x] Overlay se dibuja
- [x] Hotkeys funcionan

### Commit sugerido
```
refactor: extract FFI bindings to src/ffi/

- Move Carbon hotkey FFI to ffi/carbon.rs
- Move CoreText FFI to ffi/coretext.rs
- Move CoreGraphics/Foundation FFI to ffi/coregraphics.rs
- Move Accessibility FFI to ffi/accessibility.rs
- Move Cocoa utilities to ffi/cocoa_utils.rs
- No functional changes
```

---

## Fase 3: Extraer model

**Objetivo**: Crear estructuras de datos puras (sin FFI) y tests TDD.

**Riesgo**: Bajo - crea código nuevo sin modificar lógica existente.

### 3.1 Crear `src/model/constants.rs`

**Líneas origen en main.rs**: 165-185 (~20 líneas)

- [ ] 3.1.1 Crear archivo `src/model/constants.rs`
- [ ] 3.1.2 Mover y organizar constantes

---

### 3.2 Crear `src/model/app_state.rs` (TDD)

- [ ] 3.2.1 Crear `tests/model_tests.rs` con tests primero
- [ ] 3.2.2 Crear `src/model/app_state.rs`
- [ ] 3.2.3 Verificar `cargo test` pasa

---

### 3.3 Crear `src/model/preferences.rs`

- [ ] 3.3.1 Crear `src/model/preferences.rs`
- [ ] 3.3.2 Mover funciones de prefs de main.rs

---

### 3.4 Crear `src/model/mod.rs`

- [ ] 3.4.1 Crear `src/model/mod.rs`

---

### 3.5 Actualizar lib.rs para exports

- [ ] 3.5.1 Actualizar lib.rs con exports de model

---

### Verificación Fase 3

- [ ] `cargo test` pasa (tests existentes + nuevos)
- [ ] Build compila
- [ ] App funciona normalmente

---

## Fase 4: Extraer input handlers

- [ ] 4.1 Crear `src/input/hotkeys.rs`
- [ ] 4.2 Crear `src/input/mouse_monitors.rs`
- [ ] 4.3 Crear `src/input/keyboard_monitors.rs`
- [ ] 4.4 Crear `src/input/observers.rs`
- [ ] 4.5 Crear `src/input/mod.rs`

---

## Fase 5: Modularizar UI

- [ ] 5.1-5.4 Overlay modules
- [ ] 5.5-5.7 Settings modules
- [ ] 5.8 Dialogs modules
- [ ] 5.9 mod.rs files

---

## Fase 6: Cleanup final

- [ ] 6.1 Reducir main.rs
- [ ] 6.2 Revisar visibilidad
- [ ] 6.3 Documentación
- [ ] 6.4 Actualizar CLAUDE.md
- [ ] 6.5 Eliminar/mover este archivo

---

## Métricas de Éxito

| Métrica | Antes | Después |
|---------|-------|---------|
| Líneas en main.rs | 2174 | ~100 |
| Archivos .rs | 2 | ~18 |
| Módulos testeables | 1 (lib.rs) | 3+ (lib, model/*) |
| Coverage tests | ~5% | ~20%+ |
| Función más larga | ~590 líneas | <100 líneas |

---

## Notas para Recuperación de Contexto

Si pierdes el contexto, lee:
1. **Este archivo** - contiene todo el plan y progreso
2. **CLAUDE.md** - contexto del proyecto original
3. **src/lib.rs** - patrón a seguir (helpers puros)
4. **Cargo.toml** - dependencias del proyecto

**Comando para ver estado actual**:
```bash
git status
git log --oneline -10
cargo test
cargo build --release
```
