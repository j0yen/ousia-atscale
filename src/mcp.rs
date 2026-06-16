//! MCP server tools for ousia-atscale.
//!
//! Exposes four read-only tools over stdin/stdout JSON-RPC 2.0:
//!
//! - `ground_model`     — per-element BFO grounding (JSON array)
//! - `coverage_report`  — grounding coverage statistics (JSON object)
//! - `diff_models`      — element-by-element BFO diff of two models (JSON object)
//! - `validate_model`   — OWL 2 DL consistency check verdict (JSON object)
//!
//! All tools accept model JSON strings matching the shape returned by the AtScale
//! MCP `describe_model` tool (i.e. `AtscaleModel`). No file or network I/O is
//! performed during a tool call beyond the temporary OWL export used by
//! `validate_model` (which requires `ousia-reason` on PATH).

use mcp_core::{Tool, ToolError};
use serde_json::{json, Value};

use crate::{
    diff::diff_models,
    mapper::Mapper,
    model::AtscaleModel,
    report::CoverageReport,
    validate,
};

// ---------------------------------------------------------------------------
// ground_model
// ---------------------------------------------------------------------------

/// MCP tool: ground_model — propose a BFO category for each model element.
pub struct GroundModelTool;

impl Tool for GroundModelTool {
    fn name(&self) -> &str {
        "ground_model"
    }

    fn description(&self) -> &str {
        "Propose a BFO 2020 upper-level category for each element in an \
         AtScale model. Pass the JSON object returned by the AtScale \
         describe_model tool. Returns a JSON array of grounded elements."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "model_json": {
                    "type": "string",
                    "description": "AtScale model JSON (describe_model output) as a string."
                }
            },
            "required": ["model_json"]
        })
    }

    fn call(&self, args: &Value) -> Result<Value, ToolError> {
        let model_json = args
            .get("model_json")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("missing required argument: model_json"))?;

        let model: AtscaleModel = AtscaleModel::from_json(model_json)
            .map_err(|e| ToolError::new(format!("model parse error: {e}")))?;

        let mapper = Mapper::new();
        let grounded = mapper
            .ground_model(&model)
            .map_err(|e| ToolError::new(format!("grounding error: {e}")))?;

        serde_json::to_value(&grounded)
            .map_err(|e| ToolError::new(format!("serialise error: {e}")))
    }
}

// ---------------------------------------------------------------------------
// coverage_report
// ---------------------------------------------------------------------------

/// MCP tool: coverage_report — return grounding coverage statistics.
pub struct CoverageReportTool;

impl Tool for CoverageReportTool {
    fn name(&self) -> &str {
        "coverage_report"
    }

    fn description(&self) -> &str {
        "Return BFO grounding coverage statistics for an AtScale model. \
         Pass the JSON object returned by describe_model. Returns a JSON \
         object with total, grounded, fallback counts and by-category breakdown."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "model_json": {
                    "type": "string",
                    "description": "AtScale model JSON (describe_model output) as a string."
                }
            },
            "required": ["model_json"]
        })
    }

    fn call(&self, args: &Value) -> Result<Value, ToolError> {
        let model_json = args
            .get("model_json")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("missing required argument: model_json"))?;

        let model: AtscaleModel = AtscaleModel::from_json(model_json)
            .map_err(|e| ToolError::new(format!("model parse error: {e}")))?;

        let mapper = Mapper::new();
        let grounded = mapper
            .ground_model(&model)
            .map_err(|e| ToolError::new(format!("grounding error: {e}")))?;

        let report = CoverageReport::build(&grounded);

        // Serialize manually to a stable shape (CoverageReport contains HashMap).
        let mut by_category = serde_json::Map::new();
        let mut cats: Vec<_> = report.by_category.iter().collect();
        cats.sort_by_key(|(k, _)| k.as_str());
        for (cat, count) in cats {
            by_category.insert(cat.clone(), json!(count));
        }

        let mut by_element_type = serde_json::Map::new();
        let mut types: Vec<_> = report.by_element_type.iter().collect();
        types.sort_by_key(|(k, _)| k.as_str());
        for (et, count) in types {
            by_element_type.insert(et.clone(), json!(count));
        }

        Ok(json!({
            "total": report.total,
            "grounded": report.grounded,
            "fallback": report.fallback,
            "coverage_pct": (report.coverage_pct() * 10.0).round() / 10.0,
            "by_category": by_category,
            "by_element_type": by_element_type
        }))
    }
}

// ---------------------------------------------------------------------------
// diff_models
// ---------------------------------------------------------------------------

/// MCP tool: diff_models — compare BFO grounding of two AtScale models.
pub struct DiffModelsTool;

impl Tool for DiffModelsTool {
    fn name(&self) -> &str {
        "diff_models"
    }

    fn description(&self) -> &str {
        "Compare the BFO grounding of two AtScale models element-by-element. \
         Returns a JSON object with agree, diverge, only_in_a, and only_in_b \
         arrays. Consistent with what `ousia-atscale diff` produces."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "model_a_json": {
                    "type": "string",
                    "description": "AtScale model A JSON (describe_model output) as a string."
                },
                "model_b_json": {
                    "type": "string",
                    "description": "AtScale model B JSON (describe_model output) as a string."
                }
            },
            "required": ["model_a_json", "model_b_json"]
        })
    }

    fn call(&self, args: &Value) -> Result<Value, ToolError> {
        let model_a_json = args
            .get("model_a_json")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("missing required argument: model_a_json"))?;

        let model_b_json = args
            .get("model_b_json")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("missing required argument: model_b_json"))?;

        let model_a: AtscaleModel = AtscaleModel::from_json(model_a_json)
            .map_err(|e| ToolError::new(format!("model_a parse error: {e}")))?;

        let model_b: AtscaleModel = AtscaleModel::from_json(model_b_json)
            .map_err(|e| ToolError::new(format!("model_b parse error: {e}")))?;

        let result = diff_models(&model_a, &model_b);

        serde_json::to_value(&result)
            .map_err(|e| ToolError::new(format!("serialise error: {e}")))
    }
}

// ---------------------------------------------------------------------------
// validate_model
// ---------------------------------------------------------------------------

/// MCP tool: validate_model — OWL 2 DL consistency and profile check.
///
/// Requires `ousia-reason` to be present on PATH. Returns a `{ "verdict": "..." }`
/// object with one of: "consistent", "inconsistent — ...", "not-dl — ...".
pub struct ValidateModelTool;

impl Tool for ValidateModelTool {
    fn name(&self) -> &str {
        "validate_model"
    }

    fn description(&self) -> &str {
        "Validate OWL 2 DL profile conformance and consistency of an AtScale \
         model via ousia-reason. Requires ousia-reason on PATH. Returns \
         { \"verdict\": \"consistent\" | \"inconsistent — ...\" | \"not-dl — ...\" }."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "model_json": {
                    "type": "string",
                    "description": "AtScale model JSON (describe_model output) as a string."
                }
            },
            "required": ["model_json"]
        })
    }

    fn call(&self, args: &Value) -> Result<Value, ToolError> {
        let model_json = args
            .get("model_json")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("missing required argument: model_json"))?;

        let model: AtscaleModel = AtscaleModel::from_json(model_json)
            .map_err(|e| ToolError::new(format!("model parse error: {e}")))?;

        let verdict = validate::validate_model(&model, None)
            .map_err(|e| ToolError::new(format!("validate error: {e}")))?;

        Ok(json!({ "verdict": verdict.summary() }))
    }
}

// ---------------------------------------------------------------------------
// Public constructor: build the tool list
// ---------------------------------------------------------------------------

/// Construct the full list of MCP tools exposed by ousia-atscale.
pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(GroundModelTool),
        Box::new(CoverageReportTool),
        Box::new(DiffModelsTool),
        Box::new(ValidateModelTool),
    ]
}
