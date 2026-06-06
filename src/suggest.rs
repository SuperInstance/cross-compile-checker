use anyhow::Result;
use std::path::Path;

use crate::compat::{CompatibilityChecker, RiskLevel};
use crate::target_db::TargetDb;

/// A suggested CI target with reasoning.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub target: String,
    pub os: String,
    pub arch: String,
    pub popularity: u32,
    pub risk: RiskLevel,
    pub reason: String,
}

/// Suggests which targets to add to CI based on popularity and compatibility.
pub struct Suggestor {
    db: TargetDb,
    checker: CompatibilityChecker,
}

impl Suggestor {
    pub fn new() -> Self {
        Self {
            db: TargetDb::new(),
            checker: CompatibilityChecker::new(),
        }
    }

    /// Suggest up to `top_n` targets to add to CI.
    pub fn suggest(&self, cargo_path: &Path, top_n: usize) -> Result<Vec<Suggestion>> {
        let compat_results = self.checker.check(cargo_path)?;

        // Build a map of target -> compat result
        let compat_map: std::collections::HashMap<&str, &crate::compat::CompatResult> = compat_results
            .iter()
            .map(|r| (r.target.as_str(), r))
            .collect();

        // Score each target: popularity + compatibility bonus
        let mut scored: Vec<Suggestion> = self.db
            .all()
            .iter()
            .filter_map(|t| {
                let compat = compat_map.get(t.triple.as_str())?;
                let (risk, notes) = (&compat.risk, &compat.notes);

                // Skip targets that are completely incompatible
                if *risk == RiskLevel::Danger {
                    return None;
                }

                let reason = if notes.is_empty() {
                    "Fully compatible".to_string()
                } else {
                    format!("Minor: {}", notes.join(", "))
                };

                Some(Suggestion {
                    target: t.triple.clone(),
                    os: t.os.clone(),
                    arch: t.arch.clone(),
                    popularity: t.popularity,
                    risk: risk.clone(),
                    reason,
                })
            })
            .collect();

        // Sort by: Safe > Warning, then by popularity descending
        scored.sort_by(|a, b| {
            let risk_order = |r: &RiskLevel| match r {
                RiskLevel::Safe => 0,
                RiskLevel::Warning => 1,
                RiskLevel::Danger => 2,
            };
            match risk_order(&a.risk).cmp(&risk_order(&b.risk)) {
                std::cmp::Ordering::Equal => b.popularity.cmp(&a.popularity),
                other => other,
            }
        });

        // Ensure OS diversity: prefer at least one target per major OS
        let mut selected = Vec::new();
        let mut os_count: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        // First pass: take safe targets from different OSes
        for s in &scored {
            if selected.len() >= top_n {
                break;
            }
            let count = *os_count.get(&s.os).unwrap_or(&0);
            if count < 2 {
                selected.push(s.clone());
                *os_count.entry(s.os.clone()).or_insert(0) += 1;
            }
        }

        // Second pass: fill remaining slots
        for s in &scored {
            if selected.len() >= top_n {
                break;
            }
            if !selected.iter().any(|sel| sel.target == s.target) {
                selected.push(s.clone());
            }
        }

        Ok(selected)
    }
}

/// Run the suggest subcommand.
pub fn run(path: &Path, top: usize) -> Result<()> {
    let suggestor = Suggestor::new();
    let suggestions = suggestor.suggest(path, top)?;

    println!("Suggested CI targets (top {}):\n", top);
    for (i, s) in suggestions.iter().enumerate() {
        let risk_icon = match s.risk {
            RiskLevel::Safe => "✅",
            RiskLevel::Warning => "⚠️",
            RiskLevel::Danger => "❌",
        };
        println!(
            "  {}. {} {} (popularity: {}, os: {}, arch: {})",
            i + 1,
            risk_icon,
            s.target,
            s.popularity,
            s.os,
            s.arch,
        );
        println!("     → {}", s.reason);
    }

    // Generate a CI snippet
    println!("\nSuggested GitHub Actions matrix:");
    print!("    target: [");
    let targets: Vec<&str> = suggestions.iter().map(|s| s.target.as_str()).collect();
    println!("{}]", targets.join(", "));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_suggest_pure_rust() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&cargo_path)?;
        writeln!(f, r#"[package]
name = "test"
version = "0.1.0"
[dependencies]
serde = "1"
"#)?;
        let suggestor = Suggestor::new();
        let suggestions = suggestor.suggest(&cargo_path, 5)?;
        assert_eq!(suggestions.len(), 5);
        // All should be safe for pure Rust
        assert!(suggestions.iter().all(|s| s.risk == RiskLevel::Safe));
        // Should include diverse OSes
        let oses: std::collections::HashSet<&str> = suggestions.iter().map(|s| s.os.as_str()).collect();
        assert!(oses.len() >= 2, "Expected OS diversity, got {:?}", oses);
        Ok(())
    }

    #[test]
    fn test_suggest_filters_danger() -> Result<()> {
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
        let suggestor = Suggestor::new();
        let suggestions = suggestor.suggest(&cargo_path, 10)?;
        assert!(suggestions.iter().all(|s| s.risk != RiskLevel::Danger));
        Ok(())
    }

    #[test]
    fn test_suggestion_ordering() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&cargo_path)?;
        writeln!(f, r#"[package]
name = "test"
version = "0.1.0"
[dependencies]
serde = "1"
"#)?;
        let suggestor = Suggestor::new();
        let suggestions = suggestor.suggest(&cargo_path, 5)?;
        // First suggestion should be a high-popularity target
        assert!(suggestions[0].popularity >= 80);
        Ok(())
    }
}
