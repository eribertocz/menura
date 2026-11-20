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

| Node | Meaning |
|---|---|
| `Scan(relation)` | Read a catalog relation |
| `Join(left, right, relationship, kind)` | Join along one relationship; `kind` is `left` unless evidence allows `inner` (set only in E5) |
| `Filter(input, predicate)` | Keep rows where the predicate is TRUE |
| `SemiJoin(input, path, predicate)` | Keep rows whose related row along `path` exists and satisfies `predicate` (used for policy propagation) |
| `Deduplicate(input, group, identity)` | Keep one row per (dimension key tuple, home row identity) pair |
| `Aggregate(input, keys, components)` | Group and evaluate components |
| `Stitch(branches, keys)` | Combine branches by dimension keys (E4-5) |
| `Project(input, expressions)` | Compute output columns |
| `Sort(input, items)` and `Limit(input, n)` | Order and limit |

Every node carries its output schema (names, logical types, nullability) and its origin.

## E4-4 Branch plan

For each branch, in order:

1. `Scan` the home dataset.
2. Apply the row policies of the home dataset (`Filter`) and of every dataset it reaches (`SemiJoin`, E4-6.5).
3. `Join` the join tree with `left` joins, in path order.
4. `Filter` with the query filters, as one conjunction, followed by the `date_range` predicate of QRY-4.2 when the query has one. FALSE and NULL remove the row.
5. If any join step of the branch could match more than one row per home row, insert `Deduplicate(group, identity)` before aggregation: a home row counts once in **each** group it matches, as Apache Ossie Layer 3 section 4 requires. `identity` is a row identity **synthesized** for the home scan (a row number assigned in the home relation's scan, Apache Ossie Layer 2 section 3.2), so no declared key has to be trusted; results do not depend on the numbering. E4 always inserts the node for such a step; only E5 may remove it, with evidence (E5-3.2).
6. `Aggregate` by the dimension keys with the branch's components.

- **E4-4.1** Unmatched home rows fall into the NULL group of a dimension. That group is kept, never dropped, so totals always add up; the explanation states when the plan can produce a NULL group and which relationship causes it.
- **E4-4.2** A branch plan with no `Stitch` and no remaining `Deduplicate` after E5 is a **simple plan**; E6 generates it as one aggregate query over the joins.

## E4-5 Stitching branches

With two or more branches, the plan MUST equal Apache Ossie Layer 3 section 5.5:

1. The keys are the distinct dimension tuples present in at least one branch, NULLs equal.
2. For each key and branch, a component takes the branch's value if the branch has a row for that key, and its **empty-input value** otherwise (`SUM` → NULL, `COUNT` → 0, as defined by its expression over no rows).
3. Metrics are evaluated from their components after stitching.

- **E4-5.1** A NULL group (a branch row whose key is NULL) and a missing branch row are different: the first supplies values, the second empty-input values.
- **E4-5.2** Without dimensions, each branch has exactly one row.

## E4-6 Row policies

| ID | Rule |
|---|---|
| E4-6.1 | Row policies are injected here, before E5, as `Filter` nodes directly over the `Scan` of their dataset. No later stage may remove, weaken or move them above an aggregate. |
| E4-6.2 | **Evaluation.** For a dataset and a user, the row filter is `G AND R`, computed in this order: (1) drop every policy the user is exempt from (`exempt_roles`); (2) `R` is the AND of the remaining policies without `roles` (TRUE if none); (3) `G` is the OR of the conditions of the remaining policies whose `roles` include one of the user's roles. If the dataset has no policy with `roles`, `G` is TRUE. |
| E4-6.3 | **Exemption.** An exemption removes only that policy. If the user is exempt from **every** role-scoped policy of the dataset, `G` is TRUE. |
| E4-6.4 | **No grant.** Otherwise, if no remaining role-scoped policy lists one of the user's roles, `G` is the constant `FALSE`: the user sees no rows. Access is denied by a false predicate, never by an error that reveals data exists. |
| E4-6.5 | **Propagation.** A policy on dataset X also restricts every dataset that reaches X through many-to-one steps, transitively: a row is visible only if its related row in X exists and is visible. A row with a NULL key, or without a match in X, is not visible, because it cannot be proven allowed. |
| E4-6.6 | **Several relationships to X.** The policy propagates through every relationship that reaches X; the row must be visible through each. |
| E4-6.7 | **Dimension listing.** Dimension-only queries are governed by the dimension datasets' own access and policies. Policies of metric home datasets do not restrict them. |
| E4-6.8 | **Context modifiers never remove policies.** A modifier that removes a dimension or filter from a metric's context (E4-8) applies only to query filters and dimensions, never to row policies. |
| E4-6.9 | Attribute values are bound parameters (SEC-3.5). A policy needing an absent attribute fails compilation (`ESEC-02`) only when it is **applicable**: it remains after exemption (E4-6.2 step 1) and either has no `roles` or lists one of the user's roles. A policy the user is exempt from, or one that grants none of the user's roles, never fails. `ESEC-02` is located at `/attributes` of the security context, and its message names only the missing attribute, never the policy, its dataset or its condition (PLN-5.2, SCY-4a.1). Datasets are visited in branch order (E3-4.3), home dataset first, their policies in CON-5 order of their names (E0H-1.4), and each condition's attributes in order of appearance; the first missing attribute is reported. |
| E4-6.9a | **Empty list attribute.** `x IN (USER_ATTRIBUTE('a'))` is FALSE and `x NOT IN (USER_ATTRIBUTE('a'))` is TRUE when `a` is an empty list, whatever `x` is, NULL included, as SQL `= ANY` over an empty set. |
| E4-6.9b | **Now in a row policy.** An applicable policy (E4-6.9) whose condition uses `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP` needs `reference_time` (QRY-2). Without it, compilation fails with `EQRY-03`, located at `/query`, its message naming neither the policy nor its dataset (SCY-4a.1). Policies are visited as in E4-6.9, after every `ESEC-02` check. |
| E4-6.10 | Joins added only for propagation never add output columns, and the explanation names the policies without showing any value. |

## E4-7 Additivity

Semi-additive metrics (balances, inventory) are planned as defined by the
[additivity extension](../extensions/additivity.md), using the population extension where
declared. Non-additive metrics are always computed from base rows at the query grain.

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
| `E4-03` | Query grain of a semi-additive `over` finer than its `snapshot_grain` |
