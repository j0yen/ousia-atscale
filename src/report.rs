//! Coverage reporting (AC #4).
//!
//! Prints grounding coverage: % of model elements with a proposed BFO mapping.
//! "Grounded" means anything other than IndependentContinuant (the fallback).

use crate::mapper::{BfoCategory, GroundedElement};
use std::collections::HashMap;

/// Coverage statistics for a model.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub total: usize,
    pub grounded: usize,
    pub fallback: usize,
    pub by_category: HashMap<String, usize>,
    pub by_element_type: HashMap<String, usize>,
}

impl CoverageReport {
    pub fn build(grounded: &[GroundedElement]) -> Self {
        let total = grounded.len();
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut by_element_type: HashMap<String, usize> = HashMap::new();
        let mut fallback = 0usize;

        for elem in grounded {
            *by_category.entry(elem.bfo_category.label().to_string()).or_insert(0) += 1;
            *by_element_type.entry(elem.element_type.clone()).or_insert(0) += 1;
            if matches!(elem.bfo_category, BfoCategory::IndependentContinuant) {
                fallback += 1;
            }
        }

        let grounded_count = total - fallback;
        Self {
            total,
            grounded: grounded_count,
            fallback,
            by_category,
            by_element_type,
        }
    }

    /// Coverage percentage (0.0–100.0).
    pub fn coverage_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.grounded as f64 / self.total as f64) * 100.0
    }

    /// Print a human-readable report to stdout.
    pub fn print(&self, model_label: &str) {
        println!("=== BFO Grounding Coverage Report ===");
        println!("Model : {}", model_label);
        println!("Total elements : {}", self.total);
        println!("Grounded       : {} ({:.1}%)", self.grounded, self.coverage_pct());
        println!("Fallback (IC)  : {}", self.fallback);
        println!();
        println!("By BFO category:");
        let mut cats: Vec<_> = self.by_category.iter().collect();
        cats.sort_by_key(|(k, _)| k.as_str());
        for (cat, count) in &cats {
            println!("  {:<45} {}", cat, count);
        }
        println!();
        println!("By element type:");
        let mut types: Vec<_> = self.by_element_type.iter().collect();
        types.sort_by_key(|(k, _)| k.as_str());
        for (et, count) in &types {
            println!("  {:<20} {}", et, count);
        }
    }
}
