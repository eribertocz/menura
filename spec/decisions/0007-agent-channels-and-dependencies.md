# 0007 Agent channels, audience boundary and model dependencies

## Context

The status review of agent support (2026-11-22) found five gaps. F29 put `menura-mcp` in this repository, while decision 0004 item D called it an external module. Who may see a compiled plan was not stated. Tool parity was not defined, and the MCP server had no version. Other modules have no model-level dependency contract for impact analysis. `migrate` and `menura diff` have no versions. The maintainer decided them.

## Decision

1. `menura-mcp` is a wrapper crate of this repository, like `cli`. It uses standard input and output only, has no network and no logic of its own, and is released with every Menura version from 0.9.0 (ARC-6.1). This amends decision 0004 item D.
2. Audience boundary (ARC-6.2). `menura-mcp` and the CLI are for modelers and developers, and may show the whole compiled plan. End users and their agents query through the platform's semantic service (Accipiter, external). That service shows the explanation without SQL, parameters or policy conditions.
3. Parity (ARC-6.1). Each public function and each CLI command has one MCP tool, with the same inputs, validations and results. Version 0.9.0.
4. Model-dependency contract. Before 1.0.0, in 0.8.0, Menura publishes a public contract with the model-level graph metric → fields → datasets → physical columns and policies. It reuses the vocabulary of PLN-4 (`metric:`, `field:`, `relationship:`, `<source>.<column>`, `<dataset>.<policy>`) and is not computed per query. Mergus consumes it (MG-REQ-1423) to compute the impact radius. Its rules are written in the 0.8.0 specification work.
5. Versions. `menura diff` (F33) ships with the dependency contract in 0.8.0. `migrate` ships in 0.9.0, with the complete CLI.

0.7.0 stays the priority. None of this changes its scope.

## Consequences

Agents of end users never receive SQL from Menura. Authoring agents get the same capabilities as the CLI. Mergus gets a stable contract instead of parsing plans.

## Rules changed

ARC-6.1 and ARC-6.2 (new), architecture 3.6 table, roadmap 7.1 rows 0.8.0 and 0.9.0, backlog F29 and F33, decision 0004 (amendment).
