# PolyRef

Multi-language library reference generator, code validator, and enforcement system. PolyRef detects your project's dependencies (Rust, Python, TypeScript), generates structured reference files describing their APIs, and validates your source code against those references — catching hallucinated functions, unknown imports, and incorrect API calls before they reach production.

## Motivation

AI coding assistants and large codebases share a common problem: it's easy to call functions that don't exist, misspell method names, or destructure the wrong number of values from a hook. PolyRef bridges the gap by maintaining machine-readable API reference files that can be checked against your source code automatically — as a CLI tool, a CI gate, or a real-time Claude Code hook.

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

### Enforcement Mode

```bash
# Validate a single file and block on issues (CI gate)
polyref enforce --project src/main.rs --enforce --lang rust --refs ./refs --output-format json

# Validate from stdin
echo 'use tokio::runtime::Runtime;
fn main() { let rt = Runtime::new_async(); }' \
  | polyref enforce --from-stdin --enforce --lang rust --refs ./refs --output-format json

# Require 80% reference coverage
polyref enforce --project src/main.rs --enforce --require-coverage 80 --lang rust --refs ./refs

# Strict mode: block if any imported package lacks a reference file
polyref enforce --project src/main.rs --enforce --strict --lang rust --refs ./refs
```

## CLI Reference

```
polyref [OPTIONS] <COMMAND>
```

### Global Options

| Option | Default | Description |
|--------|---------|-------------|
| `-o, --output <FORMAT>` | `terminal` | Output format: `terminal`, `json`, `both` |
| `-l, --language <LANG>` | — | Filter to specific language |
| `--skip <LIB>` | — | Skip a specific library (repeatable) |
| `--no-cache` | `false` | Disable reference file caching |
| `-v, --verbose` | `false` | Verbose output |
| `--global-refs <DIR>` | — | Global directory of existing reference files (flat layout) |

### Commands

#### `polyref detect`

Detect languages and dependencies in a project. Outputs JSON with detected languages and all dependencies.

```bash
polyref detect --project .
```

Reads: `Cargo.toml` (Rust), `pyproject.toml` / `requirements.txt` / `Pipfile` (Python), `package.json` (TypeScript).

#### `polyref generate`

Generate reference files for all detected dependencies. Creates structured files under `refs/<language>/lib_<name>.<ext>`.

```bash
polyref generate --project . --verbose
```

Resolution order: user-provided file > global refs directory > docs.rs scrape (Rust) > stub fallback.

#### `polyref check`

Validate source code against generated reference files. Exits with code 1 if errors are found.

```bash
polyref check --project . --output json
```

#### `polyref run`

Full pipeline: detect → generate → check → report in one command.

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

#### `polyref enforce`

Enforcement gate for CI/CD and Claude Code hooks. Validates source code and returns a structured verdict.

```bash
polyref enforce [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--enforce` | `false` | Exit 1 on issues (hard block) |
| `--from-stdin` | `false` | Read source from stdin instead of file |
| `--strict` | `false` | Treat uncovered packages as blocking |
| `--require-coverage <PCT>` | — | Minimum coverage percentage (1–100) |
| `--output-format <FORMAT>` | `human` | `human` or `json` |
| `--lang <LANG>` | `auto` | Language hint: `rust`, `python`, `typescript`, `auto` |
| `--refs <DIR>` | — | Reference files directory |

**JSON output example:**

```json
{
  "polyref_enforce": true,
  "verdict": "Blocked",
  "issue_count": 1,
  "issues": [
    {
      "kind": "unknown-associated-fn",
      "line": 3,
      "message": "unknown associated function 'Runtime::new_async' — did you mean 'Runtime::new' (similarity: 0.84)?",
      "suggestion": "did you mean 'Runtime::new'?",
      "similarity": 0.84
    }
  ],
  "coverage_pct": 100.0,
  "instruction": "Fix the following issues:\n  - Line 3: unknown associated function ..."
}
```

The `instruction` field provides a ready-made prompt for AI regeneration.

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

# Optional: global directory of existing reference files (flat layout)
# global_refs_dir = "/home/you/references"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `refs_dir` | string | `"refs"` | Directory for reference files (relative to project root) |
| `skip_libraries` | string[] | `[]` | Libraries to exclude from reference generation |
| `output_format` | string | `"Terminal"` | Default output format |
| `use_cache` | bool | `true` | Enable reference file caching |
| `cache_max_age_hours` | u64 | `168` | Cache expiration in hours |
| `global_refs_dir` | string | — | Global reference directory (flat layout) |

## Validation Rules

| Rule ID | Languages | Severity | Description |
|---------|-----------|----------|-------------|
| `unknown-import` | All | Error | Imported name or module path not found in reference |
| `unknown-function` | Python, TS | Error | Called function not found in reference |
| `unknown-method` | Rust, Python | Error | Method name not found (with fuzzy suggestion) |
| `unknown-associated-fn` | Rust | Warning | Associated function (e.g. `Type::new_async()`) not found |
| `unknown-enum-variant` | Rust | Warning | Enum variant (e.g. `Color::Rojo`) not found |
| `too-many-args` | Rust | Error | Function called with more arguments than expected |
| `too-few-args` | Rust | Error | Function called with fewer arguments than expected |
| `missing-required-arg` | Python | Error | Function called with fewer arguments than required |
| `wrong-destructure` | TypeScript | Error | Array destructuring count mismatch (e.g., `useState` must be 2) |

All rules include file path, line number, code snippet, and "Did you mean?" suggestions via Jaro-Winkler similarity matching (threshold: 0.35).

### Detection Rate

Against a benchmark suite of 13 known-bad Rust patterns:

| Metric | Value |
|--------|-------|
| Detection rate | **85%** (11/13) |
| Minimum target | 77% (10/13) |

## Reference File Format

Reference files are human-readable, language-native files under `refs/<language>/lib_<name>.<ext>`.

### Rust (`refs/rust/lib_serde.rs`)

```rust
// Library: serde
// Version: 1.0.200

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}

pub struct Deserializer {
}

impl Deserializer {
    pub fn from_str(input: &str) -> Self
    pub fn from_reader<R: Read>(reader: R) -> Self
}

Serialize!(...)
Deserialize!(...)
```

### Python (`refs/python/lib_requests.py`)

```python
# Library: requests
# Version: 2.31.0

def get(url: str, **kwargs) -> Response: ...
def post(url: str, data=None, json=None, **kwargs) -> Response: ...

class Response:
    status_code: int
    text: str
    def json(self) -> dict: ...
    def raise_for_status(self) -> None: ...
```

### TypeScript (`refs/typescript/lib_react.ts`)

```typescript
// Library: react
// Version: 18.2.0

function useState<T>(initialState: T): [T, (value: T) => void];
function useEffect(effect: () => void, deps?: any[]): void;

interface ReactElement {
    type: string;
    props: Record<string, any>;
}

type FC<P = {}> = (props: P) => JSX.Element | null;
```

### Providing Custom References

Place your own reference files at the expected path and PolyRef will use them instead of generating stubs:

- Rust: `refs/rust/lib_<crate_name>.rs`
- Python: `refs/python/lib_<package_name>.py`
- TypeScript: `refs/typescript/lib_<package_name>.ts`

File names use the library name with `-` replaced by `_`. For scoped TypeScript packages like `@scope/pkg`, use `lib_scope_pkg.ts`.

## Enforcement System

The enforcement system transforms PolyRef from a warning tool into a multi-layer gate.

### Enforcement Pipeline

`scripts/enforce-pipeline.sh` runs up to four layers:

1. **polyref enforce** — API correctness
2. **cargo check** — compilation (Rust only)
3. **clippy** — linting (Rust only)
4. **cargo audit** — security (if installed)

Any layer failure blocks the pipeline.

### Coverage Gating

The `enforce` subcommand computes reference coverage — what percentage of your imported packages have reference files:

```bash
# Require 80% coverage
polyref enforce --project src/lib.rs --enforce --require-coverage 80 --lang rust --refs ./refs

# Strict: block any uncovered package
polyref enforce --project src/lib.rs --enforce --strict --lang rust --refs ./refs
```

Built-in crates (`std`, `core`, `alloc`) are excluded from coverage calculations.

### Post-Write Audit

`scripts/post-write-audit.sh` runs `cargo test` non-blocking after writes and logs failures to `.polyref-audit.jsonl`.

## Claude Code Integration

### One-Click Install

**Windows:** double-click `scripts/install-hook.bat`
**macOS:** double-click `scripts/install-hook.command`

Both scripts merge the enforce hook into your existing Claude Code settings without overwriting other hooks. Running them again is safe (idempotent).

### Manual Install

```bash
# Merge the enforce hook into ~/.claude/settings.json
python scripts/merge_hook.py ~/.claude/settings.json 'bash "/path/to/polyref/scripts/enforce-pipeline.sh"'
```

### Hook Events

| Event | Hook | Behavior |
|-------|------|----------|
| `PostToolUse` | enforce-pipeline.sh | Blocks writes that introduce API errors |
| `PostToolUse` | post-write-audit.sh | Non-blocking audit, logs failures |
| `SessionStart` | polyref_hook.py | Generates reference files for the project |
| `Stop` | polyref_hook.py | Runs full validation and prints summary |

### Settings Format

Project-level `.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit|MultiEdit|NotebookEdit",
        "hooks": [
          {
            "type": "command",
            "command": "bash scripts/enforce-pipeline.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

## Architecture

### Validation Pipeline

```
Source File
  → SourceContext (extract imports, infer types)
  → Select relevant reference files
  → Check imports against known exports
  → Check method calls with fuzzy matching
  → Check associated functions (Type::method)
  → Check argument counts
  → Produce Vec<Issue>
  → EnforceResult (verdict + coverage)
```

### Type Inference

PolyRef infers variable types from:

- Explicit annotations: `let x: Runtime = ...`
- Constructor calls: `let x = Runtime::new()`

This reduces false positives when checking method calls — `x.block_on()` is validated against `Runtime`'s methods, not all methods.

### Reference Parsing

Two parsers work together:

- **V1 parser** — line-by-line parsing of `pub fn`, `pub struct`, `pub enum`, `pub trait`, `use`, macros
- **V2 parser** — braced-block parsing of `impl Type { ... }`, `enum Type { ... }`, struct fields, re-exports

Results are merged with deduplication, preferring V2 entries that have richer type context.

### Parsed Entry Kinds

```
Function, Method, AssociatedFn, Class, Struct, Trait, Interface,
TypeAlias, Enum, EnumVariant, StructField, Constant, Decorator,
Macro, Hook, Component, Property, Module, ReExport
```

## Adding Support for New Languages

PolyRef is designed for extensibility. To add a new language:

1. **Detector** — `src/detect/<lang>.rs`: parse the manifest file, return `Vec<Dependency>`
2. **Generator** — `src/generate/<lang>.rs`: implement the `Generator` trait
3. **Checker** — `src/check/<lang>.rs`: implement the `Checker` trait
4. **Wire up** — add the `Language` variant, register in `main.rs`, add file extensions

## Project Structure

```
polyref/
├── src/
│   ├── main.rs                # CLI entry point (clap)
│   ├── lib.rs                 # Public module exports
│   ├── config.rs              # polyref.toml loading
│   ├── enforce.rs             # Enforcement verdicts and JSON output
│   ├── coverage.rs            # Reference coverage analysis
│   ├── source_context.rs      # Import-aware reference scoping
│   ├── type_inference.rs      # Type inference from let bindings
│   ├── associated_checker.rs  # Type::method() validation
│   ├── arg_checker.rs         # Argument count validation
│   ├── ref_parser_v2.rs       # Enhanced parser (impl blocks, enums, fields)
│   ├── detect/                # Language & dependency detection
│   │   ├── rust.rs            #   Cargo.toml parser
│   │   ├── python.rs          #   requirements.txt / pyproject.toml / Pipfile
│   │   └── typescript.rs      #   package.json parser
│   ├── generate/              # Reference file generation
│   │   ├── rust.rs            #   Rust reference generator (+ docs.rs scraper)
│   │   ├── python.rs          #   Python reference generator
│   │   ├── typescript.rs      #   TypeScript reference generator
│   │   ├── cache.rs           #   Reference file caching
│   │   ├── docsrs.rs          #   docs.rs HTML scraping
│   │   ├── docsrs_format.rs   #   Scraped data formatting
│   │   └── templates.rs       #   File header templates
│   ├── check/                 # Source code validation
│   │   ├── common.rs          #   Fuzzy matching, string helpers
│   │   ├── rust.rs            #   Rust checker
│   │   ├── python.rs          #   Python checker
│   │   └── typescript.rs      #   TypeScript checker
│   ├── report/                # Output formatting
│   │   ├── terminal.rs        #   Colored terminal output
│   │   └── json.rs            #   Structured JSON output
│   └── hook/                  # Claude Code hook orchestration
├── scripts/
│   ├── enforce-pipeline.sh    # Multi-layer enforcement gate
│   ├── post-write-audit.sh    # Non-blocking post-write audit
│   ├── install-hook.bat       # Windows installer (double-click)
│   ├── install-hook.command   # macOS installer (double-click)
│   ├── install-hook.sh        # Shell installer
│   └── merge_hook.py          # Merge hook into existing settings
├── hook/
│   └── polyref_hook.py        # Claude Code hook script
├── tests/                     # 301 Rust tests + 5 Python tests
│   ├── check_tests.rs         #   40 validation tests
│   ├── core_types_tests.rs    #   19 type/config tests
│   ├── detect_tests.rs        #   26 detection tests
│   ├── generate_tests.rs      #   43 generation tests
│   ├── report_tests.rs        #   9 output formatting tests
│   ├── integration_tests.rs   #   25 CLI end-to-end tests
│   ├── improvement_tests.rs   #   23 enhanced detection tests
│   ├── enforce_tests.rs       #   9 enforcement logic tests
│   ├── coverage_tests.rs      #   10 coverage analysis tests
│   ├── cli_enforce_tests.rs   #   8 enforce CLI tests
│   ├── integration_enforce_tests.rs  # 8 enforce end-to-end tests
│   └── fixtures/              #   Test project fixtures
├── Cargo.toml
└── polyref.toml
```

## Generation Test Configuration

Generation tests call Haiku via the Anthropic API to generate code, then run polyref on each output to measure detection rates.

**Required:** `ANTHROPIC_API_KEY` in `.env` or environment.

```bash
POLYREF_GEN_COUNT=5 pytest tests/generation/   # fast local run
POLYREF_GEN_COUNT=50 pytest tests/generation/  # high-confidence CI run
```

Tests are skipped automatically if `ANTHROPIC_API_KEY` is not set.

## License

MIT
