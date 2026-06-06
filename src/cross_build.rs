use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::target_db::TargetDb;

/// Result of running `cargo check --target <triple>`.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub target: String,
    pub success: bool,
    pub output: String,
}

/// The cross-builder runs `cargo check` for multiple targets.
pub struct CrossBuilder {
    db: TargetDb,
}

impl CrossBuilder {
    pub fn new() -> Self {
        Self { db: TargetDb::new() }
    }

    /// Check a project against multiple targets.
    /// If `targets` is None, uses popular targets.
    pub fn check(&self, project: &Path, targets: Option<&[String]>) -> Result<Vec<CheckResult>> {
        let target_list: Vec<&str> = if let Some(t) = targets {
            t.iter().map(|s| s.as_str()).collect()
        } else {
            self.db.popular_targets().iter().map(|t| t.triple.as_str()).collect()
        };

        let mut results = Vec::new();

        for triple in &target_list {
            println!("Checking {} ...", triple);
            let output = Command::new("cargo")
                .args(["check", "--target", triple])
                .current_dir(project)
                .env("RUSTFLAGS", "--cap-lints warn")
                .output()
                .with_context(|| format!("Failed to run cargo check for {}", triple))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}\n{}", stdout, stderr);
            let success = output.status.success();

            results.push(CheckResult {
                target: triple.to_string(),
                success,
                output: combined,
            });

            if success {
                println!("  ✅ {}", triple);
            } else {
                println!("  ❌ {}", triple);
            }
        }

        Ok(results)
    }
}

/// Run the check subcommand.
pub fn run(project: &Path, targets: Option<&[String]>) -> Result<()> {
    let builder = CrossBuilder::new();
    let results = builder.check(project, targets)?;

    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();

    println!("\nResults: {} passed, {} failed out of {} targets", passed, failed, results.len());

    if failed > 0 {
        println!("\nFailed targets:");
        for r in &results {
            if !r.success {
                println!("  ❌ {}", r.target);
                // Show last few lines of output
                let lines: Vec<&str> = r.output.lines().rev().take(5).collect();
                for line in lines.into_iter().rev() {
                    if !line.is_empty() {
                        println!("     {}", line);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_builder_new() {
        let _builder = CrossBuilder::new();
    }

    #[test]
    fn test_check_result_fields() {
        let result = CheckResult {
            target: "x86_64-unknown-linux-gnu".into(),
            success: true,
            output: "Checking test v0.1.0".into(),
        };
        assert!(result.success);
        assert_eq!(result.target, "x86_64-unknown-linux-gnu");
    }
}
