# E5 Optimize

## E5-1 Purpose

Rewrite the logical plan into an equivalent plan that is cheaper to run. Every rewrite MUST
preserve the result exactly, including NULL groups, types and row policies. If a rewrite cannot
be proven safe from the model and the catalog state, it is not applied.

## E5-2 Input and output

| | Content |
|---|---|
| Input | Logical plan (E4), catalog state (optional), security context |
| Output | Equivalent logical plan, with every rewrite recorded in the explanation |

- **E5-2.1** A missing or rejected catalog state is not an error: E5 then applies only rewrites that need no evidence.

## E5-3 Rewrites

Applied in this order; each runs on the output of the previous one.

| ID | Rewrite | Condition |
|---|---|---|
| E5-3.1 | **Remove unused joins.** Drop a `Join` whose dataset contributes no column, filter or policy. | Always: with `Deduplicate` in place (E4-4 step 5), an unused `left` join cannot change any component. |
| E5-3.2 | **Skip deduplication.** Remove a `Deduplicate` node. | Passing `key_uniqueness` evidence for every relationship that caused it (CAT-7.3). |
| E5-3.3 | **Inner joins.** Change a `left` join to `inner`. | Passing `orphan_keys` evidence for the relationship, and no NULL value possible in `from_columns`. |
| E5-3.4 | **Merge scans.** Evaluate components of different branches that read the same home dataset with the same joins and filters in one `Aggregate`. | The branches are identical except for their components. |
| E5-3.5 | **Use a summary.** Replace the `Scan` of a home dataset (and the joins it makes redundant) with a summary dataset. | All conditions of the [summaries extension](../extensions/summaries.md): equivalence, grain, policy preservation and freshness (CAT-7.2). |
| E5-3.6 | **Fold constants.** A branch whose row-policy filter is the constant `FALSE` becomes an empty input, and its components take their empty-input values. | Always. |

- **E5-3.7** Row-policy filters are never removed, weakened or reordered above an aggregate (E4-6.1). A summary is used only if every applicable policy can be applied to it exactly.
- **E5-3.8** Every rewrite applied, and every rewrite considered and rejected with its failed condition, is recorded in the explanation.

## E5-4 Summary selection

1. Discard every summary that fails any condition of E5-3.5.
2. Among the rest, choose the one with the smallest `row_count_estimate`; break ties by the summary's normalized name (P3).
3. In version 1 a summary is used for the whole queried range or not at all.
4. Plans that use a summary are marked "requires authorized executor" in the explanation, because summaries are `access: internal` (summaries extension).

## E5-5 Never

- Change a result to make a plan cheaper.
- Use the row estimate to decide correctness.
- Assume referential integrity, key uniqueness or summary freshness without evidence.
