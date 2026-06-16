//! Integration tests for the ousia-atscale MCP server (PRD-ousia-atscale-mcp).
//!
//! AC #2 — initialize handshake → well-formed response
//! AC #3 — tools/list advertises ground_model, coverage_report, diff_models, validate_model
//! AC #4 — ground_model with sales fixture returns 13 elements
//! AC #5 — diff_models consistency with the diff library
//! AC #6 — tool call creates no files in a temp cwd

use mcp_core::serve::serve;
use serde_json::Value;
use std::io::Cursor;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_mcp(requests: &str) -> Vec<Value> {
    let tools = ousia_atscale::mcp::tools();
    let mut output = Vec::new();
    serve(
        Cursor::new(requests),
        &mut output,
        tools,
        "ousia-atscale",
        "0.5.0",
    )
    .expect("serve failed");

    let text = String::from_utf8(output).expect("utf8");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("json parse"))
        .collect()
}

const SALES_JSON: &str = include_str!("../fixtures/sales_model.json");
const FINANCE_JSON: &str = include_str!("../fixtures/finance_model.json");

// ---------------------------------------------------------------------------
// AC #2: MCP initialize handshake
// ---------------------------------------------------------------------------

#[test]
fn ac2_initialize_handshake() {
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string() + "\n";
    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1, "expected exactly one response");
    let r = &responses[0];
    assert_eq!(r["id"], 1);
    assert!(r["result"].is_object(), "result should be an object: {r}");
    assert_eq!(r["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(r["result"]["serverInfo"]["name"], "ousia-atscale");
}

// ---------------------------------------------------------------------------
// AC #3: tools/list advertises all three tools
// ---------------------------------------------------------------------------

#[test]
fn ac3_tools_list_advertises_three_tools() {
    let input = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":null}"#.to_string() + "\n";
    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);
    let tools = &responses[0]["result"]["tools"];
    assert!(tools.is_array(), "tools should be array");
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"ground_model"), "missing ground_model: {names:?}");
    assert!(names.contains(&"coverage_report"), "missing coverage_report: {names:?}");
    assert!(names.contains(&"diff_models"), "missing diff_models: {names:?}");
    assert!(names.contains(&"validate_model"), "missing validate_model: {names:?}");
    assert!(names.len() >= 4, "expected at least 4 tools, got {}", names.len());
}

// ---------------------------------------------------------------------------
// AC #4: ground_model with sales fixture returns 13 elements
// ---------------------------------------------------------------------------

#[test]
fn ac4_ground_model_sales_fixture_13_elements() {
    let model_json_escaped = serde_json::to_string(SALES_JSON).unwrap();
    let input = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"ground_model","arguments":{{"model_json":{model_json_escaped}}}}}}}"#
    ) + "\n";

    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    assert!(r["error"].is_null(), "unexpected error: {r}");

    // The result is wrapped in MCP content array
    let content = &r["result"]["content"];
    assert!(content.is_array(), "content should be array: {r}");

    // Find the text content item and parse the grounded elements
    let text = content
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "text")
        .expect("no text content")["text"]
        .as_str()
        .expect("text not string");

    let grounded: Vec<Value> = serde_json::from_str(text).expect("parse grounded elements");
    assert_eq!(grounded.len(), 13, "expected 13 grounded elements, got {}: {grounded:?}", grounded.len());
}

// ---------------------------------------------------------------------------
// AC #5: diff_models consistency with the diff library
// ---------------------------------------------------------------------------

#[test]
fn ac5_diff_models_consistent_with_cli() {
    use ousia_atscale::{diff::diff_models as lib_diff, AtscaleModel};

    // Ground truth via the library
    let model_a = AtscaleModel::from_json(SALES_JSON).unwrap();
    let model_b = AtscaleModel::from_json(FINANCE_JSON).unwrap();
    let lib_result = lib_diff(&model_a, &model_b);

    // Now via MCP
    let a_escaped = serde_json::to_string(SALES_JSON).unwrap();
    let b_escaped = serde_json::to_string(FINANCE_JSON).unwrap();
    let input = format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"diff_models","arguments":{{"model_a_json":{a_escaped},"model_b_json":{b_escaped}}}}}}}"#
    ) + "\n";

    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    assert!(r["error"].is_null(), "unexpected error: {r}");

    let text = r["result"]["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "text")
        .expect("no text content")["text"]
        .as_str()
        .expect("text not string");

    let mcp_result: Value = serde_json::from_str(text).unwrap();

    // Compare diverge + agree counts
    let mcp_agree = mcp_result["agree"].as_array().unwrap().len();
    let mcp_diverge = mcp_result["diverge"].as_array().unwrap().len();
    assert_eq!(mcp_agree, lib_result.agree.len(), "agree mismatch");
    assert_eq!(mcp_diverge, lib_result.diverge.len(), "diverge mismatch");
}

// ---------------------------------------------------------------------------
// AC #6: tool call creates no files in temp cwd
// ---------------------------------------------------------------------------

#[test]
fn ac6_tool_call_no_file_io() {
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Change to the temp dir so any accidental writes land there
    std::env::set_current_dir(tmp.path()).unwrap();

    let model_json_escaped = serde_json::to_string(SALES_JSON).unwrap();
    let input = format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"ground_model","arguments":{{"model_json":{model_json_escaped}}}}}}}"#
    ) + "\n";

    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);

    // Restore cwd
    std::env::set_current_dir(&original_dir).unwrap();

    // Assert no files were created in tmp
    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .collect();
    assert!(entries.is_empty(), "unexpected files created: {entries:?}");

    // Also assert no error in the response
    let r = &responses[0];
    assert!(r["error"].is_null(), "unexpected error: {r}");
}

// ---------------------------------------------------------------------------
// Unknown method returns JSON-RPC error
// ---------------------------------------------------------------------------

#[test]
fn unknown_method_returns_error() {
    let input = r#"{"jsonrpc":"2.0","id":10,"method":"no_such_method","params":{}}"#.to_string() + "\n";
    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    assert!(r["result"].is_null(), "expected no result: {r}");
    assert!(r["error"].is_object(), "expected error object: {r}");
    assert_eq!(r["error"]["code"], -32601, "expected method-not-found code: {r}");
}

// ---------------------------------------------------------------------------
// Malformed JSON input is skipped gracefully (no panic)
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_skipped_no_panic() {
    // Malformed line followed by a valid one — only the valid one gets a response
    let input = concat!(
        "not valid json at all!!!\n",
        "{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"tools/list\",\"params\":null}\n"
    );
    let responses = run_mcp(input);
    // Only the valid request produces a response; malformed line is silently skipped
    assert_eq!(responses.len(), 1, "expected exactly 1 response, malformed line skipped");
    let r = &responses[0];
    assert_eq!(r["id"], 11);
    assert!(r["result"]["tools"].is_array());
}

// ---------------------------------------------------------------------------
// validate_model tool — missing reasoner returns a tool error (not a panic)
// ---------------------------------------------------------------------------

#[test]
fn validate_model_missing_reasoner_returns_error() {
    // ousia-reason is not expected to be on PATH in CI / test environments.
    // The tool should return a JSON-RPC error (code -32603) rather than panic.
    let model_json_escaped = serde_json::to_string(SALES_JSON).unwrap();
    let input = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"validate_model","arguments":{{"model_json":{model_json_escaped}}}}}}}"#
    ) + "\n";

    let responses = run_mcp(&input);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    // Either a successful verdict (if ousia-reason is installed) or an internal error.
    // What must NOT happen is a panic or an empty response array.
    let has_result = r["result"].is_object();
    let has_error = r["error"].is_object();
    assert!(
        has_result || has_error,
        "expected either result or error, got: {r}"
    );
}
