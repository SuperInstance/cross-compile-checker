use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Properties of a Rust target triple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    pub triple: String,
    pub os: String,
    pub arch: String,
    pub endianness: Endianness,
    pub pointer_width: u8,
    pub popularity: u32, // 0-100 heuristic
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endianness {
    Little,
    Big,
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Endianness::Little => write!(f, "little"),
            Endianness::Big => write!(f, "big"),
        }
    }
}

impl fmt::Display for TargetInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<30} os={:<12} arch={:<10} endian={:<6} ptr={}-bit  popularity={}",
            self.triple, self.os, self.arch, self.endianness, self.pointer_width, self.popularity
        )
    }
}

/// In-memory database of known Rust target triples.
pub struct TargetDb {
    targets: Vec<TargetInfo>,
}

impl TargetDb {
    /// Create a new database populated with common targets.
    pub fn new() -> Self {
        Self {
            targets: Self::builtin_targets(),
        }
    }

    /// Look up a target by its triple.
    pub fn get(&self, triple: &str) -> Option<&TargetInfo> {
        self.targets.iter().find(|t| t.triple == triple)
    }

    /// Return all targets.
    pub fn all(&self) -> &[TargetInfo] {
        &self.targets
    }

    /// Filter targets by OS and/or architecture.
    pub fn filter(&self, os: Option<&str>, arch: Option<&str>) -> Vec<&TargetInfo> {
        self.targets
            .iter()
            .filter(|t| {
                let os_match = os.map_or(true, |o| t.os.contains(o));
                let arch_match = arch.map_or(true, |a| t.arch.contains(a));
                os_match && arch_match
            })
            .collect()
    }

    /// Return the default "popular" targets for CI.
    pub fn popular_targets(&self) -> Vec<&TargetInfo> {
        let popular = [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "armv7-unknown-linux-gnueabihf",
            "wasm32-unknown-unknown",
        ];
        popular
            .iter()
            .filter_map(|t| self.get(t))
            .collect()
    }

    fn builtin_targets() -> Vec<TargetInfo> {
        vec![
            TargetInfo { triple: "x86_64-unknown-linux-gnu".into(), os: "linux".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 95 },
            TargetInfo { triple: "x86_64-unknown-linux-musl".into(), os: "linux".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 80 },
            TargetInfo { triple: "aarch64-unknown-linux-gnu".into(), os: "linux".into(), arch: "aarch64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 85 },
            TargetInfo { triple: "aarch64-unknown-linux-musl".into(), os: "linux".into(), arch: "aarch64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 70 },
            TargetInfo { triple: "armv7-unknown-linux-gnueabihf".into(), os: "linux".into(), arch: "armv7".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 55 },
            TargetInfo { triple: "arm-unknown-linux-gnueabihf".into(), os: "linux".into(), arch: "arm".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 30 },
            TargetInfo { triple: "i686-unknown-linux-gnu".into(), os: "linux".into(), arch: "x86".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 35 },
            TargetInfo { triple: "riscv64gc-unknown-linux-gnu".into(), os: "linux".into(), arch: "riscv64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 25 },
            TargetInfo { triple: "x86_64-apple-darwin".into(), os: "macos".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 80 },
            TargetInfo { triple: "aarch64-apple-darwin".into(), os: "macos".into(), arch: "aarch64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 85 },
            TargetInfo { triple: "x86_64-pc-windows-msvc".into(), os: "windows".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 80 },
            TargetInfo { triple: "x86_64-pc-windows-gnu".into(), os: "windows".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 40 },
            TargetInfo { triple: "i686-pc-windows-msvc".into(), os: "windows".into(), arch: "x86".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 30 },
            TargetInfo { triple: "aarch64-pc-windows-msvc".into(), os: "windows".into(), arch: "aarch64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 50 },
            TargetInfo { triple: "wasm32-unknown-unknown".into(), os: "unknown".into(), arch: "wasm32".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 65 },
            TargetInfo { triple: "wasm32-wasip1".into(), os: "wasi".into(), arch: "wasm32".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 30 },
            TargetInfo { triple: "x86_64-unknown-freebsd".into(), os: "freebsd".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 25 },
            TargetInfo { triple: "x86_64-unknown-netbsd".into(), os: "netbsd".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 15 },
            TargetInfo { triple: "x86_64-sun-solaris".into(), os: "solaris".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 10 },
            TargetInfo { triple: "aarch64-linux-android".into(), os: "android".into(), arch: "aarch64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 55 },
            TargetInfo { triple: "x86_64-linux-android".into(), os: "android".into(), arch: "x86_64".into(), endianness: Endianness::Little, pointer_width: 64, popularity: 40 },
            TargetInfo { triple: "thumbv7em-none-eabihf".into(), os: "none".into(), arch: "thumbv7em".into(), endianness: Endianness::Little, pointer_width: 32, popularity: 20 },
            TargetInfo { triple: "powerpc64-unknown-linux-gnu".into(), os: "linux".into(), arch: "powerpc64".into(), endianness: Endianness::Big, pointer_width: 64, popularity: 15 },
            TargetInfo { triple: "s390x-unknown-linux-gnu".into(), os: "linux".into(), arch: "s390x".into(), endianness: Endianness::Big, pointer_width: 64, popularity: 10 },
        ]
    }
}

/// List targets, optionally filtered.
pub fn list_targets(os: Option<&str>, arch: Option<&str>) -> Result<()> {
    let db = TargetDb::new();
    let targets = db.filter(os, arch);
    if targets.is_empty() {
        println!("No targets found matching filters.");
        return Ok(());
    }
    println!("Known Rust target triples ({} found):\n", targets.len());
    for t in &targets {
        println!("  {}", t);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_has_targets() {
        let db = TargetDb::new();
        assert!(!db.all().is_empty());
        assert!(db.all().len() >= 20);
    }

    #[test]
    fn test_get_known_target() {
        let db = TargetDb::new();
        let t = db.get("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.os, "linux");
        assert_eq!(t.arch, "x86_64");
        assert_eq!(t.pointer_width, 64);
        assert_eq!(t.endianness, Endianness::Little);
    }

    #[test]
    fn test_get_unknown_target() {
        let db = TargetDb::new();
        assert!(db.get("x86_64-unknown-beos").is_none());
    }

    #[test]
    fn test_filter_by_os() {
        let db = TargetDb::new();
        let linux = db.filter(Some("linux"), None);
        assert!(linux.len() >= 5);
        assert!(linux.iter().all(|t| t.os.contains("linux")));
    }

    #[test]
    fn test_filter_by_arch() {
        let db = TargetDb::new();
        let aarch64 = db.filter(None, Some("aarch64"));
        assert!(aarch64.len() >= 3);
        assert!(aarch64.iter().all(|t| t.arch.contains("aarch64")));
    }

    #[test]
    fn test_filter_combined() {
        let db = TargetDb::new();
        let results = db.filter(Some("linux"), Some("x86_64"));
        assert!(results.iter().all(|t| t.os.contains("linux") && t.arch.contains("x86_64")));
    }

    #[test]
    fn test_popular_targets() {
        let db = TargetDb::new();
        let popular = db.popular_targets();
        assert_eq!(popular.len(), 9);
        assert!(popular.iter().any(|t| t.triple == "x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn test_big_endian_targets() {
        let db = TargetDb::new();
        let big = db.all().iter().filter(|t| t.endianness == Endianness::Big).count();
        assert!(big >= 2, "Should have at least 2 big-endian targets");
    }

    #[test]
    fn test_endianness_display() {
        assert_eq!(format!("{}", Endianness::Little), "little");
        assert_eq!(format!("{}", Endianness::Big), "big");
    }
}
