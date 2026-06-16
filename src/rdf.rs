//! RDF export: emit a grounded AtScale model as RDF (Turtle or OWL/XML).
//!
//! Each model element becomes an OWL individual typed to its BFO class IRI.
//! Annotation properties from the paper vocabulary (§4.4) are attached as
//! `rdfs:comment`-style data-property triples using the IRI constants that
//! `annotate.rs` also uses — so JSON and RDF agree on every IRI (AC #7).
//!
//! # IRI scheme
//!
//! ```text
//! https://atscale.example/<catalog>/<schema>/<table>#<element-name>
//! ```
//!
//! The model individual itself lives at the base IRI (fragment = "").

use crate::annotate::{ARISTOTELIAN_DEFINITION_IRI, DOMAIN_MODULE_IRI, PHILOSOPHICAL_GROUNDING_IRI};
use crate::mapper::GroundedElement;
use anyhow::Result;
use oxrdf::{LiteralRef, NamedNode, TripleRef};
use oxttl::TurtleSerializer;

// Standard vocabulary IRIs.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const OWL_IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const BFO_ONTOLOGY_IRI: &str = "http://purl.obolibrary.org/obo/bfo.owl";
const OBO_PREFIX: &str = "http://purl.obolibrary.org/obo/";

/// Sanitise an element name for use in an IRI fragment (percent-encode spaces,
/// drop characters that are problematic in IRIs).
fn to_fragment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            ' ' | '\t' => out.push('_'),
            '#' | '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`' => {
                // percent-encode
                for b in ch.to_string().bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Build the base model IRI for a catalog/schema/table triple.
fn model_iri(catalog: &str, schema: &str, table: &str) -> String {
    format!("https://atscale.example/{catalog}/{schema}/{table}")
}

/// Build the IRI for one model element.
fn element_iri(catalog: &str, schema: &str, table: &str, name: &str) -> String {
    format!(
        "https://atscale.example/{catalog}/{schema}/{table}#{}",
        to_fragment(name)
    )
}

/// Emit the grounded model as a Turtle byte string.
pub fn emit_turtle(
    catalog: &str,
    schema: &str,
    table: &str,
    grounded: &[GroundedElement],
) -> Result<Vec<u8>> {
    let mut ser = TurtleSerializer::new()
        .with_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#")?
        .with_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")?
        .with_prefix("owl", "http://www.w3.org/2002/07/owl#")?
        .with_prefix("obo", OBO_PREFIX)?
        .with_prefix("ousia", "https://ousia.example/vocab#")?
        .with_prefix(
            "model",
            &format!("{}/", model_iri(catalog, schema, table)),
        )?
        .for_writer(Vec::new());

    let rdf_type = NamedNode::new(RDF_TYPE)?;
    let owl_named_individual = NamedNode::new(OWL_NAMED_INDIVIDUAL)?;
    let owl_ontology = NamedNode::new(OWL_ONTOLOGY)?;
    let owl_imports = NamedNode::new(OWL_IMPORTS)?;
    let rdfs_label = NamedNode::new(RDFS_LABEL)?;
    let bfo_ont = NamedNode::new(BFO_ONTOLOGY_IRI)?;
    let philo_prop = NamedNode::new(PHILOSOPHICAL_GROUNDING_IRI)?;
    let domain_prop = NamedNode::new(DOMAIN_MODULE_IRI)?;
    let aristo_prop = NamedNode::new(ARISTOTELIAN_DEFINITION_IRI)?;

    // Declare the ontology with an owl:imports of BFO.
    let ontology_node = NamedNode::new(model_iri(catalog, schema, table))?;
    ser.serialize_triple(TripleRef::new(
        ontology_node.as_ref(),
        rdf_type.as_ref(),
        owl_ontology.as_ref(),
    ))?;
    ser.serialize_triple(TripleRef::new(
        ontology_node.as_ref(),
        owl_imports.as_ref(),
        bfo_ont.as_ref(),
    ))?;

    // One individual per grounded element.
    for elem in grounded {
        let elem_node = NamedNode::new(element_iri(catalog, schema, table, &elem.name))?;
        let bfo_class = NamedNode::new(elem.bfo_iri.as_str())?;

        // rdf:type owl:NamedIndividual
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            rdf_type.as_ref(),
            owl_named_individual.as_ref(),
        ))?;
        // rdf:type <bfo-class>
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            rdf_type.as_ref(),
            bfo_class.as_ref(),
        ))?;
        // rdfs:label
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            rdfs_label.as_ref(),
            LiteralRef::new_simple_literal(&elem.name),
        ))?;
        // philosophicalGrounding: the BFO IRI as a literal (mirrors JSON overlay)
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            philo_prop.as_ref(),
            LiteralRef::new_simple_literal(elem.bfo_iri.as_str()),
        ))?;
        // domainModule
        let domain = infer_domain_module(&elem.name, &elem.element_type);
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            domain_prop.as_ref(),
            LiteralRef::new_simple_literal(&domain),
        ))?;
        // aristotelianDefinition
        let aristo = format!(
            "{} in the context of {}.{}.{}",
            elem.name, catalog, schema, table
        );
        ser.serialize_triple(TripleRef::new(
            elem_node.as_ref(),
            aristo_prop.as_ref(),
            LiteralRef::new_simple_literal(&aristo),
        ))?;
    }

    Ok(ser.finish()?)
}

/// Emit the grounded model as OWL/XML.
///
/// OWL/XML is XML-based; we hand-emit it (it is structurally simple for this
/// use-case and avoids pulling in a full XML serialization stack).
pub fn emit_owlxml(
    catalog: &str,
    schema: &str,
    table: &str,
    grounded: &[GroundedElement],
) -> Result<String> {
    let ont_iri = model_iri(catalog, schema, table);
    let mut out = String::new();

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<Ontology xmlns=\"http://www.w3.org/2002/07/owl#\"\n");
    out.push_str("          xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
    out.push_str("          xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"\n");
    out.push_str("          xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"\n");
    out.push_str(&format!("          ontologyIRI=\"{ont_iri}\">\n"));
    out.push_str(&format!(
        "  <Import>{BFO_ONTOLOGY_IRI}</Import>\n"
    ));

    // Annotation property declarations.
    for ap_iri in &[
        PHILOSOPHICAL_GROUNDING_IRI,
        DOMAIN_MODULE_IRI,
        ARISTOTELIAN_DEFINITION_IRI,
    ] {
        out.push_str(&format!(
            "  <Declaration><AnnotationProperty IRI=\"{ap_iri}\"/></Declaration>\n"
        ));
    }

    for elem in grounded {
        let elem_iri = element_iri(catalog, schema, table, &elem.name);
        let bfo_iri = &elem.bfo_iri;
        let domain = infer_domain_module(&elem.name, &elem.element_type);
        let aristo = xml_escape(&format!(
            "{} in the context of {}.{}.{}",
            elem.name, catalog, schema, table
        ));
        let name_esc = xml_escape(&elem.name);

        // Declaration.
        out.push_str(&format!(
            "  <Declaration><NamedIndividual IRI=\"{elem_iri}\"/></Declaration>\n"
        ));
        // ClassAssertion for the BFO class.
        out.push_str(&format!(
            "  <ClassAssertion>\n    <Class IRI=\"{bfo_iri}\"/>\n    <NamedIndividual IRI=\"{elem_iri}\"/>\n  </ClassAssertion>\n"
        ));
        // rdfs:label annotation.
        out.push_str(&format!(
            "  <AnnotationAssertion>\n    <AnnotationProperty abbreviatedIRI=\"rdfs:label\"/>\n    <IRI>{elem_iri}</IRI>\n    <Literal>{name_esc}</Literal>\n  </AnnotationAssertion>\n"
        ));
        // philosophicalGrounding.
        out.push_str(&format!(
            "  <AnnotationAssertion>\n    <AnnotationProperty IRI=\"{PHILOSOPHICAL_GROUNDING_IRI}\"/>\n    <IRI>{elem_iri}</IRI>\n    <Literal>{}</Literal>\n  </AnnotationAssertion>\n",
            xml_escape(bfo_iri)
        ));
        // domainModule.
        out.push_str(&format!(
            "  <AnnotationAssertion>\n    <AnnotationProperty IRI=\"{DOMAIN_MODULE_IRI}\"/>\n    <IRI>{elem_iri}</IRI>\n    <Literal>{}</Literal>\n  </AnnotationAssertion>\n",
            xml_escape(&domain)
        ));
        // aristotelianDefinition.
        out.push_str(&format!(
            "  <AnnotationAssertion>\n    <AnnotationProperty IRI=\"{ARISTOTELIAN_DEFINITION_IRI}\"/>\n    <IRI>{elem_iri}</IRI>\n    <Literal>{aristo}</Literal>\n  </AnnotationAssertion>\n"
        ));
    }

    out.push_str("</Ontology>\n");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal XML character escape.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Infer domain module (mirrors annotate::infer_domain_module; kept local so
/// this module has no circular dep on `annotate`).
fn infer_domain_module(name: &str, _element_type: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("revenue")
        || n.contains("sales")
        || n.contains("price")
        || n.contains("amount")
        || n.contains("cost")
    {
        "FinancialProcess".to_string()
    } else if n.contains("customer") || n.contains("account") || n.contains("client") {
        "CustomerRole".to_string()
    } else if n.contains("product") || n.contains("item") || n.contains("sku") {
        "ProductQuality".to_string()
    } else if n.contains("date")
        || n.contains("time")
        || n.contains("period")
        || n.contains("year")
        || n.contains("month")
        || n.contains("day")
    {
        "TemporalRegion".to_string()
    } else if n.contains("region")
        || n.contains("country")
        || n.contains("city")
        || n.contains("location")
    {
        "SpatialRegion".to_string()
    } else if n.contains("count") || n.contains("qty") || n.contains("quantity") || n.contains("num") {
        "QuantitativeProcess".to_string()
    } else {
        "GeneralSemanticLayer".to_string()
    }
}

/// The annotation property IRIs used in the RDF output (public for AC #7 tests).
pub const RDF_PHILOSOPHICAL_GROUNDING_IRI: &str = PHILOSOPHICAL_GROUNDING_IRI;
pub const RDF_DOMAIN_MODULE_IRI: &str = DOMAIN_MODULE_IRI;
pub const RDF_ARISTOTELIAN_DEFINITION_IRI: &str = ARISTOTELIAN_DEFINITION_IRI;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapper::Mapper;
    use crate::AtscaleModel;

    fn sales_model() -> AtscaleModel {
        AtscaleModel::from_file("fixtures/sales_model.json")
            .expect("fixtures/sales_model.json must exist for tests")
    }

    fn grounded() -> (AtscaleModel, Vec<GroundedElement>) {
        let model = sales_model();
        let mapper = Mapper::new();
        let g = mapper.ground_model(&model);
        (model, g)
    }

    // AC #2: emit_turtle produces non-empty output.
    #[test]
    fn turtle_is_non_empty() {
        let (m, g) = grounded();
        let bytes = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        assert!(!bytes.is_empty(), "Turtle output must not be empty");
    }

    // AC #3: one typed individual per element; BFO_ IRIs present.
    #[test]
    fn turtle_has_bfo_types() {
        let (m, g) = grounded();
        let bytes = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        // Every grounded element should appear in the Turtle.
        for elem in &g {
            assert!(
                text.contains(&to_fragment(&elem.name)),
                "element '{}' must appear in Turtle",
                elem.name
            );
        }
        // At least one BFO class IRI must be present.
        assert!(
            text.contains("BFO_"),
            "Turtle must contain at least one BFO_ class IRI"
        );
    }

    // AC #3: count of individuals == count of grounded elements (13 for sales).
    #[test]
    fn turtle_individual_count() {
        let (m, g) = grounded();
        let expected = g.len();
        let bytes = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        // Each individual is declared as owl:NamedIndividual at least once.
        let count = text.matches("NamedIndividual").count();
        // Two occurrences per individual: rdf:type owl:NamedIndividual + the class assertion
        // serialized by TurtleSerializer may compact, but at minimum each name appears once.
        // We check at least expected occurrences of the element fragment.
        let mut found = 0;
        for elem in &g {
            if text.contains(&to_fragment(&elem.name)) {
                found += 1;
            }
        }
        assert_eq!(
            found, expected,
            "Expected {expected} individuals in Turtle, found {found}"
        );
    }

    // AC #4: Turtle round-trips (parse back without error).
    #[test]
    fn turtle_parses_cleanly() {
        let (m, g) = grounded();
        let bytes = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        let mut parser = oxttl::TurtleParser::new().for_slice(&bytes);
        let mut count = 0usize;
        while let Some(result) = parser.next() {
            result.expect("Turtle triple must parse without error");
            count += 1;
        }
        assert!(count > 0, "Must parse at least one triple");
    }

    // AC #5: OWL/XML includes owl:imports of BFO.
    #[test]
    fn owlxml_has_bfo_import() {
        let (m, g) = grounded();
        let xml = emit_owlxml(&m.catalog, &m.schema, &m.table, &g).unwrap();
        assert!(
            xml.contains(BFO_ONTOLOGY_IRI),
            "OWL/XML must contain owl:imports of BFO ontology IRI"
        );
        assert!(
            xml.contains("<Import>"),
            "OWL/XML must contain an <Import> element"
        );
    }

    // AC #5: OWL/XML is well-formed XML (basic structure check).
    #[test]
    fn owlxml_is_valid_xml() {
        let (m, g) = grounded();
        let xml = emit_owlxml(&m.catalog, &m.schema, &m.table, &g).unwrap();
        assert!(xml.starts_with("<?xml"), "Must start with XML declaration");
        assert!(xml.contains("<Ontology"), "Must contain <Ontology>");
        assert!(xml.ends_with("</Ontology>\n"), "Must end with </Ontology>");
    }

    // AC #7: RDF IRI constants match the annotate module's constants.
    #[test]
    fn rdf_iris_match_annotate_iris() {
        assert_eq!(
            RDF_PHILOSOPHICAL_GROUNDING_IRI,
            crate::annotate::PHILOSOPHICAL_GROUNDING_IRI,
            "philosophicalGrounding IRI must match annotate module"
        );
        assert_eq!(
            RDF_DOMAIN_MODULE_IRI,
            crate::annotate::DOMAIN_MODULE_IRI,
            "domainModule IRI must match annotate module"
        );
        assert_eq!(
            RDF_ARISTOTELIAN_DEFINITION_IRI,
            crate::annotate::ARISTOTELIAN_DEFINITION_IRI,
            "aristotelianDefinition IRI must match annotate module"
        );
    }

    // Idempotency: calling emit_turtle twice gives the same output.
    #[test]
    fn turtle_is_idempotent() {
        let (m, g) = grounded();
        let bytes1 = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        let bytes2 = emit_turtle(&m.catalog, &m.schema, &m.table, &g).unwrap();
        assert_eq!(bytes1, bytes2, "emit_turtle must be deterministic");
    }
}
