use anyhow::Result;
use std::path::Path;

use crate::compat::{CompatibilityChecker, RiskLevel};
use crate::cross_build::{CheckResult, CrossBuilder};
use crate::target_db::TargetDb;

/// Output format for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Table,
    Json,
    Markdown,
}

impl ReportFormat {
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            _ => anyhow::bail!("Unknown format '{}'. Use: table, json, markdown", s),
        }
    }
}

/// Compatibility matrix entry.
#[derive(Debug, Clone)]
pub struct MatrixEntry {
    pub target: String,
    pub os: String,
    pub arch: String,
    pub compat_risk: RiskLevel,
    pub check_passed: Option<bool>,
    pub notes: Vec<String>,
}

/// Generate a compatibility matrix report.
pub struct ReportGenerator {
    db: TargetDb,
    checker: CompatibilityChecker,
    builder: CrossBuilder,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            db: TargetDb::new(),
            checker: CompatibilityChecker::new(),
            builder: CrossBuilder::new(),
        }
    }

    /// Generate the compatibility matrix.
    /// If `run_checks` is true, runs actual `cargo check` for each target.
    pub fn generate(
        &self,
        project: &Path,
        targets: Option<&[String]>,
        run_checks: bool,
    ) -> Result<Vec<MatrixEntry>> {
        let cargo_path = project.join("Cargo.toml");
        let compat_results = self.checker.check(&cargo_path)?;

        let target_filter: Option<Vec<&str>> = targets.map(|t| t.iter().map(|s| s.as_str()).collect());

        let check_results: Vec<CheckResult> = if run_checks {
            self.builder.check(project, targets)?
        } else {
            Vec::new()
        };

        let mut matrix = Vec::new();

        for cr in &compat_results {
            if let Some(ref filter) = target_filter {
                if !filter.contains(&cr.target.as_str()) {
                    continue;
                }
            }

            let target_info = self.db.get(&cr.target);
            let check_passed = check_results
                .iter()
                .find(|c| c.target == cr.target)
                .map(|c| c.success);

            matrix.push(MatrixEntry {
                target: cr.target.clone(),
                os: target_info.map(|t| t.os.clone()).unwrap_or_default(),
                arch: target_info.map(|t| t.arch.clone()).unwrap_or_default(),
                compat_risk: cr.risk.clone(),
                check_passed,
                notes: cr.notes.clone(),
            });
        }

        Ok(matrix)
    }

    /// Render the matrix as a table.
    pub fn render_table(matrix: &[MatrixEntry]) -> String {
        use comfy_table::{presets::UTF8_FULL, Cell, CellAlignment, Table};

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec![
            Cell::new("Target"),
            Cell::new("OS"),
            Cell::new("Arch"),
            Cell::new("Compat"),
            Cell::new("Check"),
            Cell::new("Notes"),
        ]);

        for entry in matrix {
            let compat = match entry.compat_risk {
                RiskLevel::Safe => "✅",
                RiskLevel::Warning => "⚠️",
                RiskLevel::Danger => "❌",
            };
            let check = match entry.check_passed {
                Some(true) => "✅",
                Some(false) => "❌",
                None => "—",
            };
            let notes = if entry.notes.is_empty() {
                String::new()
            } else {
                entry.notes.join("; ")
            };

            table.add_row(vec![
                Cell::new(&entry.target),
                Cell::new(&entry.os),
                Cell::new(&entry.arch),
                Cell::new(compat).set_alignment(CellAlignment::Center),
                Cell::new(check).set_alignment(CellAlignment::Center),
                Cell::new(notes),
            ]);
        }

        table.to_string()
    }

    /// Render the matrix as JSON.
    pub fn render_json(matrix: &[MatrixEntry]) -> Result<String> {
        let json_entries: Vec<serde_json::Value> = matrix
            .iter()
            .map(|e| {
                serde_json::json!({
                    "target": e.target,
                    "os": e.os,
                    "arch": e.arch,
                    "compatibility": match e.compat_risk {
                        RiskLevel::Safe => "safe",
                        RiskLevel::Warning => "warning",
                        RiskLevel::Danger => "danger",
                    },
                    "check_passed": e.check_passed,
                    "notes": e.notes,
                })
            })
            .collect();
        Ok(serde_json::to_string_pretty(&json_entries)?)
    }

    /// Render the matrix as Markdown.
    pub fn render_markdown(matrix: &[MatrixEntry]) -> String {
        let mut md = String::from("| Target | OS | Arch | Compat | Check | Notes |\n");
        md.push_str("|--------|-----|------|--------|-------|-------|\n");

        for entry in matrix {
            let compat = match entry.compat_risk {
                RiskLevel::Safe => "✅",
                RiskLevel::Warning => "⚠️",
                RiskLevel::Danger => "❌",
            };
            let check = match entry.check_passed {
                Some(true) => "✅",
                Some(false) => "❌",
                None => "—",
            };
            let notes = if entry.notes.is_empty() {
                String::new()
            } else {
                entry.notes.join("; ")
            };
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                entry.target, entry.os, entry.arch, compat, check, notes
            ));
        }

        md
    }
}

/// Run the report subcommand.
pub fn run(project: &Path, format: &str, targets: Option<&[String]>) -> Result<()> {
    let fmt = ReportFormat::from_str(format)?;
    let gen = ReportGenerator::new();
    let matrix = gen.generate(project, targets, false)?;

    match fmt {
        ReportFormat::Table => println!("{}", ReportGenerator::render_table(&matrix)),
        ReportFormat::Json => println!("{}", ReportGenerator::render_json(&matrix)?),
        ReportFormat::Markdown => println!("{}", ReportGenerator::render_markdown(&matrix)),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format_parsing() {
        assert_eq!(ReportFormat::from_str("table").unwrap(), ReportFormat::Table);
        assert_eq!(ReportFormat::from_str("json").unwrap(), ReportFormat::Json);
        assert_eq!(ReportFormat::from_str("markdown").unwrap(), ReportFormat::Markdown);
        assert!(ReportFormat::from_str("xml").is_err());
    }

    #[test]
    fn test_render_markdown() {
        let matrix = vec![MatrixEntry {
            target: "x86_64-unknown-linux-gnu".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            compat_risk: RiskLevel::Safe,
            check_passed: None,
            notes: vec![],
        }];
        let md = ReportGenerator::render_markdown(&matrix);
        assert!(md.contains("x86_64-unknown-linux-gnu"));
        assert!(md.contains("✅"));
    }

    #[test]
    fn test_render_json() -> Result<()> {
        let matrix = vec![MatrixEntry {
            target: "aarch64-apple-darwin".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
            compat_risk: RiskLevel::Warning,
            check_passed: Some(true),
            notes: vec!["test note".into()],
        }];
        let json = ReportGenerator::render_json(&matrix)?;
        assert!(json.contains("aarch64-apple-darwin"));
        assert!(json.contains("warning"));
        assert!(json.contains("test note"));
        Ok(())
    }

    #[test]
    fn test_render_table() {
        let matrix = vec![MatrixEntry {
            target: "test-target".into(),
            os: "test".into(),
            arch: "test".into(),
            compat_risk: RiskLevel::Danger,
            check_passed: Some(false),
            notes: vec!["broken".into()],
        }];
        let table = ReportGenerator::render_table(&matrix);
        assert!(table.contains("test-target"));
    }
}
