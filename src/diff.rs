//! BFO grounding diff — compare two models element-by-element.
//!
//! The diff joins elements by name (case-insensitive) and classifies each pair:
//! - `agree`    — same name, same BFO category (both models agree)
//! - `diverge`  — same name, *different* BFO category (semantic red-flag)
//! - `only_in_a` / `only_in_b` — name present in only one model
//!
//! Exit-code contract: `DiffResult::has_divergences()` → caller should exit non-zero.

use crate::{mapper::Mapper, AtscaleModel, BfoCategory, GroundedElement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A pair that agrees (same BFO category).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgreePair {
    pub name: String,
    pub bfo_category: BfoCategory,
    pub element_type_a: String,
    pub element_type_b: String,
}

/// A pair that diverges (same name, different BFO category).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergePair {
    pub name: String,
    pub category_a: BfoCategory,
    pub category_b: BfoCategory,
    pub element_type_a: String,
    pub element_type_b: String,
    pub rationale_a: String,
    pub rationale_b: String,
}

/// An element present in only one of the two models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueElement {
    pub name: String,
    pub bfo_category: BfoCategory,
    pub element_type: String,
    pub rationale: String,
}

/// The full diff result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub agree: Vec<AgreePair>,
    pub diverge: Vec<DivergePair>,
    pub only_in_a: Vec<UniqueElement>,
    pub only_in_b: Vec<UniqueElement>,
}

impl DiffResult {
    /// True when at least one divergence was found (drives non-zero exit code).
    pub fn has_divergences(&self) -> bool {
        !self.diverge.is_empty()
    }

    /// Summary line: "N agree, M diverge, P unique (Q only-A, R only-B)"
    pub fn summary_line(&self) -> String {
        let unique = self.only_in_a.len() + self.only_in_b.len();
        format!(
            "{} agree, {} diverge, {} unique ({} only-A, {} only-B)",
            self.agree.len(),
            self.diverge.len(),
            unique,
            self.only_in_a.len(),
            self.only_in_b.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Core diff logic
// ---------------------------------------------------------------------------

/// Ground both models and diff them element-by-element.
pub fn diff_models(a: &AtscaleModel, b: &AtscaleModel) -> DiffResult {
    let mapper = Mapper::new();
    // ground_model now returns Result<Vec<GroundedElement>, AtscaleError>
    let ga = mapper.ground_model(a).unwrap_or_default();
    let gb = mapper.ground_model(b).unwrap_or_default();

    // Build lookup maps: normalised name → GroundedElement
    let map_a = build_map(&ga);
    let map_b = build_map(&gb);

    let mut agree = Vec::new();
    let mut diverge = Vec::new();
    let mut only_in_a = Vec::new();

    for (key, elem_a) in &map_a {
        match map_b.get(key) {
            Some(elem_b) => {
                if elem_a.bfo_category == elem_b.bfo_category {
                    agree.push(AgreePair {
                        name: elem_a.name.clone(),
                        bfo_category: elem_a.bfo_category.clone(),
                        element_type_a: elem_a.element_type.clone(),
                        element_type_b: elem_b.element_type.clone(),
                    });
                } else {
                    diverge.push(DivergePair {
                        name: elem_a.name.clone(),
                        category_a: elem_a.bfo_category.clone(),
                        category_b: elem_b.bfo_category.clone(),
                        element_type_a: elem_a.element_type.clone(),
                        element_type_b: elem_b.element_type.clone(),
                        rationale_a: elem_a.rationale.clone(),
                        rationale_b: elem_b.rationale.clone(),
                    });
                }
            }
            None => {
                only_in_a.push(to_unique(elem_a));
            }
        }
    }

    let only_in_b = map_b
        .iter()
        .filter(|(key, _)| !map_a.contains_key(*key))
        .map(|(_, e)| to_unique(e))
        .collect();

    // Sort for deterministic output
    agree.sort_by(|a, b| a.name.cmp(&b.name));
    diverge.sort_by(|a, b| a.name.cmp(&b.name));
    only_in_a.sort_by(|a, b| a.name.cmp(&b.name));

    let mut only_in_b: Vec<UniqueElement> = only_in_b;
    only_in_b.sort_by(|a, b| a.name.cmp(&b.name));

    DiffResult { agree, diverge, only_in_a, only_in_b }
}

fn build_map(grounded: &[GroundedElement]) -> HashMap<String, GroundedElement> {
    grounded
        .iter()
        .map(|e| (e.name.to_lowercase(), e.clone()))
        .collect()
}

fn to_unique(e: &GroundedElement) -> UniqueElement {
    UniqueElement {
        name: e.name.clone(),
        bfo_category: e.bfo_category.clone(),
        element_type: e.element_type.clone(),
        rationale: e.rationale.clone(),
    }
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

/// Print a human-readable diff to stdout.
pub fn print_text(result: &DiffResult, verbose: bool) {
    println!("{}", result.summary_line());

    if !result.diverge.is_empty() {
        println!("\nDIVERGENCES ({}):", result.diverge.len());
        println!("{}", "-".repeat(80));
        for d in &result.diverge {
            println!(
                "  {} : A={} vs B={}",
                d.name,
                d.category_a.label(),
                d.category_b.label()
            );
            println!("    A rationale: {}", d.rationale_a);
            println!("    B rationale: {}", d.rationale_b);
        }
    }

    if !result.only_in_a.is_empty() {
        println!("\nONLY IN A ({}):", result.only_in_a.len());
        for u in &result.only_in_a {
            println!("  {} [{}]", u.name, u.bfo_category.label());
        }
    }

    if !result.only_in_b.is_empty() {
        println!("\nONLY IN B ({}):", result.only_in_b.len());
        for u in &result.only_in_b {
            println!("  {} [{}]", u.name, u.bfo_category.label());
        }
    }

    if verbose && !result.agree.is_empty() {
        println!("\nAGREE ({}):", result.agree.len());
        for a in &result.agree {
            println!("  {} [{}]", a.name, a.bfo_category.label());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AtscaleModel;

    fn simple_model(columns: Vec<serde_json::Value>) -> AtscaleModel {
        let json = serde_json::json!({
            "catalog": "test",
            "schema": "s",
            "table": "t",
            "columns": columns,
            "column_groups": []
        });
        serde_json::from_value(json).unwrap()
    }

    /// AC2: identical model → all agree, 0 divergences, exit code 0.
    #[test]
    fn ac2_identical_models_all_agree() {
        let col = serde_json::json!({
            "name": "revenue", "type": "measure", "aggregation": "sum"
        });
        let m = simple_model(vec![col]);
        let result = diff_models(&m, &m);
        assert_eq!(result.diverge.len(), 0, "no divergences");
        assert_eq!(result.agree.len(), 1, "one agreement");
        assert!(result.only_in_a.is_empty());
        assert!(result.only_in_b.is_empty());
        assert!(!result.has_divergences());
    }

    /// AC3: different models show unique elements from each side.
    #[test]
    fn ac3_different_models_show_uniques() {
        let m_a = simple_model(vec![
            serde_json::json!({"name": "revenue", "type": "measure", "aggregation": "sum"}),
            serde_json::json!({"name": "region", "type": "attribute"}),
        ]);
        let m_b = simple_model(vec![
            serde_json::json!({"name": "revenue", "type": "measure", "aggregation": "sum"}),
            serde_json::json!({"name": "cost_centre", "type": "dimension"}),
        ]);
        let result = diff_models(&m_a, &m_b);
        assert_eq!(result.agree.len(), 1); // revenue
        assert_eq!(result.only_in_a.len(), 1); // region
        assert_eq!(result.only_in_b.len(), 1); // cost_centre
        // summary line present
        let s = result.summary_line();
        assert!(s.contains("agree"), "summary has agree: {s}");
        assert!(s.contains("unique"), "summary has unique: {s}");
    }

    /// AC4: crafted divergence — same name grounded differently (measure vs dimension).
    #[test]
    fn ac4_divergence_detected_exit_nonzero() {
        let m_a = simple_model(vec![
            serde_json::json!({"name": "revenue", "type": "measure", "aggregation": "sum"}),
        ]);
        let m_b = simple_model(vec![
            // Same name but a dimension → Quality, not InformationGDC
            serde_json::json!({"name": "revenue", "type": "dimension"}),
        ]);
        let result = diff_models(&m_a, &m_b);
        assert_eq!(result.diverge.len(), 1, "one divergence");
        let d = &result.diverge[0];
        assert_eq!(d.name, "revenue");
        assert_ne!(d.category_a, d.category_b);
        assert!(result.has_divergences(), "exit non-zero");
    }

    /// AC5: JSON output round-trips correctly.
    #[test]
    fn ac5_json_output_roundtrip() {
        let col = serde_json::json!({"name": "revenue", "type": "measure", "aggregation": "sum"});
        let m = simple_model(vec![col]);
        let result = diff_models(&m, &m);
        let json_str = serde_json::to_string(&result).expect("serialise");
        let back: DiffResult = serde_json::from_str(&json_str).expect("deserialise");
        assert_eq!(back.agree.len(), result.agree.len());
        assert_eq!(back.diverge.len(), result.diverge.len());
        // Check four keys present in the serialised form
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(v.get("agree").is_some(), "key 'agree'");
        assert!(v.get("diverge").is_some(), "key 'diverge'");
        assert!(v.get("only_in_a").is_some(), "key 'only_in_a'");
        assert!(v.get("only_in_b").is_some(), "key 'only_in_b'");
    }

    /// AC6: join is case-insensitive — "Revenue" and "revenue" match.
    #[test]
    fn ac6_case_insensitive_join() {
        let m_a = simple_model(vec![
            serde_json::json!({"name": "Revenue", "type": "measure", "aggregation": "sum"}),
        ]);
        let m_b = simple_model(vec![
            serde_json::json!({"name": "revenue", "type": "measure", "aggregation": "sum"}),
        ]);
        let result = diff_models(&m_a, &m_b);
        assert_eq!(result.agree.len(), 1, "case-insensitive match");
        assert_eq!(result.diverge.len(), 0);
        assert!(result.only_in_a.is_empty());
        assert!(result.only_in_b.is_empty());
    }

    /// AC7: divergence count drives exit code.
    #[test]
    fn ac7_exit_code_driven_by_divergences() {
        // No divergences → false
        let m = simple_model(vec![
            serde_json::json!({"name": "x", "type": "measure", "aggregation": "sum"}),
        ]);
        let r = diff_models(&m, &m);
        assert!(!r.has_divergences());

        // One divergence → true
        let m_a = simple_model(vec![
            serde_json::json!({"name": "x", "type": "measure", "aggregation": "sum"}),
        ]);
        let m_b = simple_model(vec![
            serde_json::json!({"name": "x", "type": "key"}),
        ]);
        let r2 = diff_models(&m_a, &m_b);
        assert!(r2.has_divergences());
    }

    /// AC4-variant: bfo_hint style — role-like name triggers Role in one, measure in another.
    #[test]
    fn ac4_role_heuristic_divergence() {
        let m_a = simple_model(vec![
            // "customer_status" as a measure (contrived) → InformationGDC
            serde_json::json!({"name": "customer_status", "type": "measure", "aggregation": "count"}),
        ]);
        let m_b = simple_model(vec![
            // "customer_status" as dimension → Role (is_role_like hit)
            serde_json::json!({"name": "customer_status", "type": "dimension"}),
        ]);
        let result = diff_models(&m_a, &m_b);
        assert_eq!(result.diverge.len(), 1);
        let d = &result.diverge[0];
        assert_eq!(d.category_a, BfoCategory::InformationGDC);
        assert_eq!(d.category_b, BfoCategory::Role);
    }
}
