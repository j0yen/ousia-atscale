# Contributing to ousia-atscale

## What this project does

`ousia-atscale` maps AtScale semantic-layer model elements to BFO 2020 upper-level
categories. The mapping rules live in `src/mapper.rs`. The goal is that *every*
element a well-formed AtScale model can express has a deterministic, justified BFO
category — no silent fallbacks, every assignment carrying a rationale sentence.

## Getting started

```sh
git clone https://github.com/j0yen/ousia-atscale.git
cd ousia-atscale
cargo build --release
cargo test
```

Requires Rust 1.85+ ([rustup.rs](https://rustup.rs)).

## Contribution checklist

1. **Add a fixture first.** If your change affects how an element type is mapped,
   add or extend a JSON fixture under `fixtures/` that demonstrates the case. Real
   model shapes are better than minimal stubs.

2. **Write or update a test.** Tests live in `tests/ground_tests.rs`. Each AC has
   its own test block (`// ── AC #N ──`). Add your case in the right block or open
   a new one if it covers a genuinely new acceptance criterion.

3. **Keep mapping rules in `src/mapper.rs`.** The doc-comment table at the top of
   that file is the authoritative reference for the BFO assignment logic. If you
   change or extend the heuristics, update the table *and* the `impl` below it
   together — they must stay in sync.

4. **Pass the gates:**
   ```sh
   cargo test
   cargo clippy -- -D warnings
   ```

5. **No new external dependencies** unless the BFO mapping genuinely requires them.
   The crate is intentionally dependency-light (clap, serde, thiserror, anyhow,
   sigpipe). A new dep needs a strong reason.

## What's in scope

- New element `type` values AtScale exposes that the mapper doesn't handle yet
- Better heuristics for Quality vs Role disambiguation
- New annotation fields beyond `philosophicalGrounding` / `domainModule` /
  `aristotelianDefinition` (add to `src/annotate.rs`)
- The `bfo_hint` override mechanism (already in the README spec; not yet implemented)
- New fixtures from real AtScale model shapes

## What's out of scope

- Changing the BFO category ontology itself — BFO 2020 (ISO/IEC 21838-2) is the
  upstream standard. Disagreements with BFO belong upstream.
- Adding AtScale API calls or MCP connectivity beyond what `--from-mcp` already does.
- Anything that requires an AtScale license to test. All tests must pass with the
  included fixtures only.

## BFO reference

- BFO 2020 OWL: `http://purl.obolibrary.org/obo/bfo.owl`
- ISO/IEC 21838-2 (the standard): [iso.org/standard/71954.html](https://www.iso.org/standard/71954.html)
- The mapping rationale: `MARKET.md` §"The BFO Mapping Rules"

## Questions

Open a GitHub issue. If you're an AtScale colleague with a real model that produces
unexpected output, paste the `ground` output and we'll sort it out.
