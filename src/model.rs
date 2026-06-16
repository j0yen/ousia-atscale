//! AtScale model JSON types.
//!
//! These mirror the shape returned by the AtScale MCP `describe_model` tool.
//! Field names use snake_case matching the actual API responses; aliases
//! handle camelCase variants from older API versions.

use serde::{Deserialize, Serialize};

/// The top-level model returned by describe_model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtscaleModel {
    #[serde(default)]
    pub catalog: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub table: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub columns: Vec<Column>,
    /// Logical groupings (folder-level containers in the AtScale UI).
    #[serde(default)]
    pub column_groups: Vec<ColumnGroup>,
}

/// A single column/field within a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// "measure", "dimension", "attribute", "key", "date", "time"
    #[serde(rename = "type", default)]
    pub column_type: String,
    #[serde(default)]
    pub folder: Option<String>,
    /// Aggregation function for measures (sum, avg, count, …)
    #[serde(default)]
    pub aggregation: Option<String>,
    /// Optional explicit BFO category override.  When present it takes
    /// precedence over all heuristics.  Valid values (case-insensitive):
    /// "quality", "role", "information_gdc", "temporal_region", "process",
    /// "disposition", "independent_continuant".  An invalid value causes the
    /// mapper to return an error (typos fail loudly).
    #[serde(default)]
    pub bfo_hint: Option<String>,
}

/// A logical column group / hierarchy level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnGroup {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// "hierarchy", "level", "set", "named_set"
    #[serde(rename = "type", default)]
    pub group_type: String,
    #[serde(default)]
    pub columns: Vec<String>,
}

impl AtscaleModel {
    /// Parse from a JSON string (e.g. output of describe_model).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Parse from a file path.
    pub fn from_file(path: &str) -> Result<Self, crate::AtscaleError> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| crate::AtscaleError::ModelNotFound(path.to_string()))?;
        Ok(serde_json::from_str(&content)?)
    }

    /// All model elements as (name, type, description) tuples for mapping.
    pub fn elements(&self) -> Vec<ModelElement<'_>> {
        let mut elems = Vec::new();
        for col in &self.columns {
            elems.push(ModelElement {
                name: &col.name,
                element_type: ElementKind::from_kind_str(&col.column_type),
                description: col.description.as_deref(),
                aggregation: col.aggregation.as_deref(),
                folder: col.folder.as_deref(),
                bfo_hint: col.bfo_hint.as_deref(),
            });
        }
        for cg in &self.column_groups {
            elems.push(ModelElement {
                name: &cg.name,
                element_type: ElementKind::from_group_type(&cg.group_type),
                description: cg.description.as_deref(),
                aggregation: None,
                folder: None,
                bfo_hint: None,
            });
        }
        elems
    }
}

/// A unified view of any mappable element in the model.
#[derive(Debug, Clone)]
pub struct ModelElement<'a> {
    pub name: &'a str,
    pub element_type: ElementKind,
    pub description: Option<&'a str>,
    pub aggregation: Option<&'a str>,
    pub folder: Option<&'a str>,
    /// Forwarded from `Column::bfo_hint`; `None` for column-group elements.
    pub bfo_hint: Option<&'a str>,
}

/// Canonical element kind used for BFO mapping decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    Measure,
    Dimension,
    Attribute,
    Key,
    Date,
    Time,
    Hierarchy,
    Level,
    Set,
    Unknown,
}

impl ElementKind {
    pub fn from_kind_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "measure" => Self::Measure,
            "dimension" => Self::Dimension,
            "attribute" => Self::Attribute,
            "key" => Self::Key,
            "date" => Self::Date,
            "time" => Self::Time,
            _ => Self::Unknown,
        }
    }

    pub fn from_group_type(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hierarchy" => Self::Hierarchy,
            "level" => Self::Level,
            "named_set" | "set" => Self::Set,
            _ => Self::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Measure => "measure",
            Self::Dimension => "dimension",
            Self::Attribute => "attribute",
            Self::Key => "key",
            Self::Date => "date",
            Self::Time => "time",
            Self::Hierarchy => "hierarchy",
            Self::Level => "level",
            Self::Set => "set",
            Self::Unknown => "unknown",
        }
    }
}
