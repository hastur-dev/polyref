# PolyRef Reference File Format (v2)

This document describes the v2 reference file format used by PolyRef's enhanced checker.

## Entry Kinds

| Kind | Description | Example |
|------|-------------|---------|
| `Function` | Free function | `pub fn spawn(future: F) -> JoinHandle<T>` |
| `Method` | Instance method (has `&self`/`&mut self`) | `pub fn block_on(&self, future: F) -> T` |
| `AssociatedFn` | Associated function (no self) | `pub fn new() -> Runtime` |
| `Struct` | Struct definition | `pub struct Runtime` |
| `Enum` | Enum definition | `pub enum Color` |
| `EnumVariant` | Enum variant | `Red` (parent: `Color`) |
| `StructField` | Public struct field | `timeout` (parent: `Config`) |
| `Trait` | Trait definition | `pub trait Future` |
| `Macro` | Macro definition | `println!(...)` |
| `Constant` | Constant value | `pub const MAX_SIZE: usize` |
| `Module` | Module path | `task`, `runtime` |
| `ReExport` | Re-exported item | `pub use tokio::task::spawn` |
| `TypeAlias` | Type alias | `type Result<T> = ...` |

## ReferenceEntry Fields

```rust
pub struct ReferenceEntry {
    pub name: String,            // Entry name (e.g., "new", "spawn", "Red")
    pub kind: EntryKind,         // One of the kinds above
    pub signature: String,       // Full signature text
    pub description: String,     // Brief description
    pub type_context: Option<String>,  // Owning type for methods/associated fns
    pub parent: Option<String>,  // Parent type for enum variants/struct fields
    pub min_args: Option<usize>, // Minimum argument count (excluding self)
    pub max_args: Option<usize>, // Maximum argument count (excluding self)
    pub original_path: Option<String>, // Original path for re-exports
}
```

## Parsing Rules

### impl Blocks

```rust
impl Runtime {
    pub fn new() -> Runtime { }         // -> AssociatedFn, type_context="Runtime"
    pub fn block_on(&self, f: F) { }    // -> Method, type_context="Runtime"
    fn internal() { }                    // Skipped (not pub)
}
```

### Enum Variants

```rust
pub enum Color {
    Red,                                 // -> EnumVariant, parent="Color"
    Green,                               // -> EnumVariant, parent="Color"
    Blue(u8, u8, u8),                   // -> EnumVariant, parent="Color"
}
```

### Struct Fields

```rust
pub struct Config {
    pub timeout: u64,                    // -> StructField, parent="Config"
    internal: bool,                      // Skipped (not pub)
}
```

### Re-exports

```rust
pub use tokio::task::spawn;             // -> ReExport, original_path="tokio::task::spawn"
```

### Argument Count

Argument counts are derived from function signatures:
- `&self` / `&mut self` / `self` are excluded from the count
- `pub fn new() -> T` has min_args=0, max_args=0
- `pub fn spawn(&self, future: F) -> JoinHandle` has min_args=1, max_args=1

## Checker Capabilities

The v2 checker validates:

1. **Method calls** (`receiver.method()`) - fuzzy matched against all known methods
2. **Associated functions** (`Type::method()`) - checked against type-specific entries
3. **Crate-level calls** (`crate::function()`) - checked against crate reference
4. **Enum variants** (`Type::Variant`) - checked against known variants
5. **Import paths** (`use crate::module::item`) - validates both module path and item name
6. **Argument counts** - validates min/max arg count for known functions
7. **Type-aware checking** - uses type inference to scope method lookups

### Fuzzy Matching

- Algorithm: Jaro-Winkler similarity
- Threshold: 0.35 (lower = more suggestions)
- All unrecognized methods are flagged (universal flagging)
