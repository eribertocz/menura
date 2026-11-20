# 0003 E2 query order and the choice of E_NOT_YET_SUPPORTED

## Context

The 0.3.0 audit of E2 (issue #231) found that the first error within one row of E2-5 depends
on an undefined "query order", while CS-2.9 compares its location and the determinism gate
shuffles JSON key order. VS-2 did not say which capability `E_NOT_YET_SUPPORTED` names, nor
where it is located, and VS-1 did not tie "reaches a dataset with row policies" to E4-6.5.

## Decision

- Query order follows the query tree: the wide-table members in a fixed order, lists in index
  order, references in order of appearance; every row is checked over all elements before the
  next (E2-5.3).
- `E_NOT_YET_SUPPORTED` is located at the first element in query order outside the slice, and
  names the capability whose version comes first in table 7.1 (E2-5.4).
- The propagation test of VS-1 uses many-to-one reachability (E4-6.5) of any dataset that
  declares `row_access`, independent of the user's roles.

## Consequences

The first error and its location are the same for any key order of the input. Whether a query
is in the slice never reveals a policy the user does not hold.

## Rules changed

E2-5.3, E2-5.4 (new), VS-1, VS-2.
