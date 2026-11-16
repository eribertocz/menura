# E0.c Validate extensions

## E0C-1 Which entries are ours

An entry of `custom_extensions` is a compiler extension only if all of these hold:

1. `vendor_name` is `COMMON`;
2. `data` parses as a JSON object;
3. `data.namespace` equals the configured namespace (default `semantic_compiler`).

- **E0C-1.1** An entry that fails 1 or 3 belongs to someone else: it is kept and never interpreted.
- **E0C-1.2** A `COMMON` entry whose `data` is not valid JSON is kept, not interpreted, and produces `W0C-01`.

## E0C-2 Rules for our entries

| ID | Rule |
|---|---|
| E0C-2.1 | `data.version` MUST be present and supported (version 1). |
| E0C-2.2 | An object has at most one entry of ours. |
| E0C-2.3 | Each extension key is valid only on the object type listed in the [model contract](../../contracts/model.md) (MDL-2). |
| E0C-2.4 | Every key inside `data` MUST be defined by the extension's schema; unknown keys are errors. |
| E0C-2.5 | Each extension is then validated by its own chapter in [`spec/extensions/`](../../extensions/README.md). |

## E0C-3 Diagnostics

| Code | Condition |
|---|---|
| `E0C-01` | `data.version` missing or not supported |
| `E0C-02` | Unknown key in an extension of ours |
| `E0C-03` | Wrong value type in an extension of ours |
| `E0C-04` | More than one entry of ours on the same object |
| `E0C-05` | Extension key on the wrong object type |
| `W0C-01` | `COMMON` entry whose `data` is not valid JSON; kept, not interpreted |

Extension-specific codes are defined in each extension chapter.
