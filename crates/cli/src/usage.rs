//! The usage text of `menura` (ARC-5.5, ARC-5.10).

/// The usage text `--help` writes and every usage error repeats.
pub(crate) const USAGE: &str =
    "usage: menura validate --model <path> --catalog-schema <path> [--load-options <path>]
       menura compile --model <path> --catalog-schema <path> --query <path>
                      --security-context <path> [--catalog-state <path>] [--load-options <path>]
                      [--compile-options <path>] [--audit-record <path>]
       menura explain --model <path> --catalog-schema <path> --query <path>
                      --security-context <path> [--catalog-state <path>] [--load-options <path>]
                      [--compile-options <path>]
       menura describe --model <path> --catalog-schema <path> --security-context <path>
                       [--catalog-state <path>] [--load-options <path>]
       menura integrity-checks --model <path> --catalog-schema <path> [--load-options <path>]
                               [--dialect <name>]
       menura dependencies --model <path> --catalog-schema <path> [--load-options <path>]
       menura migrate --model <path> [--load-options <path>]
       menura diff --old-model <path> --old-catalog-schema <path> --new-model <path>
                   --new-catalog-schema <path> [--load-options <path>]

`validate` validates an Apache Ossie model against a catalog schema and writes the validation
report. `compile` writes the compiled plan of a query, and `explain` its explanation. `describe`
writes the model description a security context sees. `integrity-checks` writes the
integrity-check document of a dialect. `dependencies` writes the model-dependency document.
`migrate` rewrites the deprecated forms of a model and writes the migration report; it reads no
catalog file. `diff` loads both models with the same load options, old first, and writes the
diff of their dependencies. Each writes its document as JSON on standard output. Exit status: 0
for a document without errors, 1 for a report with errors or a compile error, 2 when the
command cannot run.

  --model <path>               the model document (YAML or JSON)
  --catalog-schema <path>      the catalog schema document (JSON)
  --catalog-state <path>       the catalog state document (JSON)
  --query <path>               the query document (JSON)
  --security-context <path>    the security context document (JSON)
  --old-model <path>           `diff`: the old model document
  --old-catalog-schema <path>  `diff`: the catalog schema of the old model
  --new-model <path>           `diff`: the new model document
  --new-catalog-schema <path>  `diff`: the catalog schema of the new model
  --load-options <path>        a JSON object with load options; an omitted option takes its
                               default
  --compile-options <path>     a JSON object with compile options; an omitted option takes its
                               default
  --audit-record <path>        `compile` with `audit: true`: where the audit record is written
  --dialect <name>             `integrity-checks`: the dialect, `duckdb` (the default)
  --help                       writes this text and exits 0
";
