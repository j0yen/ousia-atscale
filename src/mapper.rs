//! BFO category mapping rules.
//!
//! Mapping rules (AC #2) are explicit and deterministic:
//!
//! | AtScale element kind | BFO 2020 category    | Rationale |
//! |----------------------|---------------------|-----------|
//! | measure              | InformationGDC      | A measure is a generically dependent continuant that is information *about* a process (e.g. revenue is GDC about a sales process). |
//! | dimension/attribute  | Quality or Role     | A dimension member classifies or characterises an entity; qualities inhere in bearers, roles are relational. Heuristic: if the name contains "status", "type", "category" → Role; otherwise → Quality. |
//! | date/time            | TemporalRegion      | Temporal intervals and instants in BFO are temporal regions. |
//! | key                  | Quality             | Identifiers are qualities that inhere in the bearer (the entity identified). |
//! | hierarchy/level      | Role                | A hierarchical level (e.g. "Product Category") is a role played by members in a classification scheme. |
//! | set/named_set        | Quality             | Named sets collect entities sharing a quality. |
//! | unknown              | IndependentContinuant (inferred) | Fallback — the bearer of unclassified properties. |

use crate::model::{ElementKind, ModelElement};
use serde::{Deserialize, Serialize};

/// BFO 2020 upper-level categories used in the mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BfoCategory {
    /// Generically dependent continuant that carries propositional content.
    InformationGDC,
    /// Specifically dependent continuant that inheres in a bearer.
    Quality,
    /// Realizable entity — a role played by a bearer in a context.
    Role,
    /// A temporal region (interval or instant).
    TemporalRegion,
    /// Fallback for bearers of unknown properties.
    IndependentContinuant,
    /// A process — something that unfolds over time.
    Process,
    /// A disposition — a realizable that can be triggered.
    Disposition,
}

impl BfoCategory {
    /// Short IRI fragment in BFO namespace.
    pub fn bfo_iri(&self) -> &'static str {
        match self {
            Self::InformationGDC => "BFO_0000033",   // generically dependent continuant
            Self::Quality => "BFO_0000019",
            Self::Role => "BFO_0000023",
            Self::TemporalRegion => "BFO_0000008",
            Self::IndependentContinuant => "BFO_0000004",
            Self::Process => "BFO_0000015",
            Self::Disposition => "BFO_0000016",
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::InformationGDC => "information generically dependent continuant",
            Self::Quality => "quality",
            Self::Role => "role",
            Self::TemporalRegion => "temporal region",
            Self::IndependentContinuant => "independent continuant",
            Self::Process => "process",
            Self::Disposition => "disposition",
        }
    }
}

/// A model element with its proposed BFO grounding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedElement {
    pub name: String,
    pub element_type: String,
    pub bfo_category: BfoCategory,
    pub bfo_iri: String,
    pub rationale: String,
}

/// Stateless mapper — applies explicit rules to model elements.
pub struct Mapper;

impl Mapper {
    pub fn new() -> Self {
        Self
    }

    /// Propose a BFO category for a single model element.
    pub fn ground<'a>(&self, elem: &ModelElement<'a>) -> GroundedElement {
        let (cat, rationale) = self.assign(&elem.element_type, elem.name, elem.description, elem.aggregation);
        GroundedElement {
            name: elem.name.to_string(),
            element_type: elem.element_type.label().to_string(),
            bfo_iri: format!("http://purl.obolibrary.org/obo/{}", cat.bfo_iri()),
            rationale,
            bfo_category: cat,
        }
    }

    fn assign(
        &self,
        kind: &ElementKind,
        name: &str,
        _desc: Option<&str>,
        _agg: Option<&str>,
    ) -> (BfoCategory, String) {
        let name_lower = name.to_lowercase();
        match kind {
            ElementKind::Measure => (
                BfoCategory::InformationGDC,
                format!(
                    "'{}' is a measure — an information GDC that inheres in a process; \
                     it carries propositional content about the magnitude of some activity (BFO:GDC).",
                    name
                ),
            ),
            ElementKind::Date | ElementKind::Time => (
                BfoCategory::TemporalRegion,
                format!(
                    "'{}' is a date/time field — a temporal region (interval or instant) \
                     at which processes and states are located (BFO:temporal region).",
                    name
                ),
            ),
            ElementKind::Key => (
                BfoCategory::Quality,
                format!(
                    "'{}' is a key — an identifier quality that inheres in the entity it identifies \
                     (BFO:quality).",
                    name
                ),
            ),
            ElementKind::Hierarchy | ElementKind::Level => (
                BfoCategory::Role,
                format!(
                    "'{}' is a hierarchy/level — a role that members play within a \
                     classification scheme (BFO:role).",
                    name
                ),
            ),
            ElementKind::Set => (
                BfoCategory::Quality,
                format!(
                    "'{}' is a named set — a grouping of entities sharing a quality; \
                     the shared quality grounds the set (BFO:quality).",
                    name
                ),
            ),
            ElementKind::Dimension | ElementKind::Attribute => {
                // Heuristic: role-like names contain relational/status language.
                if is_role_like(&name_lower) {
                    (
                        BfoCategory::Role,
                        format!(
                            "'{}' is a dimension/attribute with relational/status semantics — \
                             it denotes a role played by entities in a social or organisational context \
                             (BFO:role).",
                            name
                        ),
                    )
                } else {
                    (
                        BfoCategory::Quality,
                        format!(
                            "'{}' is a dimension/attribute — a quality that inheres in its bearer, \
                             characterising it intrinsically (BFO:quality).",
                            name
                        ),
                    )
                }
            }
            ElementKind::Unknown => (
                BfoCategory::IndependentContinuant,
                format!(
                    "'{}' has an unrecognised element type; defaulting to independent continuant \
                     as a conservative bearer placeholder (BFO:independent continuant).",
                    name
                ),
            ),
        }
    }

    /// Ground all elements of a model.
    pub fn ground_model(&self, model: &crate::AtscaleModel) -> Vec<GroundedElement> {
        model.elements().iter().map(|e| self.ground(e)).collect()
    }
}

impl Default for Mapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Heuristic: does this name suggest a relational role rather than an intrinsic quality?
fn is_role_like(name_lower: &str) -> bool {
    let role_tokens = [
        "status", "type", "category", "role", "class", "tier",
        "segment", "group", "label", "flag", "indicator", "kind",
        "classification", "designation", "rank",
    ];
    role_tokens.iter().any(|t| name_lower.contains(t))
}
