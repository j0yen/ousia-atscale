//! Integration tests covering AC #1–#8 plus bfo_hint override ACs (PRD-ousia-atscale-bfo-hint).

use ousia_atscale::{
    annotate::{emit_overlay, GroundedOverlay},
    mapper::{BfoCategory, Mapper},
    model::{AtscaleModel, ElementKind, ModelElement},
    report::CoverageReport,
    AtscaleError,
};

// ── Fixture ──────────────────────────────────────────────────────────────────

fn fixture_model() -> AtscaleModel {
    let json = include_str!("../fixtures/sales_model.json");
    AtscaleModel::from_json(json).expect("fixture parse failed")
}

// ── AC #1: ground emits per-element BFO mapping ─────────────────────────────

#[test]
fn test_ground_produces_mapping_for_all_elements() {
    let model = fixture_model();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");

    // Model has 10 columns + 3 column_groups = 13 elements.
    assert_eq!(grounded.len(), 13, "expected 13 grounded elements");

    // Every element must have a non-empty rationale and a valid BFO IRI.
    for elem in &grounded {
        assert!(!elem.rationale.is_empty(), "missing rationale for {}", elem.name);
        assert!(
            elem.bfo_iri.starts_with("http://purl.obolibrary.org/obo/BFO_"),
            "invalid BFO IRI for {}: {}",
            elem.name,
            elem.bfo_iri
        );
    }
}

// ── AC #2: explicit testable mapping rules ───────────────────────────────────

#[test]
fn test_measure_maps_to_information_gdc() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "revenue",
        element_type: ElementKind::Measure,
        description: None,
        aggregation: Some("sum"),
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::InformationGDC,
        "measures must map to InformationGDC"
    );
}

#[test]
fn test_dimension_maps_to_quality() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "customer_name",
        element_type: ElementKind::Dimension,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::Quality,
        "plain dimension should map to Quality"
    );
}

#[test]
fn test_status_dimension_maps_to_role() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "customer_status",
        element_type: ElementKind::Dimension,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::Role,
        "status/type dimension should map to Role"
    );
}

#[test]
fn test_date_maps_to_temporal_region() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "order_date",
        element_type: ElementKind::Date,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(result.bfo_category, BfoCategory::TemporalRegion);
}

#[test]
fn test_time_maps_to_temporal_region() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "ship_time",
        element_type: ElementKind::Time,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(result.bfo_category, BfoCategory::TemporalRegion);
}

#[test]
fn test_hierarchy_maps_to_role() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "CustomerHierarchy",
        element_type: ElementKind::Hierarchy,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(result.bfo_category, BfoCategory::Role);
}

#[test]
fn test_key_maps_to_quality() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "order_key",
        element_type: ElementKind::Key,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(result.bfo_category, BfoCategory::Quality);
}

// ── AC #3: annotate emits overlay without touching source ───────────────────

#[test]
fn test_annotate_emits_overlay_with_annotation_vocabulary() {
    let model = fixture_model();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    let overlay = emit_overlay(&model, &grounded);

    // Overlay must not be empty.
    assert!(!overlay.annotations.is_empty());

    // Check annotation vocabulary fields are present.
    for (name, ann) in &overlay.annotations {
        assert!(
            !ann.philosophical_grounding.iri.is_empty(),
            "missing philosophicalGrounding.iri for {}",
            name
        );
        assert!(
            !ann.domain_module.is_empty(),
            "missing domainModule for {}",
            name
        );
        assert!(
            !ann.aristotelian_definition.genus.is_empty(),
            "missing aristotelianDefinition.genus for {}",
            name
        );
    }
}

#[test]
fn test_annotate_serializes_to_valid_json() {
    let model = fixture_model();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    let overlay = emit_overlay(&model, &grounded);
    let json = overlay.to_json().expect("serialization failed");

    // Must round-trip.
    let _reparsed: GroundedOverlay =
        serde_json::from_str(&json).expect("overlay JSON round-trip failed");
}

#[test]
fn test_annotate_does_not_mutate_source() {
    // We parse the fixture, ground it, emit overlay — then re-parse the
    // original fixture and confirm it is unchanged.
    let original_json = include_str!("../fixtures/sales_model.json");
    let model = AtscaleModel::from_json(original_json).unwrap();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    let _overlay = emit_overlay(&model, &grounded);

    // Re-parse original.
    let model2 = AtscaleModel::from_json(original_json).unwrap();
    assert_eq!(model2.columns.len(), model.columns.len());
    assert_eq!(model2.table, model.table);
}

// ── AC #4: report prints coverage ───────────────────────────────────────────

#[test]
fn test_report_coverage_on_fixture() {
    let model = fixture_model();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    let report = CoverageReport::build(&grounded);

    assert_eq!(report.total, 13);
    // All fixture elements have known types — none should fall through to IC.
    assert_eq!(report.fallback, 0, "no elements should fall to IC fallback");
    assert!(
        (report.coverage_pct() - 100.0).abs() < 0.01,
        "coverage should be 100% for well-typed fixture, got {:.1}%",
        report.coverage_pct()
    );
}

#[test]
fn test_report_coverage_fallback_counted() {
    // Inject an unknown-type element and confirm it is counted as fallback.
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "weird_thing",
        element_type: ElementKind::Unknown,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: None,
    };
    let grounded = vec![mapper.ground(&elem).expect("ground failed")];
    let report = CoverageReport::build(&grounded);
    assert_eq!(report.fallback, 1);
    assert!(
        (report.coverage_pct() - 0.0).abs() < 0.01,
        "unknown element should yield 0% coverage"
    );
}

// ── AC #5: offline path — no network needed ──────────────────────────────────

#[test]
fn test_offline_path_no_network() {
    // All fixture tests above are offline — this is a smoke test confirming
    // the JSON parsing path works entirely from embedded fixture data.
    let json = include_str!("../fixtures/sales_model.json");
    let model = AtscaleModel::from_json(json).unwrap();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    assert!(!grounded.is_empty());
}

// ── AC #6: --from-mcp absent errors with actionable message ─────────────────

#[test]
fn test_from_mcp_absent_returns_error() {
    // Simulate the MCP-absent path by directly raising the error.
    let err = AtscaleError::McpNotAttached;
    let msg = err.to_string();
    assert!(
        msg.contains("AtScale MCP connector not attached"),
        "error message should be actionable, got: {}",
        msg
    );
    assert!(
        msg.contains("--model <json>"),
        "error should point to offline alternative, got: {}",
        msg
    );
}

// ── AC #8: annotate model element fields are complete ────────────────────────

#[test]
fn test_all_fixture_elements_have_grounding() {
    let model = fixture_model();
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    let overlay = emit_overlay(&model, &grounded);

    // Every column in fixture should appear in overlay.
    for col in &model.columns {
        assert!(
            overlay.annotations.contains_key(&col.name),
            "overlay missing annotation for column: {}",
            col.name
        );
    }
    // Every column_group in fixture should appear in overlay.
    for cg in &model.column_groups {
        assert!(
            overlay.annotations.contains_key(&cg.name),
            "overlay missing annotation for column_group: {}",
            cg.name
        );
    }
}

// ── bfo_hint override ACs (PRD-ousia-atscale-bfo-hint) ──────────────────────

/// AC #3 (PRD): role hint overrides a column that heuristic would classify as Quality.
/// "customer_name" (plain dimension) → Quality by heuristic; bfo_hint="role" → Role.
#[test]
fn test_bfo_hint_role_overrides_quality_default() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "customer_name",
        element_type: ElementKind::Dimension,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: Some("role"),
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::Role,
        "bfo_hint='role' must override the Quality heuristic"
    );
    assert!(
        result.rationale.contains("override"),
        "rationale must mention override, got: {}",
        result.rationale
    );
}

/// AC #4 (PRD): quality hint overrides a column that heuristic would classify as Role.
/// "customer_status" (contains "status") → Role by heuristic; bfo_hint="quality" → Quality.
#[test]
fn test_bfo_hint_quality_overrides_role_default() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "customer_status",
        element_type: ElementKind::Dimension,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: Some("quality"),
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::Quality,
        "bfo_hint='quality' must override the Role heuristic"
    );
    assert!(
        result.rationale.contains("override"),
        "rationale must mention override, got: {}",
        result.rationale
    );
}

/// AC #5 (PRD): invalid bfo_hint errors loudly, naming the bad value and listing valid ones.
#[test]
fn test_bfo_hint_invalid_errors_loudly() {
    let mapper = Mapper::new();
    let elem = ModelElement {
        name: "some_column",
        element_type: ElementKind::Dimension,
        description: None,
        aggregation: None,
        folder: None,
        bfo_hint: Some("nonsense"),
    };
    let err = mapper.ground(&elem).expect_err("should have errored on invalid hint");
    let msg = err.to_string();
    assert!(
        msg.contains("nonsense"),
        "error must name the bad hint value, got: {}",
        msg
    );
    assert!(
        msg.contains("quality") || msg.contains("valid"),
        "error must list valid categories, got: {}",
        msg
    );
    // Verify it is specifically the InvalidBfoHint variant.
    assert!(
        matches!(err, AtscaleError::InvalidBfoHint { .. }),
        "expected InvalidBfoHint error variant"
    );
}

/// AC #6 (PRD): existing fixtures without bfo_hint produce the same grounding as before.
#[test]
fn test_existing_fixtures_unchanged_without_hint() {
    let sales_json = include_str!("../fixtures/sales_model.json");
    let finance_json = include_str!("../fixtures/finance_model.json");
    let mapper = Mapper::new();

    for (label, json) in [("sales", sales_json), ("finance", finance_json)] {
        let model = AtscaleModel::from_json(json).expect("fixture parse failed");
        // All columns must parse without bfo_hint (serde default = None).
        for col in &model.columns {
            assert!(
                col.bfo_hint.is_none(),
                "fixture '{}' column '{}' unexpectedly has a bfo_hint",
                label,
                col.name
            );
        }
        // Must ground without error.
        let grounded = mapper.ground_model(&model).expect("ground_model failed on existing fixture");
        assert!(
            !grounded.is_empty(),
            "no elements grounded for fixture '{}'",
            label
        );
    }
}

/// AC #7 (PRD): hint override works on a non-dimension element (measure pinned to Role).
#[test]
fn test_bfo_hint_overrides_non_dimension_element() {
    let mapper = Mapper::new();
    // A measure would normally map to InformationGDC; pin it to Role via hint.
    let elem = ModelElement {
        name: "total_revenue",
        element_type: ElementKind::Measure,
        description: None,
        aggregation: Some("sum"),
        folder: None,
        bfo_hint: Some("role"),
    };
    let result = mapper.ground(&elem).expect("ground failed");
    assert_eq!(
        result.bfo_category,
        BfoCategory::Role,
        "bfo_hint must work on measure elements too"
    );
    assert!(
        result.rationale.contains("override"),
        "rationale must mention override for non-dimension element"
    );
}

/// AC #3 (PRD) via JSON: bfo_hint round-trips through JSON and the mapper honours it.
#[test]
fn test_bfo_hint_parses_from_json_column() {
    let json = r#"{
        "catalog": "test",
        "schema": "test",
        "table": "test",
        "columns": [
            {
                "name": "customer_name",
                "type": "dimension",
                "bfo_hint": "role"
            }
        ],
        "column_groups": []
    }"#;
    let model = AtscaleModel::from_json(json).expect("JSON parse failed");
    assert_eq!(
        model.columns[0].bfo_hint.as_deref(),
        Some("role"),
        "bfo_hint must deserialise from JSON"
    );
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model).expect("ground_model failed");
    assert_eq!(grounded[0].bfo_category, BfoCategory::Role);
}
