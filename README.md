# ousia-atscale

BFO grounding bridge for the AtScale semantic layer.

Maps AtScale model elements (measures, dimensions, column-groups) onto BFO 2020
upper-level categories and emits annotation overlays using the paper annotation
vocabulary (`philosophicalGrounding`, `domainModule`, `aristotelianDefinition`).

See [MARKET.md](MARKET.md) for the market framing and book-outline thesis.

## Usage

```sh
# Propose BFO mapping for each element in a model
ousia-atscale ground --model atscale-model.json

# Emit a grounded annotation overlay (does NOT mutate the source file)
ousia-atscale annotate --model atscale-model.json --out grounded.json

# Print coverage: % of model elements with a BFO mapping
ousia-atscale report --model atscale-model.json
```

The `--from-mcp <catalog.schema.table>` flag pulls the model live from an
attached AtScale MCP connector (interactive sessions only). It errors with an
actionable message when the connector is absent.

## Build

```sh
cargo build --release
```

## Test

```sh
cargo test
```

## Install

```sh
bash install.sh
```

## License

MIT — Joe Yen
