use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::target_db::{Endianness, TargetDb};

/// Compatibility risk level for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Warning,
    Danger,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "✅ Safe"),
            RiskLevel::Warning => write!(f, "⚠️  Warning"),
            RiskLevel::Danger => write!(f, "❌ Danger"),
        }
    }
}

/// Compatibility result for a single target.
#[derive(Debug, Clone)]
pub struct CompatResult {
    pub target: String,
    pub risk: RiskLevel,
    pub notes: Vec<String>,
}

/// Parsed Cargo.toml dependency info.
#[derive(Debug, Deserialize, Default)]
struct CargoToml {
    #[serde(default)]
    dependencies: HashMap<String, toml::Value>,
    #[serde(default)]
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: HashMap<String, toml::Value>,
    #[serde(default)]
    #[serde(rename = "build-dependencies")]
    build_dependencies: HashMap<String, toml::Value>,
    #[serde(default)]
    features: HashMap<String, Vec<String>>,
}

/// Known platform-specific crates and their associated OS/arch constraints.
struct PlatformCrate {
    name: &'static str,
    incompatible_os: Vec<&'static str>,
    notes: &'static str,
}

/// The compatibility checker.
pub struct CompatibilityChecker {
    db: TargetDb,
    platform_crates: Vec<PlatformCrate>,
}

impl CompatibilityChecker {
    pub fn new() -> Self {
        Self {
            db: TargetDb::new(),
            platform_crates: Self::known_platform_crates(),
        }
    }

    /// Check compatibility of a Cargo.toml against all known targets.
    pub fn check(&self, cargo_path: &Path) -> Result<Vec<CompatResult>> {
        let content = std::fs::read_to_string(cargo_path)
            .with_context(|| format!("Failed to read {}", cargo_path.display()))?;
        let cargo: CargoToml = toml::from_str(&content)
            .with_context(|| "Failed to parse Cargo.toml")?;

        let all_deps: Vec<&str> = cargo.dependencies.keys()
            .chain(cargo.dev_dependencies.keys())
            .chain(cargo.build_dependencies.keys())
            .map(|s| s.as_str())
            .collect();

        let uses_std_os = content.contains("std::os") || content.contains("std::os::");
        let uses_libc = all_deps.iter().any(|d| *d == "libc");

        let mut results = Vec::new();

        for target in self.db.all() {
            let mut notes = Vec::new();
            let mut risk_score = 0u32;

            // Check wasm constraints
            if target.arch.contains("wasm") {
                if uses_std_os {
                    notes.push("Uses std::os — not available on wasm".into());
                    risk_score += 3;
                }
                if uses_libc {
                    notes.push("Uses libc — not available on wasm32-unknown-unknown".into());
                    risk_score += 3;
                }
                if target.os == "unknown" {
                    // No filesystem, networking, etc.
                    for dep in &all_deps {
                        if matches!(*dep, "tokio" | "reqwest" | "hyper" | "std::fs" | "diesel" | "sqlx") {
                            notes.push(format!("{} likely won't work on wasm32-unknown-unknown", dep));
                            risk_score += 2;
                        }
                    }
                }
            }

            // Check no_std targets
            if target.os == "none" {
                for dep in &all_deps {
                    if matches!(*dep, "tokio" | "reqwest" | "serde_json" | "diesel" | "sqlx" | "hyper") {
                        notes.push(format!("{} requires std — incompatible with bare-metal", dep));
                        risk_score += 3;
                    }
                }
            }

            // Check pointer width differences
            if target.pointer_width == 32 {
                for dep in &all_deps {
                    if matches!(*dep, "num_cpus" | "sysinfo") {
                        notes.push(format!("{} may have issues on 32-bit targets", dep));
                        risk_score += 1;
                    }
                }
            }

            // Check big-endian specific issues
            if target.endianness == Endianness::Big {
                // Most crates work fine, but some bit manipulation may break
                if all_deps.iter().any(|d| *d == "byteorder" || *d == "bincode") {
                    notes.push("Endian-aware crate detected — verify big-endian behavior".into());
                    risk_score += 1;
                }
            }

            // Check known platform-specific crates
            for pc in &self.platform_crates {
                if all_deps.iter().any(|d| *d == pc.name) {
                    if pc.incompatible_os.iter().any(|o| target.os.contains(o)) {
                        // Fine
                    } else if !pc.incompatible_os.is_empty() {
                        // Crate only works on specific OS, this target might not be one
                        let target_os_matches = pc.incompatible_os.iter().any(|o| target.os.contains(o));
                        if !target_os_matches && pc.incompatible_os.len() <= 3 {
                            notes.push(format!("{} — {}", pc.name, pc.notes));
                            risk_score += 2;
                        }
                    }
                }
            }

            // Windows-specific checks
            if target.os == "windows" {
                if all_deps.iter().any(|d| *d == "nix" || *d == "libc") {
                    notes.push("nix/libc have limited Windows support".into());
                    risk_score += 1;
                }
            }

            // Android checks
            if target.os == "android" {
                if all_deps.iter().any(|d| *d == "core-foundation" || *d == "cocoa") {
                    notes.push("Apple frameworks not available on Android".into());
                    risk_score += 3;
                }
            }

            let risk = if risk_score == 0 {
                RiskLevel::Safe
            } else if risk_score <= 3 {
                RiskLevel::Warning
            } else {
                RiskLevel::Danger
            };

            results.push(CompatResult {
                target: target.triple.clone(),
                risk,
                notes,
            });
        }

        Ok(results)
    }

    fn known_platform_crates() -> Vec<PlatformCrate> {
        vec![
            PlatformCrate { name: "winapi", incompatible_os: vec!["windows"], notes: "Windows-only API bindings" },
            PlatformCrate { name: "windows", incompatible_os: vec!["windows"], notes: "Windows-only crate" },
            PlatformCrate { name: "windows-sys", incompatible_os: vec!["windows"], notes: "Windows-only sys bindings" },
            PlatformCrate { name: "cocoa", incompatible_os: vec!["macos"], notes: "macOS-only Cocoa bindings" },
            PlatformCrate { name: "core-foundation", incompatible_os: vec!["macos", "ios"], notes: "Apple platform only" },
            PlatformCrate { name: "core-graphics", incompatible_os: vec!["macos", "ios"], notes: "Apple platform only" },
            PlatformCrate { name: "nix", incompatible_os: vec!["linux", "macos", "freebsd", "netbsd", "android"], notes: "Unix-only — not available on Windows/WASM" },
            PlatformCrate { name: "x11", incompatible_os: vec!["linux", "freebsd"], notes: "X11 Linux/BSD only" },
            PlatformCrate { name: "wayland-client", incompatible_os: vec!["linux"], notes: "Linux Wayland only" },
            PlatformCrate { name: "udev", incompatible_os: vec!["linux"], notes: "Linux udev only" },
        ]
    }
}

/// Run the compat subcommand.
pub fn run(path: &Path, json: bool) -> Result<()> {
    let checker = CompatibilityChecker::new();
    let results = checker.check(path)?;

    if json {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "target": r.target,
                    "risk": format!("{}", r.risk),
                    "notes": r.notes,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        println!("Cross-compilation compatibility analysis for {}\n", path.display());
        for r in &results {
            println!("  {} {}", r.risk, r.target);
            for note in &r.notes {
                println!("      → {}", note);
            }
        }
        let safe = results.iter().filter(|r| r.risk == RiskLevel::Safe).count();
        let warn = results.iter().filter(|r| r.risk == RiskLevel::Warning).count();
        let danger = results.iter().filter(|r| r.risk == RiskLevel::Danger).count();
        println!("\nSummary: {} safe, {} warnings, {} danger (out of {} targets)", safe, warn, danger, results.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_check_pure_rust_project() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&cargo_path)?;
        writeln!(f, r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
serde_json = "1"
"#)?;
        let checker = CompatibilityChecker::new();
        let results = checker.check(&cargo_path)?;
        assert!(!results.is_empty());
        // Pure Rust deps should be safe for most targets
        let safe = results.iter().filter(|r| r.risk == RiskLevel::Safe).count();
        assert!(safe >= 10, "Expected many safe targets for pure Rust, got {}", safe);
        Ok(())
    }

    #[test]
    fn test_check_platform_specific_project() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&cargo_path)?;
        writeln!(f, r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
winapi = "0.3"
nix = "0.27"
"#)?;
        let checker = CompatibilityChecker::new();
        let results = checker.check(&cargo_path)?;
        // Linux targets should have warnings about winapi
        let linux_results: Vec<_> = results.iter().filter(|r| r.target.contains("linux")).collect();
        assert!(linux_results.iter().any(|r| r.notes.iter().any(|n| n.contains("winapi"))));
        Ok(())
    }

    #[test]
    fn test_check_wasm_compatibility() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&cargo_path)?;
        writeln!(f, r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
libc = "0.2"
"#)?;
        let checker = CompatibilityChecker::new();
        let results = checker.check(&cargo_path)?;
        let wasm = results.iter().find(|r| r.target == "wasm32-unknown-unknown").unwrap();
        assert!(wasm.risk == RiskLevel::Warning || wasm.risk == RiskLevel::Danger);
        assert!(wasm.notes.iter().any(|n| n.contains("libc")));
        Ok(())
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe != RiskLevel::Warning);
        assert!(RiskLevel::Warning != RiskLevel::Danger);
    }
}
