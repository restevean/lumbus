# Plan de Migracion: cocoa/objc → objc2

## Resumen Ejecutivo

**Objetivo**: Migrar de los crates deprecated (`cocoa`, `objc`, `block`) al ecosistema `objc2`.

**Magnitud del cambio**:
- ~470 llamadas `msg_send!`
- ~71 usos de `class!`
- ~19 bloques (`ConcreteBlock`)
- 1 clase custom (`CustomViewMulti` con ~30 ivars)
- 13 archivos afectados

**Estimacion**: 3-5 sesiones de trabajo intensivo

---

## Diferencias Clave entre APIs

### 1. Tipos Basicos

| cocoa/objc | objc2 | Notas |
|------------|-------|-------|
| `id` | `*mut AnyObject` | O tipos especificos como `Retained<NSView>` |
| `nil` | `std::ptr::null_mut()` | O `None` para `Option<Retained<T>>` |
| `YES` / `NO` | `true` / `false` | O `Bool::YES` / `Bool::NO` |
| `NSPoint`, `NSRect`, `NSSize` | `CGPoint`, `CGRect`, `CGSize` | Re-exportados desde `objc2-foundation` |

### 2. Creacion de Clases

```rust
// ANTES (objc crate)
use objc::declare::ClassDecl;
let mut decl = ClassDecl::new("MyClass", superclass).unwrap();
decl.add_ivar::<f64>("_value");
decl.add_method(sel!(doThing:), my_fn as extern "C" fn(...));
decl.register();

// DESPUES (objc2)
use objc2::runtime::ClassBuilder;
let mut builder = ClassBuilder::new(c"MyClass", NSObject::class()).unwrap();
builder.add_ivar::<Cell<f64>>(c"_value");
unsafe { builder.add_method(sel!(doThing:), my_fn as extern "C-unwind" fn(...)); }
builder.register();
```

### 3. Acceso a Clases

```rust
// ANTES
use objc::class;
let cls = class!(NSString);

// DESPUES
use objc2::ClassType;
let cls = NSString::class();
// O para clases dinamicas:
use objc2::runtime::AnyClass;
let cls = AnyClass::get(c"NSString").unwrap();
```

### 4. Creacion de NSString

```rust
// ANTES
use std::ffi::CString;
let cstr = CString::new("hello").unwrap();
let ns: id = msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];

// DESPUES
use objc2_foundation::NSString;
let ns = NSString::from_str("hello");  // Retained<NSString>
// O para literales:
use objc2_foundation::ns_string;
let ns = ns_string!("hello");  // &'static NSString
```

### 5. Bloques (Callbacks)

```rust
// ANTES
use block::ConcreteBlock;
let block = ConcreteBlock::new(|event: id| { ... }).copy();
let _: id = msg_send![cls, addMonitor: &*block];

// DESPUES
use block2::RcBlock;
let block = RcBlock::new(|event: *mut AnyObject| { ... });
let _: *mut AnyObject = msg_send![cls, addMonitor: &*block];
```

### 6. Instance Variables (ivars)

```rust
// ANTES
(*view).set_ivar::<f64>("_radius", 50.0);
let r = *(*view).get_ivar::<f64>("_radius");

// DESPUES (con ClassBuilder)
// Los ivars se acceden a traves del AnyClass
let cls = AnyClass::get(c"MyClass").unwrap();
let ivar = cls.instance_variable(c"_radius").unwrap();
unsafe { *ivar.load_mut::<Cell<f64>>(obj) = Cell::new(50.0); }
let r = unsafe { ivar.load::<Cell<f64>>(obj) }.get();
```

### 7. msg_send! Macro

```rust
// ANTES (objc crate)
use objc::{msg_send, sel, sel_impl};
let result: id = msg_send![obj, methodName: arg1 second: arg2];

// DESPUES (objc2)
use objc2::{msg_send, sel};
let result: *mut AnyObject = msg_send![obj, methodName: arg1, second: arg2];
// Nota: comas entre argumentos, no espacios
```

---

## Fases de Migracion

### Fase 0: Preparacion (Ya completada parcialmente)
- [x] Actualizar `Cargo.toml` con dependencias objc2
- [x] Migrar `src/ffi/types.rs` (aliases de tipos)
- [x] Migrar `src/ffi/cocoa_utils.rs` (helpers basicos)
- [ ] Crear modulo bridge para compatibilidad temporal

### Fase 1: Modulos FFI de Bajo Nivel
**Archivos**: `src/ffi/*.rs`
**Esfuerzo**: Bajo (ya usan FFI manual, poco que cambiar)

| Archivo | msg_send! | Cambios necesarios |
|---------|-----------|-------------------|
| `carbon.rs` | 0 | Ninguno (FFI puro a Carbon) |
| `coretext.rs` | 0 | Ninguno (FFI puro a CoreText) |
| `coregraphics.rs` | 0 | Ninguno (FFI puro a CG/CF) |
| `accessibility.rs` | 0 | Ninguno (FFI puro) |
| `cocoa_utils.rs` | 3 | Ya migrado |
| `types.rs` | 0 | Ya migrado |

### Fase 2: Modulos Simples
**Archivos**: Modulos con pocas llamadas msg_send!
**Esfuerzo**: Bajo-Medio

| Archivo | msg_send! | class! | Blocks | Prioridad |
|---------|-----------|--------|--------|-----------|
| `model/preferences.rs` | 11 | 5 | 0 | Alta |
| `handlers/dispatcher.rs` | 3 | 0 | 0 | Alta |
| `app/helpers.rs` | 5 | 0 | 0 | Alta |
| `input/keyboard_monitors.rs` | 4 | 1 | 2 | Media |

### Fase 3: Modulos de Input
**Archivos**: Manejo de eventos
**Esfuerzo**: Medio (requiere migracion de blocks)

| Archivo | msg_send! | class! | Blocks |
|---------|-----------|--------|--------|
| `input/mouse_monitors.rs` | 10 | 1 | 6 |
| `input/observers.rs` | 9 | 4 | 3 |
| `input/hotkeys.rs` | 0 | 0 | 0 | (solo usa Carbon FFI) |

### Fase 4: UI Components
**Archivos**: Ventanas y dialogos
**Esfuerzo**: Alto (mucho codigo UI)

| Archivo | msg_send! | class! | Blocks |
|---------|-----------|--------|--------|
| `ui/status_bar.rs` | 38 | 10 | 0 |
| `ui/overlay/drawing.rs` | 23 | 0 | 0 |
| `ui/settings/window.rs` | 114 | 14 | 2 |
| `ui/dialogs/quit_dialog.rs` | 95 | 16 | 3 |
| `ui/dialogs/help_overlay.rs` | 85 | 16 | 3 |

### Fase 5: Main y CustomView Class
**Archivo**: `src/main.rs`
**Esfuerzo**: Muy Alto (clase custom con 30+ ivars)

- 70 llamadas msg_send!
- 4 usos de class!
- 2 ClassDecl (CustomViewMulti)
- ~30 instance variables
- ~20 metodos registrados

**Estrategia especial necesaria**:
1. Opcion A: Usar `ClassBuilder` (mantiene estructura similar)
2. Opcion B: Usar `define_class!` macro (mas idiomatico pero requiere reestructurar)

---

## Estrategia de Bridge (Recomendada)

Para minimizar cambios y permitir migracion gradual, crear un modulo bridge:

```rust
// src/ffi/bridge.rs
//! Compatibility layer for cocoa -> objc2 migration

pub use objc2::runtime::AnyObject;
pub use objc2::{class, msg_send, sel};

/// Type alias for backward compatibility with cocoa::base::id
pub type id = *mut AnyObject;

/// Null pointer (replaces cocoa::base::nil)
pub const nil: id = std::ptr::null_mut();

/// Boolean constants
pub const YES: bool = true;
pub const NO: bool = false;

// Re-export geometry types
pub use objc2_foundation::{CGPoint as NSPoint, CGRect as NSRect, CGSize as NSSize};
```

---

## Riesgos y Mitigaciones

### Riesgo 1: ClassBuilder vs ivars
**Problema**: objc2 maneja ivars de forma diferente (Cell<T> requerido)
**Mitigacion**: Mantener patron actual con ClassBuilder, envolver en Cell<T>

### Riesgo 2: Blocks con estado capturado
**Problema**: ConcreteBlock captura estado de forma diferente a RcBlock
**Mitigacion**: Revisar cada block individualmente, usar StackBlock donde sea posible

### Riesgo 3: Compatibilidad de msg_send!
**Problema**: Diferencias sutiles en sintaxis y tipos de retorno
**Mitigacion**: Compilar y testear frecuentemente, archivo por archivo

### Riesgo 4: NSAutoreleasePool
**Problema**: objc2 maneja autorelease pools diferente
**Mitigacion**: Usar `autoreleasepool` closure de objc2

---

## Orden de Ejecucion Recomendado

```
Fase 0: Bridge module                    [1 hora]
   |
   v
Fase 1: FFI modules (ya casi listo)      [30 min]
   |
   v
Fase 2: preferences.rs, helpers.rs       [1-2 horas]
   |
   v
Fase 2b: dispatcher.rs                   [30 min]
   |
   v
Fase 3: input/*.rs                       [2-3 horas]
   |
   v
Fase 4a: status_bar.rs, drawing.rs       [2 horas]
   |
   v
Fase 4b: dialogs/*.rs                    [3-4 horas]
   |
   v
Fase 4c: settings/window.rs              [2-3 horas]
   |
   v
Fase 5: main.rs + CustomViewMulti        [4-6 horas]
   |
   v
Cleanup: Eliminar bridge, tests          [1-2 horas]
```

**Total estimado**: 15-25 horas de trabajo

---

## Criterios de Exito por Fase

1. **Compila sin errores**
2. **cargo test pasa** (para modulos con tests)
3. **Aplicacion arranca y muestra overlay**
4. **Hotkeys funcionan**
5. **Settings window abre y guarda cambios**
6. **Multi-monitor funciona**

---

## Alternativas Consideradas

### Alternativa 1: No migrar
**Pros**: Cero esfuerzo, codigo funciona
**Contras**: Dependencia deprecated, posibles problemas futuros con nuevas versiones de Rust/macOS

### Alternativa 2: Migracion parcial (hybrid)
**Pros**: Menor esfuerzo inicial
**Contras**: Dos sistemas de FFI, mas confusion

### Alternativa 3: Reescribir con framework diferente (Tauri, etc)
**Pros**: Multiplataforma real
**Contras**: Pierde acceso a APIs nativas necesarias para overlay

**Decision**: Migracion completa a objc2 (este plan)

---

## Progreso Actual

### Fase 0: Preparacion - COMPLETADA
- [x] `Cargo.toml` - Dependencias actualizadas a objc2 ecosystem
- [x] `src/ffi/bridge.rs` - Modulo de compatibilidad creado
- [x] `src/ffi/types.rs` - Migrado a objc2
- [x] `src/ffi/cocoa_utils.rs` - Migrado a objc2
- [x] `src/ffi/mod.rs` - Actualizado con re-exports del bridge
- [x] `src/lib.rs` - Reorganizado para incluir ffi con cfg(target_os = "macos")

### Fase 1: FFI Modules - COMPLETADA
- [x] `src/ffi/carbon.rs` - Sin cambios (FFI puro)
- [x] `src/ffi/coretext.rs` - Sin cambios (FFI puro)
- [x] `src/ffi/coregraphics.rs` - Sin cambios (FFI puro)
- [x] `src/ffi/accessibility.rs` - Sin cambios (FFI puro)

### Fase 2: Modulos Simples - COMPLETADA
- [x] `src/model/preferences.rs` - Migrado a usar bridge
- [x] `src/app/helpers.rs` - Migrado a usar bridge
- [x] `src/handlers/dispatcher.rs` - Migrado a usar bridge
- [x] `src/input/hotkeys.rs` - Migrado a usar bridge

### Fase 3: Input Modules (con blocks) - COMPLETADA
- [x] `src/input/keyboard_monitors.rs` - Migrado (ConcreteBlock → RcBlock)
- [x] `src/input/mouse_monitors.rs` - Migrado (ConcreteBlock → RcBlock)
- [x] `src/input/observers.rs` - Migrado (ConcreteBlock → RcBlock)

### Fase 4: UI Modules - COMPLETADA
| Archivo | msg_send! | Blocks | Estado |
|---------|-----------|--------|--------|
| `ui/overlay/drawing.rs` | 23 | 0 | MIGRADO |
| `ui/status_bar.rs` | 38 | 0 | MIGRADO |
| `ui/dialogs/help_overlay.rs` | 85 | 3 | MIGRADO |
| `ui/dialogs/quit_dialog.rs` | 95 | 3 | MIGRADO |
| `ui/settings/window.rs` | 114 | 2 | MIGRADO |

### Fase 5: Main + CustomView - COMPLETADA
| Archivo | msg_send! | Blocks | Estado |
|---------|-----------|--------|--------|
| `main.rs` | 70 | 0 | MIGRADO (ClassDecl → ClassBuilder) |

---

## Resumen de Progreso

**Archivos migrados**: 15 de 15 ✅
**msg_send! migrados**: ~470 de ~470 (100%)
**Migracion completada**: Febrero 2026

### Notas Tecnicas

1. **Blocks**: `block::ConcreteBlock` reemplazado por `block2::RcBlock`
2. **msg_send!**: objc2 requiere comas entre argumentos
3. **Booleans**: `bool` de Rust no implementa `Encode`, usar `Bool` de objc2
4. **ivars**: Usar `load_ivar`/`store_ivar` con tipos correctos
5. **CGColorRef**: Tipo opaco con `RefEncode` para encoding correcto
6. **Retain/Release**: Ventanas y vistas requieren `retain` explicito + `mem::forget`

---

## Post-Migracion: Bugs Corregidos

1. **Circulo desaparecia**: Ventanas/vistas eran autoreleased. Fix: `retain` + `mem::forget`
2. **Tipos de retorno incorrectos**: `runModalForWindow:` → `i64`, `CGColor` → `CGColorRef`
3. **Hotkey Cmd+, conflictuaba**: Cambiado a `Ctrl+,`
4. **Multiples ventanas modales**: Guard atomico en dispatcher + drain de eventos duplicados
