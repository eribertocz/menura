# Validity: range shape and temporal relationships

Rules for history tables, whose rows are versions of an entity with a validity range (slowly
changing dimensions of type 2). VAL-1 lets a `validity` population (POP-2) declare a half-open
range and a sentinel end. Its rules join the coverage scope at the gate of the release that
implements them (7.1a).

## VAL-1 Range shape of a `validity` population

| ID | Rule |
|---|---|
| VAL-1.1 | **Exclusive end.** A `validity` population may declare `valid_to_bound`: `inclusive` (the default, the meaning of POP-2) or `exclusive`. With `exclusive`, a row covers the values t of the type of `valid_from` with `valid_from` ≤ t < `valid_to`: instants for a `timestamp`, days for a `date`. |
| VAL-1.2 | **Open end.** A `validity` population may declare `open_end`: one value of the logical type of `valid_to`, written as CON-4 encodes that type (`9999-12-31` for a `date`, `9999-12-31T00:00:00Z` for a `timestamp` with time zone). A row whose `valid_to` equals `open_end`, compared as values of that type, is open-ended exactly as a row whose `valid_to` is NULL, in every rule that reads the population (POP-3.5, CHK-6.3, VAL-1.4). |
| VAL-1.3 | **Days of an exclusive end.** Where POP-3.5 compares a row's days with a period, the last day of a row with an exclusive `valid_to` v is: for a `date`, the day before v; for a `timestamp`, the day of v in the query time zone (POP-3.5), or the day before it when v is the first instant of that day. Its first day is the day of `valid_from`, as in POP-3.5. Every other part of POP-3.5 applies unchanged. |
| VAL-1.4 | **Integrity check of an exclusive range.** For a population with `valid_to_bound: exclusive`, CHK-6.3 compares values, not days: a row is `invalid_interval` when `valid_from` and `valid_to` are non-NULL, `valid_to` is not `open_end`, and `valid_from` ≥ `valid_to`; two rows overlap when each one's `valid_from` is before the other's `valid_to`, an open-ended `valid_to` being after every value. The check then computes no day, so it has no `time_zone` slot (CHK-7.3). Every other part of CHK-6.3 applies unchanged. |
| VAL-1.5 | **Validation.** `valid_to_bound` and `open_end` are keys of the `validity` mode: on a `periodic` population they are `E0C-21` (POP-4.1). A `valid_to_bound` other than `inclusive` or `exclusive`, or an `open_end` that is not a string, is `E0C-03` (POP-4.2). An `open_end` that is not a value of the logical type of `valid_to` in its CON-4 form is `E0C-66`, located at `open_end` and checked in E0.f only when `valid_to` has a type (POP-4.5). |

## VAL-3 Diagnostics

| Code | Condition |
|---|---|
| `E0C-66` | `open_end` is not a value of the type of `valid_to` in its CON-4 form |
