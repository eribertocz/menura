# 0005 Summary evidence, build time zone and the E5 audit

## Context

Issues #92, #93 and #94, raised by the data pipeline specification, asked how a summary's time
zone is chosen, whether a summary can be coarser than its base partitions, and what a pipeline
reports for a check that cannot run. The 0.6.0 audit of E5 also found that an inner join can
drop a home row whose match is hidden by a row policy, that a truncated summary field cannot
serve a filter, and that `COUNT` recomposed by `SUM` changes type on DuckDB.

## Decision

- The compiler generates each summary's build query, from the same plan as its check. Both
  take the time zone as a bound parameter chosen by the pipeline and reported in
  `aggregate_build.time_zone`. The zone matters only when a time dimension of the summary reads
  a `timestamp` with time zone (CHK-5, CAT-7.2 condition 5). The build query's contract fields
  come with `integrity_checks` in 0.7.0.
- A summary may be coarser than its base partitions when its grain derives from theirs by exact
  grouping. `built_from` and `verified` then list base periods, and every base period inside the
  used summary periods must match exactly (CAT-7.2, CAT-8.8). A partition column that is a
  `timestamp` with time zone gives no evidence, because partition periods carry no time zone.
- A check that did not complete has no result: the pipeline keeps the last completed result or
  omits it (CAT-7.1.6, CHK-2.3). The compiler needs no change: evidence is tied to versions.
- E5: inner joins also need non-nullable keys, an inner or home `from` side, and a joined scan
  that no policy filters (E5-3.3a). A summary field with a grain serves only time grains and
  aligned date ranges (SUM-3.3a). Summaries whose values depend on the build's time or zone,
  and nullable partition columns without a date range, are not used (SUM-3.2a, CAT-7.2). Recomposed components keep the base type (SUM-3.8, E6-4.15).
  E5's records wait for `explanation` in 0.7.0 (E5-2.2). The 0.6.0 slice is unchanged (VS-1d).

## Consequences

Summaries never change a result or a type, and fail safe to the base relation. Catalog states
that were valid stay valid; states with base-grain `built_from` periods become valid.

## Rules changed

CAT-7.1.6, CAT-7.1.7, CAT-7.1.8, CAT-7.1.9, CAT-7.2, CAT-7.2.1, CAT-8.6, CAT-8.8, CHK-2.3, CHK-5.1–CHK-5.4,
E5-2.2, E5-2.3, E5-3.1a, E5-3.2a, E5-3.3a, E5-3.4a, E5-3.5a, E5-4, SUM-3.2, SUM-3.2a, SUM-3.3, SUM-3.3a, SUM-3.5, SUM-3.8,
E6-4.15, E4-7.9, VS-1d.
