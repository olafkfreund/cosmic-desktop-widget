# Code Quality Standards & Metrics

## Overview

This document defines code quality standards, metrics, and measurement approaches for the COSMIC Desktop Widget project.

---

## Quality Metrics

### 1. Code Coverage

**Target**: ≥ 70% line coverage

**Measurement**:
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage

# View report
firefox coverage/index.html
```

**Per-Module Targets**:
- `src/config/` - 90% (highly testable)
- `src/widget/` - 80% (business logic)
- `src/wayland/` - 60% (hard to test, needs mocks)
- `src/render/` - 50% (visual, integration tests better)
- `src/main.rs` - 40% (integration heavy)

**What to Test**:
- ✅ All public functions
- ✅ Error conditions
- ✅ Edge cases
- ✅ Configuration parsing
- ✅ Widget logic
- ❌ Don't test tiny-skia internals
- ❌ Don't test Wayland protocol internals

---

### 2. Cyclomatic Complexity

**Target**: ≤ 10 per function

**Measurement**:
```bash
# Install cargo-cyclomat
cargo install cargo-cyclomat

# Check complexity
cargo cyclomat
```

**What It Measures**:
- Number of independent paths through code
- High complexity = hard to test, understand, maintain

**Refactoring Triggers**:
- **> 10**: Consider splitting function
- **> 15**: Definitely split function
- **> 20**: Critical - must refactor

**Example**:
```rust
// ❌ BAD: High complexity
fn process_event(event: Event) -> Result<()> {
    if event.is_keyboard() {
        if event.is_press() {
            if event.key() == Key::Enter {
                // nested logic...
            } else if event.key() == Key::Space {
                // more nested logic...
            }
        } else {
            // even more nesting...
        }
    } else if event.is_pointer() {
        // pointer logic...
    }
    // ... continues
}

// ✅ GOOD: Split into smaller functions
fn process_event(event: Event) -> Result<()> {
    match event {
        Event::Keyboard(k) => process_keyboard(k),
        Event::Pointer(p) => process_pointer(p),
    }
}

fn process_keyboard(k: KeyboardEvent) -> Result<()> {
    if k.is_press() {
        handle_key_press(k.key())
    } else {
        handle_key_release(k.key())
    }
}
```

---

### 3. Lines of Code (LOC)

**Targets**:
- **Function**: ≤ 50 lines
- **File**: ≤ 500 lines
- **Total Project**: Track growth, aim for maintainability

**Measurement**:
```bash
# Install tokei
cargo install tokei

# Count lines
tokei

# Or use cloc
cloc src/
```

**Current Baseline** (Initial):
```
Language   Files  Lines  Code  Comments  Blanks
Rust          5   2000   1600      200     200
```

**What to Watch**:
- Large files (> 500 lines) → Split into modules
- Large functions (> 50 lines) → Extract helper functions
- Rapid growth → May indicate duplication

---

### 4. Dependency Count

**Target**: Minimize runtime dependencies

**Current Dependencies**: 15 direct dependencies

**Guidelines**:
- ✅ Essential: wayland, smithay, tiny-skia, calloop
- ✅ Standard: tokio, serde, chrono, anyhow, thiserror
- ✅ Useful: tracing, reqwest, toml
- ⚠️ Review carefully: Image processing, font rendering
- ❌ Avoid: Large frameworks, duplicate functionality

**Measurement**:
```bash
# List dependencies
cargo tree --depth 1

# Check for duplicates
cargo tree --duplicates

# Check outdated
cargo outdated
```

**Dependency Health Checklist**:
- [ ] Well maintained (recent commits)
- [ ] Stable (> 1.0 or established < 1.0)
- [ ] Appropriate license (GPL-compatible)
- [ ] Reasonable size
- [ ] No critical vulnerabilities

---

### 5. Compilation Time

**Target**: < 2 minutes clean build, < 10 seconds incremental

**Measurement**:
```bash
# Clean build
cargo clean
time cargo build --release

# Incremental build
touch src/main.rs
time cargo build --release
```

**Optimization Strategies**:
- Use feature flags to disable unused dependencies
- Split into smaller crates if needed
- Profile compilation with `cargo build --timings`

---

### 6. Binary Size

**Target**: < 5 MB release binary (stripped)

**Measurement**:
```bash
cargo build --release
ls -lh target/release/cosmic-desktop-widget
strip target/release/cosmic-desktop-widget
ls -lh target/release/cosmic-desktop-widget
```

**Optimization**:
```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
strip = true         # Strip symbols
```

**Expected Sizes**:
- Debug build: 20-30 MB (unstripped)
- Release build: 3-5 MB (stripped)

---

### 7. Runtime Performance

**Memory Usage Targets**:
- **Idle**: < 20 MB RSS
- **Active**: < 50 MB RSS
- **Peak**: < 100 MB RSS

**CPU Usage Targets**:
- **Idle**: < 0.1% CPU
- **Updating**: < 1% CPU (brief spikes)
- **Rendering**: < 5% CPU (brief spikes)

**Measurement**:
```bash
# Memory usage
ps aux | grep cosmic-desktop-widget

# Or use heaptrack
heaptrack ./target/release/cosmic-desktop-widget

# CPU profiling
perf record -g ./target/release/cosmic-desktop-widget
perf report

# Or use flamegraph
cargo flamegraph

# Runtime metrics
RUST_LOG=debug ./cosmic-desktop-widget
# Watch for timing logs
```

**Performance Test Cases**:
1. Idle for 1 hour → memory should be stable
2. 100 widget updates → no memory growth
3. Render 1000 frames → consistent timing

---

### 8. Error Handling Quality

**Target**: 0 unwrap()/expect() in production code

**Measurement**:
```bash
# Find all unwrap/expect
rg "unwrap\(\)" src/
rg "expect\(" src/

# Should return 0 results (except in tests)
```

**Standards**:
```rust
// ❌ NEVER in production code
let value = some_option.unwrap();
let result = some_result.expect("failed");

// ✅ ALWAYS use proper error handling
let value = some_option.ok_or(Error::MissingValue)?;
let result = some_result.context("operation failed")?;

// ✅ OK in tests
#[test]
fn test_something() {
    let value = some_option.unwrap();  // OK in tests
}
```

**Error Handling Checklist**:
- [ ] All public functions return Result where fallible
- [ ] All errors have clear messages
- [ ] All errors can be traced to source
- [ ] No silent failures
- [ ] No generic "Error" types

---

### 9. Documentation Coverage

**Target**: 100% of public APIs documented

**Measurement**:
```bash
# Check missing docs
cargo doc --no-deps 2>&1 | grep warning

# Generate docs
cargo doc --no-deps --open

# Count documented items
rg "^///|^//!" src/ | wc -l
```

**Documentation Standards**:
```rust
/// Brief description of function.
///
/// Longer explanation if needed. Can be multiple paragraphs.
///
/// # Arguments
///
/// * `width` - Width in pixels
/// * `height` - Height in pixels
///
/// # Returns
///
/// Returns the created surface or an error.
///
/// # Errors
///
/// Returns `Error::InvalidSize` if width or height is 0.
///
/// # Examples
///
/// ```
/// let surface = create_surface(400, 150)?;
/// ```
pub fn create_surface(width: u32, height: u32) -> Result<Surface> {
    // ...
}
```

**What to Document**:
- ✅ All public functions
- ✅ All public structs
- ✅ All public enums
- ✅ All public traits
- ✅ Module-level docs
- ✅ Complex algorithms
- ⚠️ Private functions (if complex)

---

### 10. Clippy Warnings

**Target**: 0 warnings

**Measurement**:
```bash
# Run clippy
cargo clippy --all-targets -- -D warnings

# Or with just
just check
```

**Clippy Configuration**:
```toml
# .cargo/config.toml
[clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"

# Allow some pedantic lints
too_many_lines = "allow"
must_use_candidate = "allow"
```

**Common Fixes**:
```rust
// ❌ Clippy: use of `unwrap()`
let x = opt.unwrap();

// ✅ Fixed
let x = opt?;

// ❌ Clippy: large enum variant
enum Event {
    Simple(u32),
    Complex([u8; 1024]),  // Large!
}

// ✅ Fixed: Box large variants
enum Event {
    Simple(u32),
    Complex(Box<[u8; 1024]>),
}
```

---

## Code Quality Tools

### Essential Tools

```bash
# Install all tools
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-audit      # Security
cargo install cargo-outdated   # Dependency updates
cargo install cargo-bloat      # Binary size analysis
cargo install cargo-expand     # Macro expansion
cargo install cargo-flamegraph # Profiling
cargo install cargo-watch      # Watch for changes
```

### Continuous Integration

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Check formatting
        run: cargo fmt -- --check
      
      - name: Run clippy
        run: cargo clippy --all-targets -- -D warnings
      
      - name: Run tests
        run: cargo test
      
      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v2
```

---

## Quality Gates

### Pre-Commit Checklist

Before committing code:

```bash
# 1. Format code
cargo fmt

# 2. Run clippy
cargo clippy --all-targets -- -D warnings

# 3. Run tests
cargo test

# 4. Check for unwrap/expect
rg "unwrap\(\)|expect\(" src/ || echo "Clean!"

# 5. Build release
cargo build --release
```

**Automated Pre-Commit Hook**:
```bash
#!/bin/bash
# .git/hooks/pre-commit

cargo fmt -- --check || exit 1
cargo clippy --all-targets -- -D warnings || exit 1
cargo test || exit 1

echo "✅ Pre-commit checks passed"
```

### Pre-PR Checklist

Before opening a pull request:

- [ ] All tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy`)
- [ ] Formatted (`cargo fmt`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] No unwrap()/expect() in new code
- [ ] Code coverage ≥ 70%
- [ ] Performance impact measured
- [ ] Memory leaks checked

### Pre-Release Checklist

Before releasing a version:

- [ ] All CI checks pass
- [ ] Integration tests pass
- [ ] Manual testing complete
- [ ] Documentation reviewed
- [ ] CHANGELOG.md complete
- [ ] Version bumped
- [ ] Performance benchmarks run
- [ ] Memory profiling clean
- [ ] Binary size acceptable
- [ ] Dependencies up to date (no vulnerabilities)

---

## Metrics Dashboard

### Quick Check Script

```bash
#!/bin/bash
# quality-check.sh

echo "=== Code Quality Metrics ==="
echo

echo "Lines of Code:"
tokei src/

echo
echo "Test Coverage:"
cargo tarpaulin --out Stdout | grep "Coverage"

echo
echo "Clippy Warnings:"
cargo clippy --all-targets 2>&1 | grep "warning:" | wc -l

echo
echo "unwrap() count:"
rg "unwrap\(\)" src/ | wc -l

echo
echo "Binary Size:"
cargo build --release 2>/dev/null
ls -lh target/release/cosmic-desktop-widget | awk '{print $5}'

echo
echo "Dependencies:"
cargo tree --depth 1 | wc -l

echo
echo "Compilation Time:"
cargo clean
/usr/bin/time -f "%E" cargo build --release 2>&1 | tail -1

echo
echo "=== End Metrics ==="
```

---

## Anti-Patterns to Avoid

### 1. The God Object

```rust
// ❌ BAD: One struct does everything
struct Application {
    wayland: WaylandState,
    rendering: RenderState,
    widgets: Vec<Widget>,
    config: Config,
    // ... 50 more fields
}

impl Application {
    // ... 100 methods
}

// ✅ GOOD: Separate concerns
struct WaylandState { ... }
struct RenderPipeline { ... }
struct WidgetManager { ... }
struct Application {
    wayland: WaylandState,
    render: RenderPipeline,
    widgets: WidgetManager,
}
```

### 2. Clone Everything

```rust
// ❌ BAD: Unnecessary clones
fn process(&self, data: Vec<u8>) {
    let copy = data.clone();  // Unnecessary!
    self.internal.send(copy);
}

// ✅ GOOD: Use references
fn process(&self, data: &[u8]) {
    self.internal.send(data);
}
```

### 3. String Soup

```rust
// ❌ BAD: Strings for everything
fn set_position(&mut self, pos: &str) {
    // Now have to parse "top-right" everywhere
}

// ✅ GOOD: Use enums
enum Position {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn set_position(&mut self, pos: Position) {
    // Type-safe!
}
```

### 4. Premature Optimization

```rust
// ❌ BAD: Complex optimization that's not needed
fn calculate(n: usize) -> usize {
    // 50 lines of optimized code
    // that runs once per minute
}

// ✅ GOOD: Simple and clear
fn calculate(n: usize) -> usize {
    n * 2  // Profile first, optimize if needed
}
```

---

## Refactoring Guidelines

### When to Refactor

**Immediate** (blocking):
- Clippy errors
- Test failures
- Compilation errors
- Memory leaks
- Security issues

**Soon** (next PR):
- Cyclomatic complexity > 15
- Function > 100 lines
- File > 1000 lines
- Test coverage < 50%
- Multiple clippy warnings

**Later** (technical debt):
- Cyclomatic complexity > 10
- Duplicate code
- Unclear naming
- Missing documentation
- Minor performance issues

### How to Refactor

1. **Write tests first** - Ensure behavior doesn't change
2. **Small steps** - One change at a time
3. **Run tests** - After each change
4. **Commit often** - Each successful refactor
5. **Measure** - Verify improvements

---

## Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [The Little Book of Rust Macros](https://veykril.github.io/tlborm/)

---

**Last Updated**: 2025-01-13  
**Next Review**: After Phase 1 completion
