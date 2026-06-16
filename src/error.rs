use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtscaleError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("AtScale MCP connector not attached; use --model <json> for offline grounding")]
    McpNotAttached,

    #[error("Model file not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid model structure: {0}")]
    InvalidModel(String),

    #[error(
        "Invalid bfo_hint '{hint}' on column '{column}'; valid values are: \
         quality, role, information_gdc, temporal_region, process, disposition, \
         independent_continuant"
    )]
    InvalidBfoHint { column: String, hint: String },
}
