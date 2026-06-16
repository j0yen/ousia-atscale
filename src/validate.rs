//! OWL 2 DL profile conformance and consistency checking via `ousia-reason`.
//!
//! Grounds the model, exports it as OWL/XML to a temporary file, then invokes
//! `ousia-reason check --owl <file> --json` and parses the JSON result.
//!
//! # Verdict
//! - [`ConsistencyVerdict::Consistent`] — profile conformant, no inconsistencies.
//! - [`ConsistencyVerdict::Inconsistent`] — ABox inconsistencies detected.
//! - [`ConsistencyVerdict::NotDl`] — OWL 2 DL profile violations detected.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use serde::Deserialize;
use tempfile::NamedTempFile;

use crate::mapper::Mapper;
use crate::rdf::emit_owlxml;
use crate::AtscaleError;
use crate::AtscaleModel;

/// Result of running the reasoner over the grounded model export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyVerdict {
    /// OWL 2 DL profile conformant; no inconsistencies detected.
    Consistent,
    /// ABox inconsistencies were found.
    Inconsistent { details: String },
    /// OWL 2 DL profile violation detected.
    NotDl { construct: String },
}

impl ConsistencyVerdict {
    /// Human-readable one-line summary.
    pub fn summary(&self) -> String {
        match self {
            ConsistencyVerdict::Consistent => "consistent".to_string(),
            ConsistencyVerdict::Inconsistent { details } => {
                format!("inconsistent — {details}")
            }
            ConsistencyVerdict::NotDl { construct } => {
                format!("not-dl — {construct}")
            }
        }
    }
}

/// JSON shape returned by `ousia-reason check --json`.
#[derive(Debug, Deserialize)]
struct CheckJson {
    dl_conformant: bool,
    violations: Vec<String>,
    consistent: bool,
    inconsistencies: Vec<String>,
}

/// Locate the `ousia-reason` binary.
///
/// Resolution order:
/// 1. `explicit` path (from `--reasoner` CLI flag), if provided.
/// 2. `ousia-reason` resolved via `PATH`.
///
/// Returns an actionable error naming the `--reasoner` flag when the binary
/// cannot be found.
pub fn locate_reasoner(explicit: Option<&Path>) -> Result<PathBuf, AtscaleError> {
    if let Some(p) = explicit {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(AtscaleError::ReasonerNotFound {
            tried: p.display().to_string(),
        });
    }

    // Search PATH via `which`-style probe: try to run `ousia-reason --version`.
    match which_ousia_reason() {
        Some(path) => Ok(path),
        None => Err(AtscaleError::ReasonerNotFound {
            tried: "ousia-reason (PATH)".to_string(),
        }),
    }
}

fn which_ousia_reason() -> Option<PathBuf> {
    // Use `command -v` equivalent: attempt to resolve via `which` or fallback to
    // iterating PATH ourselves.
    if let Ok(output) = Command::new("which").arg("ousia-reason").output() {
        if output.status.success() {
            let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    None
}

/// Run `ousia-reason check --owl <owl_file> --json` and parse the verdict.
///
/// The OWL file must already exist on disk.
pub fn run_reasoner(reasoner: &Path, owl_file: &Path) -> Result<ConsistencyVerdict> {
    let output = Command::new(reasoner)
        .args(["check", "--owl"])
        .arg(owl_file)
        .arg("--json")
        .output()
        .map_err(|e| {
            AtscaleError::ReasonerFailed(format!(
                "failed to spawn {}: {e}",
                reasoner.display()
            ))
        })?;

    // ousia-reason exits 0 on success, non-zero on profile violations or errors.
    // We parse the JSON stdout regardless of exit code so we can surface details.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // If stdout is empty (e.g. parse error), surface the stderr error message.
    if stdout.trim().is_empty() {
        let msg = stderr.trim().to_string();
        return Err(AtscaleError::ReasonerFailed(if msg.is_empty() {
            format!(
                "ousia-reason exited {} with no output",
                output.status.code().unwrap_or(-1)
            )
        } else {
            msg
        })
        .into());
    }

    let parsed: CheckJson = serde_json::from_str(stdout.trim()).map_err(|e| {
        AtscaleError::ReasonerFailed(format!(
            "could not parse ousia-reason JSON output: {e}\noutput was: {stdout}"
        ))
    })?;

    Ok(verdict_from_check(&parsed))
}

/// Map a parsed `CheckJson` onto a [`ConsistencyVerdict`].
fn verdict_from_check(check: &CheckJson) -> ConsistencyVerdict {
    if !check.dl_conformant {
        let construct = check.violations.join("; ");
        return ConsistencyVerdict::NotDl {
            construct: if construct.is_empty() {
                "unknown violation".to_string()
            } else {
                construct
            },
        };
    }
    if !check.consistent {
        let details = check.inconsistencies.join("; ");
        return ConsistencyVerdict::Inconsistent {
            details: if details.is_empty() {
                "unknown inconsistency".to_string()
            } else {
                details
            },
        };
    }
    ConsistencyVerdict::Consistent
}

/// Ground `model`, export to OWL/XML in a temp file, run `ousia-reason check`,
/// clean up the temp file, and return the verdict.
///
/// `reasoner_path` is forwarded to [`locate_reasoner`].
pub fn validate_model(
    model: &AtscaleModel,
    reasoner_path: Option<&Path>,
) -> Result<ConsistencyVerdict> {
    let reasoner = locate_reasoner(reasoner_path)?;

    let mapper = Mapper::new();
    let grounded = mapper.ground_model(model)?;

    // Write OWL/XML to a named temp file so it auto-deletes on drop (AC6).
    let tmp = NamedTempFile::new().map_err(|e| {
        AtscaleError::Io(e)
    })?;
    let owl_path = tmp.path().to_path_buf();

    let xml = emit_owlxml(&model.catalog, &model.schema, &model.table, &grounded)?;
    std::fs::write(&owl_path, xml.as_bytes()).map_err(AtscaleError::Io)?;

    let verdict = run_reasoner(&reasoner, &owl_path)?;

    // `tmp` drops here, deleting the file (AC6).
    drop(tmp);

    Ok(verdict)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a CheckJson for testing verdict_from_check without spawning a process.
    fn make_check(dl_conformant: bool, violations: &[&str], consistent: bool, incons: &[&str]) -> CheckJson {
        CheckJson {
            dl_conformant,
            violations: violations.iter().map(|s| s.to_string()).collect(),
            consistent,
            inconsistencies: incons.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn verdict_consistent() {
        let c = make_check(true, &[], true, &[]);
        assert_eq!(verdict_from_check(&c), ConsistencyVerdict::Consistent);
    }

    #[test]
    fn verdict_inconsistent() {
        let c = make_check(true, &[], false, &["Fido is both Cat and Dog"]);
        match verdict_from_check(&c) {
            ConsistencyVerdict::Inconsistent { details } => {
                assert!(details.contains("Fido"), "details should include inconsistency message");
            }
            other => panic!("expected Inconsistent, got {other:?}"),
        }
    }

    #[test]
    fn verdict_not_dl() {
        let c = make_check(false, &["nominal in class expression: ObjectOneOf(...)"], true, &[]);
        match verdict_from_check(&c) {
            ConsistencyVerdict::NotDl { construct } => {
                assert!(construct.contains("nominal"), "construct should describe the violation");
            }
            other => panic!("expected NotDl, got {other:?}"),
        }
    }

    #[test]
    fn verdict_not_dl_over_inconsistent() {
        // DL violation takes priority over inconsistency in the verdict.
        let c = make_check(false, &["punning: X"], false, &["something"]);
        assert!(matches!(verdict_from_check(&c), ConsistencyVerdict::NotDl { .. }));
    }

    #[test]
    fn consistent_summary() {
        assert_eq!(ConsistencyVerdict::Consistent.summary(), "consistent");
    }

    #[test]
    fn inconsistent_summary_contains_details() {
        let v = ConsistencyVerdict::Inconsistent { details: "foo bar".to_string() };
        assert!(v.summary().contains("foo bar"));
    }
}
