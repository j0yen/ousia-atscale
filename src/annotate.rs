//! Annotation overlay emitter (AC #3).
//!
//! Emits a grounded overlay using the paper's annotation vocabulary (§4.4):
//!   - `philosophicalGrounding`  — BFO IRI + label
//!   - `domainModule`            — semantic domain (e.g. "FinancialProcess")
//!   - `aristotelianDefinition`  — genus + differentia pattern
//!
//! The overlay is a separate JSON document keyed by element name.
//! It does NOT mutate the source model file.

use crate::mapper::GroundedElement;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Paper vocabulary IRIs (§4.4). These are the canonical constants used by
// BOTH the JSON overlay emitter (this module) and the RDF exporter (rdf.rs).
// Keeping them here prevents drift between the two emitters (AC #7).
// ---------------------------------------------------------------------------

/// IRI for the `philosophicalGrounding` annotation property.
pub const PHILOSOPHICAL_GROUNDING_IRI: &str =
    "https://ousia.example/vocab#philosophicalGrounding";
/// IRI for the `domainModule` annotation property.
pub const DOMAIN_MODULE_IRI: &str = "https://ousia.example/vocab#domainModule";
/// IRI for the `aristotelianDefinition` annotation property.
pub const ARISTOTELIAN_DEFINITION_IRI: &str =
    "https://ousia.example/vocab#aristotelianDefinition";

/// Annotation for one model element (§4.4 vocabulary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAnnotation {
    /// BFO IRI for the proposed category.
    #[serde(rename = "philosophicalGrounding")]
    pub philosophical_grounding: PhilosophicalGrounding,
    /// Semantic domain inferred from element name / type.
    #[serde(rename = "domainModule")]
    pub domain_module: String,
    /// Aristotelian definition: genus + differentia.
    #[serde(rename = "aristotelianDefinition")]
    pub aristotelian_definition: AristotelianDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosophicalGrounding {
    pub iri: String,
    pub label: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AristotelianDefinition {
    /// The BFO genus (parent category label).
    pub genus: String,
    /// The differentia that specifies this element within the genus.
    pub differentia: String,
}

/// Full grounded overlay document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedOverlay {
    pub model_catalog: String,
    pub model_schema: String,
    pub model_table: String,
    pub annotations: HashMap<String, ElementAnnotation>,
    /// Source overlay version/schema marker.
    pub overlay_version: String,
}

impl GroundedOverlay {
    /// Build an overlay from grounded elements + model metadata.
    pub fn build(
        catalog: &str,
        schema: &str,
        table: &str,
        grounded: &[GroundedElement],
    ) -> Self {
        let mut annotations = HashMap::new();
        for elem in grounded {
            let ann = ElementAnnotation {
                philosophical_grounding: PhilosophicalGrounding {
                    iri: elem.bfo_iri.clone(),
                    label: elem.bfo_category.label().to_string(),
                    rationale: elem.rationale.clone(),
                },
                domain_module: infer_domain_module(&elem.name, &elem.element_type),
                aristotelian_definition: AristotelianDefinition {
                    genus: elem.bfo_category.label().to_string(),
                    differentia: format!(
                        "{} in the context of {}.{}.{}",
                        elem.name, catalog, schema, table
                    ),
                },
            };
            annotations.insert(elem.name.clone(), ann);
        }
        Self {
            model_catalog: catalog.to_string(),
            model_schema: schema.to_string(),
            model_table: table.to_string(),
            annotations,
            overlay_version: "ousia-atscale/0.1.0".to_string(),
        }
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write overlay to a file. Does NOT touch the source model.
    pub fn write_to_file(&self, path: &str) -> Result<(), crate::AtscaleError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Heuristic domain module inference from element name.
fn infer_domain_module(name: &str, _element_type: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("revenue") || n.contains("sales") || n.contains("price") || n.contains("amount") || n.contains("cost") {
        "FinancialProcess".to_string()
    } else if n.contains("customer") || n.contains("account") || n.contains("client") {
        "CustomerRole".to_string()
    } else if n.contains("product") || n.contains("item") || n.contains("sku") {
        "ProductQuality".to_string()
    } else if n.contains("date") || n.contains("time") || n.contains("period") || n.contains("year") || n.contains("month") || n.contains("day") {
        "TemporalRegion".to_string()
    } else if n.contains("region") || n.contains("country") || n.contains("city") || n.contains("location") {
        "SpatialRegion".to_string()
    } else if n.contains("count") || n.contains("qty") || n.contains("quantity") || n.contains("num") {
        "QuantitativeProcess".to_string()
    } else {
        "GeneralSemanticLayer".to_string()
    }
}

/// Helper: emit JSON from grounded elements (used by the annotate command).
pub fn emit_overlay(
    model: &crate::AtscaleModel,
    grounded: &[GroundedElement],
) -> GroundedOverlay {
    GroundedOverlay::build(
        &model.catalog,
        &model.schema,
        &model.table,
        grounded,
    )
}

/// Parse a raw describe_model JSON value into our AtscaleModel type.
/// Handles both direct model JSON and wrapped responses.
pub fn parse_describe_model_response(value: &Value) -> Result<crate::AtscaleModel, crate::AtscaleError> {
    // Try direct parse first.
    if let Ok(m) = serde_json::from_value::<crate::AtscaleModel>(value.clone()) {
        return Ok(m);
    }
    // Try unwrapping a "model" key.
    if let Some(inner) = value.get("model") {
        if let Ok(m) = serde_json::from_value::<crate::AtscaleModel>(inner.clone()) {
            return Ok(m);
        }
    }
    Err(crate::AtscaleError::InvalidModel(
        "Could not parse describe_model response".to_string(),
    ))
}
