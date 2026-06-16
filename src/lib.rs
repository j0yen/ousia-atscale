//! ousia-atscale: BFO grounding bridge for AtScale semantic layer.
//!
//! Maps AtScale model elements (measures, dimensions, column-groups) onto
//! BFO 2020 upper-level categories, and emits annotation overlays using the
//! paper's annotation vocabulary (§4.4 of *The Ontological Semantic Layer*).
//!
//! # Modules
//! - [`model`]   — AtScale model JSON types (describe_model output shape)
//! - [`mapper`]  — BFO category assignment rules
//! - [`annotate`] — Grounded overlay emitter
//! - [`rdf`]      — RDF/Turtle/OWL-XML exporter
//! - [`report`]   — Coverage statistics
//! - [`error`]    — Error types

pub mod annotate;
pub mod diff;
pub mod error;
pub mod mapper;
pub mod mcp;
pub mod model;
pub mod rdf;
pub mod report;
pub mod validate;

pub use error::AtscaleError;
pub use mapper::{BfoCategory, GroundedElement, Mapper};
pub use model::AtscaleModel;
