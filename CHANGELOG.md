# Changelog

## v0.5.0 — 2026-06-16

Add `serve` subcommand: MCP stdio server exposing `ground_model`, `coverage_report`, and `diff_models` tools.
Uses the shared `mcp-core` crate (same JSON-RPC 2.0 wire format as `ousia-mcp`).
All tools are read-only and perform no file or network I/O during a call.
An AI agent can now query BFO grounding for an AtScale model mid-conversation.
Register via `claude mcp add ousia-atscale -- ousia-atscale serve`.
All 6 MCP acceptance criteria verified green (AC2-AC6, plus AC1 clippy clean).

## v0.4.0 — 2026-06-16

Add `diff` subcommand comparing BFO groundings of two AtScale models element-by-element.
Joins on case-insensitive name; classifies pairs as agree/diverge/only_in_a/only_in_b.
Exits non-zero if divergences found — enables CI gate on semantic consistency.
Supports --format text|json; --verbose shows agreeing elements in text mode.
All 7 acceptance criteria verified green (AC2-AC7).

## v0.3.0 — 2026-06-16

Add optional bfo_hint field to Column struct; mapper checks hint before heuristic; invalid hints error loudly

## v0.2.0 — 2026-06-16

export subcommand: emit BFO-grounded model as RDF/Turtle and OWL/XML; bridges AtScale grounding to ousia-sparql/ousia-reason
