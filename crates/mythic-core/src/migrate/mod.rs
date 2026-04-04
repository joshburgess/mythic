//! Migration tools for converting sites from other static site generators.

pub mod convert;
pub mod eleventy;
pub mod hugo;
pub mod hugo_theme;
pub mod jekyll;

use serde::{Deserialize, Serialize};

/// Report generated after a migration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MigrationReport {
    pub files_copied: usize,
    pub files_converted: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl MigrationReport {
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }
}

impl std::fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\nMigration complete:")?;
        writeln!(f, "  Files copied:    {}", self.files_copied)?;
        writeln!(f, "  Files converted: {}", self.files_converted)?;
        if !self.warnings.is_empty() {
            writeln!(f, "\n  Warnings ({}):", self.warnings.len())?;
            for w in &self.warnings {
                writeln!(f, "    - {w}")?;
            }
        }
        if !self.errors.is_empty() {
            writeln!(f, "\n  Errors ({}):", self.errors.len())?;
            for e in &self.errors {
                writeln!(f, "    - {e}")?;
            }
        }
        Ok(())
    }
}
