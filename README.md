# ousia-atscale

A CLI that maps every element of an AtScale semantic model onto a BFO 2020 upper-level category, so a model's elements carry not just a computation rule but a formal account of what they are.

## Why it exists

A semantic layer says *how* a metric is computed. It does not say *what the metric means*. When an AI agent or a new analyst asks "is this the same revenue as last quarter?", the model has no formal answer — only YAML and a SQL expression. Two columns named `revenue` can be the same thing or different things, and nothing in the model can tell you which.

`ousia-atscale` gives the model something to answer with. It reads an AtScale model and assigns each element — measure, dimension, key, date, hierarchy — a category from Basic Formal Ontology (BFO 2020, ISO 21838), the upper-level ontology that distinguishes a thing from a process, a quality from a role, a piece of information from what the information is about. A measure becomes an information generically dependent continuant; a status column becomes a role; a date becomes a temporal region. The mapping is emitted as an annotation overlay — three new fields per element, alongside the original model, never mutating it.

The argument behind this is §4.4 of [*The Ontological Semantic Layer*](MARKET.md).

## What it produces

Each grounded element gains three fields:

- `philosophicalGrounding` — the BFO class IRI (e.g. `BFO_0000031`, an information GDC)
- `domainModule` — the BFO branch (continuant / occurrent / …)
- `aristotelianDefinition` — a human-readable definition in genus + differentia form, citing the BFO parent

## Quickstart

No AtScale instance is required. Every offline command reads a `--model` JSON file in the shape of AtScale's `describe_model` output; two example models ship in `fixtures/`.

```sh
git clone https://github.com/j0yen/ousia-atscale.git
cd ousia-atscale
cargo build --release

# Propose a BFO category for each element
./target/release/ousia-atscale ground   --model fixtures/sales_model.json

# Coverage report — what fraction of elements got grounded
./target/release/ousia-atscale report   --model fixtures/sales_model.json

# Write the annotation overlay (source model untouched)
./target/release/ousia-atscale annotate --model fixtures/sales_model.json --out grounded.json

# Emit the grounded model as RDF, so a reasoner can classify it
./target/release/ousia-atscale export    --model fixtures/sales_model.json --format turtle --out grounded.ttl

# Install to ~/.local/bin/
bash install.sh
```

`export` writes OWL named individuals typed to their BFO class, with `owl:imports` of BFO, in Turtle or OWL/XML — the bridge from a grounded AtScale model to the rest of the ousia toolchain.

## Using your own model

Export your model from AtScale as JSON — the shape the AtScale MCP connector's `describe_model` returns:

```json
{
  "catalog": "my_catalog",
  "schema": "my_schema",
  "table": "my_model",
  "columns": [
    { "name": "revenue",    "type": "measure",   "aggregation": "sum" },
    { "name": "region",     "type": "dimension" },
    { "name": "order_date", "type": "date" }
  ],
  "column_groups": [
    { "name": "RegionHierarchy", "type": "hierarchy" }
  ]
}
```

Then run `ground`, `annotate`, `report`, or `export` against it with `--model my_model.json`.

## The BFO mapping rules

| AtScale element | BFO 2020 category | Why |
|-----------------|-------------------|-----|
| `measure` | Information GDC (`BFO_0000031`) | A measure carries propositional content *about* a process — revenue is information about a sales process. |
| `dimension` / `attribute`, intrinsic | Quality (`BFO_0000019`) | A property that inheres in a bearer (name, region). |
| `dimension` / `attribute`, status/type/category | Role (`BFO_0000023`) | A relational classification a bearer plays in a context. |
| `date` / `time` | Temporal Region (`BFO_0000008`) | A temporal location or interval. |
| `key` / `identifier` | Quality (`BFO_0000019`) | An identifier is a quality that inheres in its bearer. |
| `hierarchy` / `level` / `set` | Role (`BFO_0000023`) | A level is a role played by members in a classification scheme. |

The dimension-to-Quality-vs-Role split is a heuristic: a name containing "status", "type", "category", "level", or "class" maps to Role; otherwise Quality. Override it per column by setting `"bfo_hint": "quality"` or `"bfo_hint": "role"` in the column JSON; an invalid hint errors rather than being silently ignored. The authoritative rule table lives in the doc-comment at the top of `src/mapper.rs`.

## Live MCP mode (reserved, not yet wired)

Every command also accepts `--from-mcp <catalog.schema.table>`, intended to pull the model live from an attached AtScale MCP connector instead of a file. The flag is present and parsed, but the live path is not implemented: it returns an explicit "MCP connector not attached" error in every context today. Use `--model` until this is wired.

## Where it fits

`ousia-atscale` is the AtScale bridge of the **ousia** project — a suite of Rust tools that operationalize a BFO-grounded, OWL 2 DL ontology. Its `export` output feeds the rest of the chain: [`ousia-forge`](https://github.com/j0yen/ousia-forge) builds the OWL, and the planned `ousia-reason`, `ousia-sparql`, and `ousia-guard` reason over, query, and gate on it. [MARKET.md](MARKET.md) carries the thesis.

## Status

Early. The concept, the mapping rules, and the fixtures are real; the offline `ground` / `annotate` / `report` / `export` commands are the working surface, and `--from-mcp` is a reserved stub.

**The v0.3.0 commit does not compile** — the `bfo_hint` change left `cmd_ground` and the report path treating a `Vec<GroundedElement>` and a `Result` as if they were single grounded elements (11 type errors in `src/main.rs`). Build it after that regression is fixed; the sample outputs and the integration-test suite below describe the intended, pre-regression behavior.

## Tests

```sh
cargo test
```

The suite is 16 integration tests under `tests/`, covering the eight acceptance criteria — once the crate compiles again.

## Build requirements

- Rust 1.85+ ([rustup.rs](https://rustup.rs))
- No external runtime dependencies

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: add a fixture that motivates the change, add or update a test in `tests/`, keep the mapping rules in `src/mapper.rs`, and make `cargo test` and `cargo clippy -- -D warnings` pass.

## License

MIT — Joe Yen.
