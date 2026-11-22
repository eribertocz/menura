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
- **E5-2.2** The output plan uses only the nodes of E4-3, so E6 needs no knowledge of which rewrites ran. Its records of applied and rejected rewrites (E5-3.8, E5-4.4) are part of E5's output; before 0.7.0 the compiled plan's `explanation` stays empty (VS-5).
- **E5-2.3** E5 is deterministic (P3): the same plan, catalog state and security context always give the same output plan and records, whatever the order of the input documents' keys.

## E5-3 Rewrites

Applied in this order; each runs on the output of the previous one.

| ID | Rewrite | Condition |
|---|---|---|
| E5-3.1 | **Remove unused joins.** Drop a `Join` whose dataset contributes no column, filter or policy. | Always: with `Deduplicate` in place (E4-4 step 5), an unused `left` join cannot change any component. |
| E5-3.2 | **Skip deduplication.** Remove a `Deduplicate` node. | Passing `key_uniqueness` evidence for every relationship that caused it (CAT-7.3). |
| E5-3.3 | **Inner joins.** Change a `left` join to `inner`. | Passing `orphan_keys` evidence for the relationship, and no NULL value possible in `from_columns`. |
| E5-3.4 | **Merge scans.** Evaluate components of different branches that read the same home dataset with the same joins and filters in one `Aggregate`. | The branches are identical except for their components. |
| E5-3.5 | **Use a summary.** Replace the `Scan` of a home dataset (and the joins it makes redundant) with a summary dataset. | All conditions of the [summaries extension](../extensions/summaries.md): equivalence, grain, policy preservation and freshness (CAT-7.2). |
| E5-3.6 | **Fold constants.** A branch whose home dataset's row-policy filter is the constant `FALSE` becomes an empty input: a branch with `Aggregate` keys yields no rows, and one without keys yields one row of its components' empty-input values (E4-5.0b), as the base plan does over no rows. A joined dataset whose filter is `FALSE` is not folded: its home rows fall into the NULL group (E4-4.1). | Always. |

- **E5-3.1a** **Unused join.** A `Join` is unused when no later node reads a column of its joined dataset: no `Aggregate` key or component, no query filter, no `date_range` predicate, no `order_by` column and no condition of another `Join`. Joins are examined from the last one in path order backwards, so a chain of unused joins is removed whole. When the last `Join` of a branch is removed, its `Deduplicate` is removed too, because E4-4.0e inserts one only with a `Join`. `SemiJoin` nodes of row policies are never removed.
- **E5-3.2a** **Deduplication evidence.** A `Deduplicate` is removed only when every `Join` of the branch follows its relationship from `from` to `to` and has `key_uniqueness` evidence for that relationship (CAT-7.3). Each joined `Scan` reads a subset of `to` (E4-6.5c), so uniqueness over all of `to` holds for it.
- **E5-3.3a** **Inner-join conditions.** A `left` join becomes `inner` only when, in addition to `orphan_keys` evidence: (1) every `from_columns` column has `nullable: false` in the catalog schema; (2) the `from` side is the home `Scan` or a dataset joined by an `inner` join; and (3) the joined `Scan` reads every row of `to` for this user: no applicable policy of `to` (E4-6.2) and no propagation `SemiJoin` filters it, and `G` is not `FALSE`. Otherwise a home row whose match is not visible would be dropped instead of falling into the NULL group (E4-4.1).
- **E5-3.4a** **Identical branches.** Two branches are identical except for their components when they have the same home `Scan` with the same row-policy nodes, the same `Join` nodes in the same order, the same query filter, the same presence of `Deduplicate` and the same `Aggregate` keys. A branch with a `Population`, `Snapshot` or `Periods` node is never merged. The merged branch keeps the position of the first of them (E3-4.3), and its components keep their names, so the stitched result is unchanged. When one branch remains, the `Stitch` is removed (E4-4.2).
- **E5-3.5a** **Summary branch.** Using summary S for a branch replaces its home `Scan` with a `Scan` of S and maps every reference to a base field to the summary field of the dimension with that `base`. The branch then has, in order: the `Filter` of the base's applicable row policies and the propagation `SemiJoin` nodes, over the mapped fields (SUM-3.6); the branch's `Join` nodes, from the mapped `from_columns`; the query filter and `date_range` predicate, mapped; and the `Aggregate`. A branch that still has a `Deduplicate` after E5-3.2 never uses a summary (SUM-3.2), so the summary branch has none; its `Aggregate` has the recomposed components of SUM-3.1. The data versions of the plan are those of PLN-3.3.
- **E5-3.7** Row-policy filters are never removed, weakened or reordered above an aggregate (E4-6.1). A summary is used only if every applicable policy can be applied to it exactly.
- **E5-3.8** Every rewrite applied, and every rewrite considered and rejected with its failed condition, is recorded in the explanation.

## E5-4 Summary selection

1. Discard every summary that fails any condition of E5-3.5.
2. Among the rest, choose the one with the smallest `row_count_estimate`; break ties by the summary's normalized name (P3).
3. In version 1 a summary is used for the whole queried range or not at all.
4. Selection is done for each branch separately, after E5-3.1 to E5-3.4; E5-3.6 then runs on the result.
5. Plans that use a summary are marked "requires authorized executor" in the explanation, because summaries are `access: internal` (summaries extension).

## E5-5 Never

- Change a result to make a plan cheaper.
- Use the row estimate to decide correctness.
- Assume referential integrity, key uniqueness or summary freshness without evidence.
