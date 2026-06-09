# Market Framing: Formal Semantics for the Semantic Layer

> *"Ten tools, ten definitions of revenue."*
> — *The Ontological Semantic Layer* (book outline, §1)

## The Thesis

The semantic-layer category has a semantics problem. Existing tools — AtScale,
Cube, dbt Metrics, LookML — encode metrics as YAML and property graphs. These
representations capture **syntax** (how measures are computed) but not
**meaning** (what the measure *is*). When a CFO asks "is this the same revenue
as last quarter?" there is no formal answer.

**BFO (Basic Formal Ontology) and OWL 2 DL provide that answer.**

The World Ontology project (`ousia-*`) formalises an upper ontology over BFO
categories. `ousia-atscale` applies it to a real, shipping semantic-layer
product — AtScale — to demonstrate that formal grounding is:

1. **Implementable today.** Every AtScale model element maps to a BFO category
   via deterministic rules (see `src/mapper.rs`).
2. **Machine-checkable.** The grounding is stored as an annotation overlay
   (`philosophicalGrounding`, `domainModule`, `aristotelianDefinition`) that
   can be validated with OWL reasoners.
3. **AI-ready.** A semantic layer whose measures and dimensions trace to BFO
   categories can be traversed deductively by an LLM or agent — the agent
   *knows* that `revenue` is an information GDC about a sales process, not a
   property graph label.

## The BFO Mapping Rules (from *The Ontological Semantic Layer* §4.4)

| AtScale element | BFO 2020 category | Why |
|-----------------|------------------|----|
| measure | information GDC | A measure carries propositional content about the magnitude of a process. |
| dimension / attribute | quality *or* role | Intrinsic properties → quality; relational/status → role. |
| date / time | temporal region | Temporal locations in BFO are temporal regions. |
| hierarchy / level | role | A level is a role played by members in a classification scheme. |
| key | quality | An identifier is a quality that inheres in its bearer. |

## The Pitch

> *"The formal semantic layer is a competitive moat. Once your measures are
> BFO-grounded, no tool can misinterpret them — not an AI agent, not a new
> analyst, not a downstream system. The definition lives in the ontology, not
> the YAML."*
> — *The Ontological Semantic Layer*, §6 (Executive Summary draft)

`ousia-atscale` is that pitch made **executable**. Run `ousia-atscale ground`
against any AtScale model and you get a formal grounding in seconds. Run
`annotate` and you get an overlay ready for OWL import. Run `report` and you
have a coverage metric you can show to a CTO.

## Next Steps

- Integrate the grounded overlay into the AtScale model YAML/JSON as a first-class
  property (pending AtScale API support).
- Expose `ousia-atscale` as an MCP tool so AI agents can query grounding directly.
- Extend to cover measure groups and calculation chains (derived measures →
  GDC about composed processes).
- Publish as a white-paper companion tool for *The Ontological Semantic Layer*.
