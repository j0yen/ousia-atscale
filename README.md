# ousia-atscale

**BFO 2020 grounding bridge for the AtScale semantic layer.**

Semantic-layer tools encode *how* metrics are computed but not *what they mean*.
When an AI agent or a new analyst asks "is this the same revenue as last quarter?"
there is no formal answer — just YAML.

`ousia-atscale` gives a formal answer. It maps every element in an AtScale model
(measures, dimensions, keys, hierarchies) onto a **BFO 2020 upper-level category**
and emits an annotation overlay with three new fields per element:

- `philosophicalGrounding` — the BFO class IRI (e.g. `BFO_0000031` = GDC)
- `domainModule` — which BFO branch (continuant / occurrent / …)
- `aristotelianDefinition` — human-readable definition citing BFO genus + differentia

Based on §4.4 of [*The Ontological Semantic Layer*](MARKET.md).

---

## Quickstart

**No AtScale instance required.** The `--model` flag takes any JSON file matching
AtScale's `describe_model` output shape (see `fixtures/`).

```sh
# Clone and build
git clone https://github.com/j0yen/ousia-atscale.git
cd ousia-atscale
cargo build --release

# Try it on the included sales fixture
./target/release/ousia-atscale ground  --model fixtures/sales_model.json
./target/release/ousia-atscale report  --model fixtures/sales_model.json
./target/release/ousia-atscale annotate --model fixtures/sales_model.json --out /tmp/grounded.json

# Install to ~/.local/bin/
bash install.sh
```

Sample `ground` output:

```
BFO Grounding — acme.sales.fact_orders

Name                 ElementType   BFO Category                                Rationale
─────────────────────────────────────────────────────────────────────────────────────────
revenue              measure       information generically dependent continuant  'revenue' is a measure — an information GDC…
customer_status      dimension     role                                          'customer_status' has relational/status semantics…
order_date           date          temporal region                               'order_date' is a date/time field — a temporal…
```

Sample `report` output:

```
=== BFO Grounding Coverage Report ===
Model : acme.sales.fact_orders
Total elements : 13
Grounded       : 13 (100.0%)

By BFO category:
  information generically dependent continuant  3
  quality                                       4
  role                                          4
  temporal region                               2
```

---

## Using your own model

Export your model from AtScale as JSON — the same shape that the AtScale MCP
connector's `describe_model` tool returns:

```json
{
  "catalog": "my_catalog",
  "schema": "my_schema",
  "table": "my_model",
  "description": "...",
  "columns": [
    { "name": "revenue",    "type": "measure",   "description": "...", "aggregation": "sum" },
    { "name": "region",     "type": "dimension", "description": "..." },
    { "name": "order_date", "type": "date",      "description": "..." }
  ],
  "column_groups": [
    { "name": "RegionHierarchy", "type": "hierarchy", "description": "..." }
  ]
}
```

Then:

```sh
ousia-atscale ground   --model my_model.json
ousia-atscale annotate --model my_model.json --out my_model_grounded.json
ousia-atscale report   --model my_model.json
```

---

## The BFO Mapping Rules

| AtScale element | BFO 2020 category | Why |
|-----------------|-------------------|-----|
| `measure` | Information GDC (`BFO_0000031`) | A measure carries propositional content *about* a process (revenue is a GDC about a sales process). |
| `dimension` / `attribute` — intrinsic | Quality (`BFO_0000019`) | Properties that inhere in a bearer (name, region). |
| `dimension` / `attribute` — status/type/category | Role (`BFO_0000023`) | Relational classifications a bearer plays in a context. |
| `date` / `time` | Temporal Region (`BFO_0000008`) | Temporal locations and intervals in BFO. |
| `key` / `identifier` | Quality (`BFO_0000019`) | An identifier is a quality that inheres in its bearer. |
| `hierarchy` / `level` / `set` | Role (`BFO_0000023`) | A hierarchy level is a role played by members in a classification scheme. |

The heuristic for `dimension` → Quality vs Role: if the column name contains
"status", "type", "category", "level", or "class", it maps to Role; otherwise Quality.
You can override by setting `"bfo_hint": "quality"` or `"bfo_hint": "role"` in the
column JSON.

---

## Live MCP mode

If you have the AtScale MCP connector attached to your Claude session:

```sh
ousia-atscale ground   --from-mcp acme.sales.fact_orders
ousia-atscale annotate --from-mcp acme.sales.fact_orders --out grounded.json
ousia-atscale report   --from-mcp acme.sales.fact_orders
```

When the MCP connector is absent the tool exits with an actionable error message.

---

## Fixtures

Two example models ship with the repo so you can try the tool without any AtScale instance:

| File | Description |
|------|-------------|
| `fixtures/sales_model.json` | ACME Corp order fact table — measures, dimensions, date, hierarchy, named set |
| `fixtures/finance_model.json` | P&L model — budget vs actuals, cost centres, fiscal periods |

---

## Build requirements

- Rust 1.85+ (install via [rustup.rs](https://rustup.rs))
- No external runtime dependencies

---

## Tests

```sh
cargo test
```

16 integration tests covering all 8 acceptance criteria.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

Short version:
1. Add a fixture in `fixtures/` that motivates the change.
2. Add or update a test in `tests/ground_tests.rs`.
3. Keep the BFO mapping rules in `src/mapper.rs`; the doc-comment table at the top of
   that file is the authoritative reference.
4. `cargo test` and `cargo clippy -- -D warnings` must pass.

---

## Project context

`ousia-atscale` is one component of the **ousia** project — a suite of Rust tools
that operationalise the *World Ontology* (a BFO-grounded, OWL 2 DL ontology of 509
classes encoding ethical commitments as reasoner-enforced axioms). See
[MARKET.md](MARKET.md) for the market framing and thesis.

Other ousia tools: `ousia-forge` (OWL builder), `ousia-reason` (OWL 2 DL entailment),
`ousia-sparql` (SPARQL 1.1 query layer), `ousia-guard` (action gating), `ousia-mcp`
(MCP server).

---

## License

MIT — Joe Yen
