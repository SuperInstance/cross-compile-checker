# cross-compile-checker

![License](https://img.shields.io/badge/license-MIT-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![Part of SuperInstance](https://img.shields.io/badge/part%20of-SuperInstance-blue)

CLI tool to check cross-compilation compatibility of Rust projects — analyzes `Cargo.toml` dependencies against known target triples, runs `cargo check` across platforms, and generates compatibility matrix reports.

## Overview

When you're shipping a Rust crate that needs to run on more than `x86_64-unknown-linux-gnu`, figuring out *which* targets actually work is tedious. This tool parses your `Cargo.toml`, checks every dependency against a built-in database of 50+ Rust target triples, detects platform-specific crates (winapi, cocoa, nix, x11...), flags wasm/no_std incompatibilities, and produces a report telling you exactly where your project will break and why.

Built for the SuperInstance fleet, where crates need to run on everything from ESP32 (`thumbv6m-none-eabi`) to WASM to `aarch64-apple-darwin`.

## Installation

```bash
cargo install --path .
```

Requires Rust 1.70+. No external dependencies beyond what's in `Cargo.toml`.

## Usage

### List known targets

```bash
# All targets
cross-compile-checker targets

# Filter by OS
cross-compile-checker targets --os linux

# Filter by architecture
cross-compile-checker targets --arch aarch64
```

### Check compatibility

```bash
# Analyze a Cargo.toml against all targets
cross-compile-checker compat --path ./Cargo.toml

# JSON output for scripting
cross-compile-checker compat --json
```

Output shows risk levels per target:

```
Cross-compilation compatibility analysis for ./Cargo.toml

  ✅ Safe x86_64-unknown-linux-gnu
  ⚠️  Warning wasm32-unknown-unknown
      → tokio likely won't work on wasm32-unknown-unknown
      → reqwest likely won't work on wasm32-unknown-unknown
  ❌ Danger thumbv6m-none-eabi
      → tokio requires std — incompatible with bare-metal
      → serde_json requires std — incompatible with bare-metal

Summary: 12 safe, 5 warnings, 3 danger (out of 20 targets)
```

### Run cargo check across targets

```bash
# Check popular targets
cross-compile-checker check --project ./my-crate

# Specific targets
cross-compile-checker check --targets x86_64-pc-windows-msvc,aarch64-unknown-linux-gnu,wasm32-unknown-unknown
```

### Generate a compatibility report

```bash
# Table format (default)
cross-compile-checker report --project ./my-crate

# Markdown for READMEs
cross-compile-checker report --project ./my-crate --format markdown

# JSON for CI pipelines
cross-compile-checker report --project ./my-crate --format json
```

### Suggest CI targets

```bash
# Get top 5 CI targets based on popularity + compatibility
cross-compile-checker suggest --path ./Cargo.toml --top 5
```

## Architecture

```
cross-compile-checker
├── src/main.rs            CLI entry, clap subcommands (Targets, Compat, Check, Report, Suggest)
├── src/target_db.rs       Built-in database of Rust target triples (triple, os, arch, pointer_width, endianness)
├── src/compat.rs          CompatibilityChecker: parses Cargo.toml, runs risk analysis per target
├── src/cross_build.rs     cargo check runner for multiple targets (parallel via rayon)
├── src/report.rs          Report generator: table, JSON, markdown output formats
└── src/suggest.rs         CI target suggestion engine based on popularity + compat scores
```

```
           ┌─────────────┐
           │  Cargo.toml  │
           └──────┬───────┘
                  │ parse
                  ▼
      ┌───────────────────────┐
      │  CompatibilityChecker │
      │  ┌─────────────────┐  │
      │  │   TargetDb      │  │  50+ target triples
      │  │   (os/arch/ptr) │  │  with endianness + pointer width
      │  └────────┬────────┘  │
      │           │           │
      │  ┌────────▼────────┐  │
      │  │ PlatformCrate   │  │  Known platform-specific
      │  │ rules (10+)     │  │  crates: winapi, nix, cocoa...
      │  └────────┬────────┘  │
      │           │           │
      │  ┌────────▼────────┐  │
      │  │ Risk scoring    │  │  Safe (0) / Warning (1-3) / Danger (4+)
      │  └─────────────────┘  │
      └───────────┬───────────┘
                  │
          ┌───────▼────────┐
          │    Reporter     │  table / json / markdown
          └────────────────┘
```

## API Reference

### `compat::CompatibilityChecker`

```rust
pub struct CompatibilityChecker { /* ... */ }

impl CompatibilityChecker {
    pub fn new() -> Self;
    pub fn check(&self, cargo_path: &Path) -> Result<Vec<CompatResult>>;
}
```

Returns a `Vec<CompatResult>` with `target`, `risk: RiskLevel`, and `notes: Vec<String>`.

### `compat::RiskLevel`

```rust
pub enum RiskLevel {
    Safe,      // risk_score == 0
    Warning,   // risk_score 1–3
    Danger,    // risk_score >= 4
}
```

### Detection rules

The checker applies these rules per target:

| Rule | Condition | Score |
|------|-----------|-------|
| WASM std::os | target contains `wasm` + source uses `std::os` | +3 |
| WASM libc | wasm32 + depends on `libc` | +3 |
| WASM networking | wasm32-unknown-unknown + tokio/reqwest/hyper/diesel/sqlx | +2 |
| no_std | `os == "none"` + tokio/serde_json/diesel/sqlx/hyper | +3 |
| 32-bit | pointer_width == 32 + num_cpus/sysinfo | +1 |
| Big-endian | endianness == Big + byteorder/bincode | +1 |
| Platform crate | depends on winapi/cocoa/nix/x11 on incompatible OS | +2 |
| Windows Unix | windows target + nix/libc | +1 |
| Android Apple | android + core-foundation/cocoa | +3 |

## Related Crates

- **dep-audit** — vulnerability, outdated, and health scoring for Rust crates
- **fleet-dedup** — detect duplicate repos across a fleet
- **ternary-compiler** — ternary-aware compilation targets
- **open-parallel** — cross-platform parallel scheduling

## License

MIT
