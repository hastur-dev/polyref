# PolyRef

Multi-language library reference generator and code validator. PolyRef detects your project's dependencies (Rust, Python, TypeScript), generates structured reference files describing their APIs, and validates your source code against those references — catching typos, unknown imports, and incorrect function calls before they reach production.

## Motivation

AI coding assistants and large codebases share a common problem: it's easy to call functions that don't exist, misspell method names, or destructure the wrong number of values from a hook. PolyRef bridges the gap by maintaining machine-readable API reference files that can be checked against your source code automatically.

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/polyref
```

## Quick Start

```bash
# 1. Initialize config in your project
polyref init --project /path/to/project

# 2. Detect languages and dependencies
polyref detect --project /path/to/project

# 3. Generate reference files
polyref generate --project /path/to/project

# 4. Validate source code against references
polyref check --project /path/to/project

# Or run the full pipeline at once
polyref run --project /path/to/project
```

## CLI Reference

```
polyref [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `-o, --output <FORMAT>` | Output format: `terminal`, `json`, `both` (default: `terminal`) |
| `-l, --language <LANG>` | Filter to specific language |
| `--skip <LIB>` | Skip a specific library (repeatable) |
| `--no-cache` | Disable reference file caching |
| `-v, --verbose` | Verbose output |

### Commands

#### `polyref detect`

Detect languages and dependencies in a project. Outputs JSON with detected languages and all dependencies.

```bash
polyref detect --project .
```

#### `polyref generate`

Generate reference files for all detected dependencies. Creates structured files under `refs/` (or configured directory).

```bash
polyref generate --project . --verbose
```

#### `polyref check`

Validate source code against generated reference files. Exits with code 1 if errors are found.

```bash
polyref check --project . --output json
```

#### `polyref run`

Full pipeline: detect, generate, check, and report in one command.

```bash
polyref run --project . --output terminal
```

#### `polyref init`

Create a `polyref.toml` configuration file with default settings.

```bash
polyref init --project .
```

#### `polyref list-refs`

List all generated reference files grouped by language.

```bash
polyref list-refs --project .
```

## Configuration

Create a `polyref.toml` in your project root (or use `polyref init`):

```toml
# Where to store generated reference files
refs_dir = "refs"

# Libraries to skip (don't generate references for these)
skip_libraries = ["dev-only-lib", "internal-crate"]

# Output format: Terminal, Json, or Both
output_format = "Terminal"

# Whether to use cached reference files
use_cache = true

# Maximum age of cached files in hours (168 = 1 week)
cache_max_age_hours = 168
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `refs_dir` | string | `"refs"` | Directory for reference files (relative to project root) |
| `skip_libraries` | string[] | `[]` | Libraries to exclude from reference generation |
| `output_format` | string | `"Terminal"` | Default output format |
| `use_cache` | bool | `true` | Enable reference file caching |
| `cache_max_age_hours` | u64 | `168` | Cache expiration in hours |

## Reference File Format

Reference files are human-readable, language-native files that describe a library's public API. They live under `refs/<language>/lib_<name>.<ext>`.

### Rust (`refs/rust/lib_serde.rs`)

```rust
// ================================================
// Library: serde
// Version: 1.0.200
// ================================================

// TRAITS
pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}

pub trait Deserialize<'de>: Sized {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>;
}

// DERIVE MACROS
Serialize!(...)
Deserialize!(...)
```

Parsed entry kinds: `fn`, `struct`, `enum`, `trait`, `const`, `macro!(...)`, methods via `Type.method(...)` patterns.

### Python (`refs/python/lib_requests.py`)

```python
# ================================================
# Library: requests
# Version: 2.31.0
# ================================================

# FUNCTIONS
def get(url: str, **kwargs) -> Response: ...
def post(url: str, data=None, json=None, **kwargs) -> Response: ...

# CLASSES
class Response:
    status_code: int
    text: str
    def json(self) -> dict: ...
    def raise_for_status(self) -> None: ...

# CONSTANTS
DEFAULT_TIMEOUT: int = 30
```

Parsed entry kinds: `def` (function/method/property), `class`, `@decorator`, constants (`UPPER_CASE: type`), class attributes.

### TypeScript (`refs/typescript/lib_react.ts`)

```typescript
// ================================================
// Library: react
// Version: 18.2.0
// ================================================

// HOOKS
function useState<T>(initialState: T): [T, (value: T) => void];
function useEffect(effect: () => void, deps?: any[]): void;
function useRef<T>(initialValue: T): { current: T };

// COMPONENTS
function memo<T>(component: T): T;

// INTERFACES
interface ReactElement {
    type: string;
    props: Record<string, any>;
}

// TYPES
type FC<P = {}> = (props: P) => JSX.Element | null;
```

Parsed entry kinds: `function` (+ hook/component detection), `class`, `interface`, `type`, `enum`, `const`, class/interface members (methods + properties).

## Validation Rules

| Rule ID | Languages | Severity | Description |
|---------|-----------|----------|-------------|
| `unknown-import` | All | Error | Imported name not found in reference file |
| `unknown-function` | Python, TS | Error | Called function not found in reference |
| `unknown-method` | Rust, Python | Error | Method name not found (with fuzzy "did you mean?" suggestion) |
| `missing-required-arg` | Python | Error | Function called with fewer arguments than required |
| `wrong-destructure` | TypeScript | Error | Array destructuring count mismatch (e.g., `useState` must be 2) |

All rules include:
- File path and line number
- Code snippet of the offending line
- "Did you mean '...'?" suggestions via Levenshtein distance fuzzy matching

## Providing Custom Reference Files

To provide your own reference files instead of auto-generated stubs:

1. Create the reference file at the expected path:
   - Rust: `refs/rust/lib_<crate_name>.rs`
   - Python: `refs/python/lib_<package_name>.py`
   - TypeScript: `refs/typescript/lib_<package_name>.ts`

2. Follow the format shown above — PolyRef will parse your file and use it instead of generating a stub.

3. The file name uses the library name with `-` replaced by `_`. For scoped TypeScript packages like `@scope/pkg`, the name becomes `lib_scope_pkg.ts`.

PolyRef checks for existing files before generating stubs, so your custom files are always preferred.

## Claude Code Hook Setup

PolyRef integrates with Claude Code via hooks that run on session events.

### Automatic Installation

```bash
python hook/install_hooks.py
```

This copies `polyref_hook.py` to `.claude/hooks/` and configures the event handlers.

### Manual Setup

1. Copy `hook/polyref_hook.py` to your project's `.claude/hooks/` directory.

2. Add to your `.claude/hooks/config.json`:

```json
{
  "hooks": {
    "SessionStart": ["python .claude/hooks/polyref_hook.py SessionStart"],
    "PostToolUse": ["python .claude/hooks/polyref_hook.py PostToolUse"],
    "Stop": ["python .claude/hooks/polyref_hook.py Stop"]
  }
}
```

### Hook Events

| Event | Action |
|-------|--------|
| `SessionStart` | Generates reference files for the project |
| `PostToolUse` | Validates changed source files incrementally |
| `Stop` | Runs full validation and prints summary |

## Adding Support for New Languages

PolyRef is designed for extensibility via two traits:

### 1. Implement `Generator`

```rust
// src/generate/go.rs
pub struct GoGenerator;

impl Generator for GoGenerator {
    fn language(&self) -> Language {
        Language::Go // Add variant to Language enum first
    }

    fn generate(&self, dep: &Dependency, output_dir: &Path) -> anyhow::Result<ReferenceFile> {
        // Parse or generate reference file for the dependency
        // Return ReferenceFile with parsed entries
    }
}
```

### 2. Implement `Checker`

```rust
// src/check/go.rs
pub struct GoChecker;

impl Checker for GoChecker {
    fn language(&self) -> Language {
        Language::Go
    }

    fn check(
        &self,
        source_files: &[PathBuf],
        reference_files: &[ReferenceFile],
    ) -> anyhow::Result<ValidationResult> {
        // Validate source files against reference entries
        // Return ValidationResult with any issues found
    }
}
```

### 3. Add Detection

Create `src/detect/go.rs` with a function that parses the language's manifest file (e.g., `go.mod`) and returns a list of `Dependency` structs.

### 4. Wire It Up

- Add the `Language::Go` variant to `src/detect/mod.rs`
- Register the detector in `detect()`
- Register the generator/checker in `main.rs` command handlers
- Add the file extension to `find_source_files()`

## Project Structure

```
polyref/
  src/
    main.rs          # CLI (clap)
    lib.rs           # Public module exports
    config.rs        # Config loading (polyref.toml)
    detect/          # Language & dependency detection
      mod.rs         # Language enum, detect()
      rust.rs        # Cargo.toml parser
      python.rs      # requirements.txt / pyproject.toml / Pipfile
      typescript.rs  # package.json parser
    generate/        # Reference file generation
      mod.rs         # ReferenceEntry, Generator trait
      cache.rs       # Reference file caching
      templates.rs   # File header templates
      rust.rs        # Rust reference parser/generator
      python.rs      # Python reference parser/generator
      typescript.rs  # TypeScript reference parser/generator
    check/           # Source code validation
      mod.rs         # Issue, Checker trait
      common.rs      # Shared utils (fuzzy match, string helpers)
      rust.rs        # Rust checker
      python.rs      # Python checker
      typescript.rs  # TypeScript checker
    report/          # Output formatting
      mod.rs         # Reporter trait
      terminal.rs    # Colored terminal output
      json.rs        # Structured JSON output
    hook/            # Claude Code integration
      mod.rs
      orchestrator.rs
  hook/              # Python hook scripts
    polyref_hook.py
    install_hooks.py
    test_hook.py
  tests/             # 144 Rust tests + 4 Python tests
    core_types_tests.rs
    detect_tests.rs
    generate_tests.rs
    check_tests.rs
    report_tests.rs
    integration_tests.rs
    fixtures/        # Test project fixtures
```

## License

MIT
