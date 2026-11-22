# E4 Build the logical plan

## E4-1 Purpose

Build a dialect-neutral logical plan (P5) whose result is exactly the Apache Ossie Layer 3
lowering, with row policies, additivity rules and context modifiers applied.

## E4-2 Input and output

| | Content |
|---|---|
| Input | Branches (E3), bound query, security context, internal model (no catalog state: every evidence-based decision belongs to E5) |
| Output | Logical plan: a tree of typed nodes (E4-3) |

## E4-3 Plan nodes

| ID | Node | Meaning |
|---|---|---|
| E4-3.1 | `Scan(relation)` | Read a catalog relation. Every `Scan` reads only the visible rows of its dataset (E4-6.5c). |
| E4-3.2 | `Join(left, right, relationship, kind)` | Join along one relationship; `kind` is `left` unless evidence allows `inner` (set only in E5) |
| E4-3.3 | `Filter(input, predicate)` | Keep rows where the predicate is TRUE |
| E4-3.4 | `SemiJoin(input, path, predicate)` | Keep the rows of `input` for which at least one related row exists along `path` and every related row along it satisfies `predicate` (used for policy propagation, E4-6.5a). A row with a NULL foreign key never matches. |
| E4-3.5 | `Deduplicate(input, group, identity)` | Keep one row per (`group`, `identity`) pair: `group` is the `Aggregate` key values after time-grain truncation, NULLs compared equal, and `identity` is the home row identity (E4-4.0f) |
| E4-3.6 | `Aggregate(input, keys, components)` | Group and evaluate components |
| E4-3.7 | `Stitch(branches, keys)` | Combine branches by dimension keys (E4-5) |
| E4-3.8 | `Project(input, expressions)` | Compute output columns |
| E4-3.9 | `Sort(input, items)` | Order |
| E4-3.10 | `Limit(input, n)` | Limit |
| E4-3.12 | `Population(dataset, population, grain, periods)` | Output the expected (entity, period) pairs of `population` (POP-3) at `grain` over `periods`, the query's unit of periods (ADD-4.7). It reads V(D) of `dataset` (E4-6.5, E2-4.9). |
| E4-3.13 | `Snapshot(input, expected, rule)` | Reduce the rows of `input` to one value per (result group, entity) under the semi-additive `rule` (ADD-3.2, E4-7), `expected` being the output of a `Population` |

- **E4-3.11** Every node carries its output schema (names, logical types, nullability) and its origin. Column names and the order of the inputs of a multi-node plan follow E6-4.12.

## E4-4 Branch plan

For each branch, in order:

1. **E4-4.0a** `Scan` the home dataset.
2. **E4-4.0b** Apply the row policies so that the home `Scan` reads its visible rows (E4-6.5): a `Filter` for its own applicable policies, then a `SemiJoin` for each relationship that E4-6.5 follows.
3. **E4-4.0c** `Join` the join tree with `left` joins, in path order, one `Join` per join instance (E3-4.2a). Each joined `Scan` reads the visible rows of its dataset (E4-6.5c).
4. **E4-4.0d** `Filter` with the query filters, as one conjunction, followed by the `date_range` predicate of QRY-4.2 when the query has one. FALSE and NULL remove the row.
5. **E4-4.0e** If the branch has at least one `Join` and at least one metric component, insert `Deduplicate(group, identity)` before its `Aggregate`, whatever keys the model declares: a home row counts once in **each** group it matches, as Apache Ossie Layer 3 section 4 requires. Only E5 may remove it, with evidence (E5-3.2). A dimension-only branch never gets one.
   **E4-4.0f** `group` is the `Aggregate` key values after time-grain truncation, with NULLs compared equal. `identity` is one row-number column **synthesized** over the home `Scan` (Apache Ossie Layer 2 section 3.2), so no declared key has to be trusted; results do not depend on the numbering.
6. **E4-4.0g** `Aggregate` by the dimension keys with the branch's components.

- **E4-4.1** Unmatched home rows fall into the NULL group of a dimension. That group is kept, never dropped, so totals always add up; the explanation states when the plan can produce a NULL group and which relationship causes it.
- **E4-4.2** A branch plan with no `Stitch` and no remaining `Deduplicate` after E5 is a **simple plan**; E6 generates it as one aggregate query over the joins.

## E4-5 Stitching branches

With two or more branches, the plan MUST equal Apache Ossie Layer 3 section 5.5:

1. **E4-5.0a** The keys are the distinct dimension tuples present in at least one branch, NULLs equal.
2. **E4-5.0b** For each key and branch, a component takes the branch's value if the branch has a row for that key, and its **empty-input value** otherwise (`SUM` → NULL, `COUNT` → 0, as defined by its expression over no rows).
3. **E4-5.0c** Metrics are evaluated from their components after stitching.

- **E4-5.1** A NULL group (a branch row whose key is NULL) and a missing branch row are different: the first supplies values, the second empty-input values.
- **E4-5.2** Without dimensions, each branch has exactly one row.

## E4-6 Row policies

| ID | Rule |
|---|---|
| E4-6.1 | Row policies are injected here, before E5, as `Filter` and `SemiJoin` nodes directly over the `Scan` of their dataset, so that every `Scan` reads only the visible rows of its dataset (E4-6.5). No later stage may remove, weaken or move them above an aggregate. |
| E4-6.2 | **Evaluation.** For a dataset and a user, the row filter is `G AND R`, computed in this order: (1) drop every policy the user is exempt from (`exempt_roles`); (2) `R` is the AND of the remaining policies without `roles` (TRUE if none); (3) `G` is the OR of the conditions of the remaining policies whose `roles` include one of the user's roles. If the dataset has no policy with `roles`, `G` is TRUE. |
| E4-6.3 | **Exemption.** An exemption removes only that policy. If the user is exempt from **every** role-scoped policy of the dataset, `G` is TRUE. |
| E4-6.4 | **No grant.** Otherwise, if no remaining role-scoped policy lists one of the user's roles, `G` is the constant `FALSE`: the user sees no rows. Access is denied by a false predicate, never by an error that reveals data exists. |
| E4-6.5 | **Visibility.** The visible rows V(D) of a dataset D are defined recursively: the rows of D that pass its own applicable policies (E4-6.2–E4-6.4) and that, for each usable many-to-one relationship from D to a dataset D' that has row policies or reaches one, satisfy E4-6.5a against V(D'). A row that is not visible is removed, never kept with NULL dimensions. |
| E4-6.5a | **Propagation step.** Along a many-to-one relationship from D to D', a row of D is visible only if at least one related row exists in D' and every related row is in V(D'). A row with a NULL foreign key is never visible, because it cannot be proven allowed (deny by default). |
| E4-6.5b | **Cycles.** While V(D) is computed, a relationship that leads back to a dataset already on the current chain of relationships is not followed. |
| E4-6.5c | **Every scan.** Every `Scan` of a plan, home or joined, including the anchor of a dimension-only branch, reads V(D) of its dataset. Joined dimension values from rows that are not visible never appear. |
| E4-6.6 | **Several relationships.** E4-6.5 applies to each such relationship of D separately, including several relationships to the same dataset: the row must be visible along each. |
| E4-6.7 | **Dimension listing.** Dimension-only queries are governed by their own datasets' access and policies. "Own" means only that the policies of metric home datasets that are not on the branch do not apply. Propagation (E4-6.5) applies to the anchor, as to every `Scan` (E4-6.5c). |
| E4-6.8 | **Context modifiers never remove policies.** A modifier that removes a dimension or filter from a metric's context (E4-8) applies only to query filters and dimensions, never to row policies. |
| E4-6.9 | Attribute values are bound parameters (SEC-3.5). A policy needing an absent attribute fails compilation (`ESEC-02`) only when it is **applicable**: it remains after exemption (E4-6.2 step 1) and either has no `roles` or lists one of the user's roles. A policy the user is exempt from, or one that grants none of the user's roles, never fails. `ESEC-02` is located at `/attributes` of the security context, and its message names only the missing attribute, never the policy, its dataset or its condition (PLN-5.2, SCY-4a.1). Branches are visited in branch order (E3-4.3). Within each branch, policies are visited for the home dataset first, then for the other datasets whose policies take part in V(D) of the home dataset (E4-6.5), ordered by the length of their shortest path from the home dataset, then by normalized name; then the population dataset of a semi-additive branch (E2-4.9) and the datasets that take part in its V(D), in the same order from the population dataset; each dataset is visited once per query. Within a dataset, policies are visited in CON-5 order of their names (E0H-1.4), and each condition's attributes in order of appearance; the first missing attribute is reported. |
| E4-6.9a | **Empty list attribute.** `x IN (USER_ATTRIBUTE('a'))` is FALSE and `x NOT IN (USER_ATTRIBUTE('a'))` is TRUE when `a` is an empty list, whatever `x` is, NULL included, as SQL `= ANY` over an empty set. |
| E4-6.9b | **Now in a row policy.** An applicable policy (E4-6.9) whose condition uses `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP` needs `reference_time` (QRY-2). Without it, compilation fails with `EQRY-03`, located at `/query`, its message naming neither the policy nor its dataset (SCY-4a.1). Policies are visited as in E4-6.9, after every `ESEC-02` check. |
| E4-6.10 | Joins added only for propagation never add output columns, and the explanation names the policies without showing any value. |

## E4-7 Additivity

Semi-additive metrics (balances, inventory) are planned as defined by the
[additivity extension](../extensions/additivity.md) and the
[population extension](../extensions/population.md), with `Population` and `Snapshot` nodes.

| ID | Rule |
|---|---|
| E4-7.1 | **Evaluation.** In each evaluation unit (ADD-4.7), each component of the metric is evaluated per (result row, entity, `over` truncated to `snapshot_grain`); `aggregation` is then applied per entity across those values; the entity values of each component are then combined with the recomposition function of its aggregate (E0G-3: `SUM` and `COUNT` by `SUM`, `MIN` by `MIN`, `MAX` by `MAX`). The metric expression is evaluated from the combined components, as after stitching (E4-5.0c). A semi-additive metric has only distributive components (E0G-4.1), so every component has a recomposition function. |
| E4-7.2 | **Average.** `aggregation: avg` averages an entity's values over the periods of the unit where the entity has a value after `missing` and `nulls` apply. |
| E4-7.3 | **Periods with data.** The periods with data (ADD-4.7) are the periods of the home rows that pass the row policies and the query filters, for any entity, rows with a NULL value included. |
| E4-7.4 | **Date range.** The query `date_range` bounds the unit only when its dimension is `over`; otherwise it is a query filter like any other. |
| E4-7.5 | **Carry-forward read.** The read that finds carried values keeps the row policies and the other query filters, and drops only the `date_range` predicate on `over`. |
| E4-7.6 | **Rows that do not exist.** A carried row keeps the fields of the row it carries. A row added for `missing: zero` has its `entity` fields from the population and NULL for every other home field; it joins like a home row and can fall into the NULL group (E4-4.1). Query filters apply after these rows are added. |
| E4-7.7 | **Missing under `per_entity`.** With `selection: per_entity`, an entity is missing when it is expected in the last (or first, per `aggregation`) period of the unit and has no row in the unit. |
| E4-7.8 | **Grain of a plain `over`.** A query dimension `over` without a time grain has grain `day` for a `date` and the source precision for a `timestamp` (ADD-4.9). In the SQL form, `DATE_TRUNC('<grain>', over)` counts as that grain. `E4-03` is located at the `time_grains` entry, else at the dimension, else at the SQL `SELECT` item. |
| E4-7.9 | **Non-additive.** Before summaries (0.6.0), a non-additive metric plans exactly as its deduced additivity. |

## E4-8 Context modifiers

Period-to-date, prior-period and percent-of-total metrics are planned as defined by the
[context extension](../extensions/context.md). Each modified metric becomes an additional
branch with its own grain and filters, stitched by the query keys.

## E4-9 Order and limit

- **E4-9.1** `Sort` uses the query's explicit items only; ties are impossible because a limit requires a total order (QRY-4.6).
- **E4-9.2** Without `order_by`, the row order is undefined, as in Apache Ossie.

## E4-10 Diagnostics

| Code | Condition |
|---|---|
| `ESEC-02` | An applicable row policy needs an absent attribute |
| `EQRY-03` | An applicable row policy needs `reference_time` and the query has none (E4-6.9b) |
| `E4-02` | A context modifier needs a time dimension the query lacks, or at a coarser grain |
| `E4-03` | Query grain of a semi-additive `over` finer than its `snapshot_grain` (ADD-4.9, E4-7.8); located at the `time_grains` entry, else the dimension, else the SQL `SELECT` item |
