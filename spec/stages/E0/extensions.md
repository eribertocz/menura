# E0.c Validate extensions

## E0C-1 Which entries are ours

An entry of `custom_extensions` is a compiler extension only if all of these hold:

1. `vendor_name` is `COMMON`;
2. `data` parses as a JSON object;
3. `data.namespace` equals the configured namespace (default `semantic_compiler`).

- **E0C-1.1** An entry that fails 1 or 3 belongs to someone else: it is kept and never interpreted.
- **E0C-1.2** A `COMMON` entry whose `data` is not valid JSON is kept, not interpreted, and produces `W0C-01`.
- **E0C-1.3** If `data` repeats the member `namespace`, condition 3 holds when **any** of its values equals the configured namespace; the entry is then ours and rejected with `E0C-06` (E0C-2.7). A document with one reading as ours is never silently ignored (P1).

## E0C-2 Rules for our entries

| ID | Rule |
|---|---|
| E0C-2.1 | `data.version` MUST be present and be the JSON integer `1`. Any other value, of any JSON type (`"1"`, `1.0`, `null`), is `E0C-01`. |
| E0C-2.2 | An object has at most one entry of ours. |
| E0C-2.3 | Each extension key is valid only on the object type listed in the [model contract](../../contracts/model.md) (MDL-2). |
| E0C-2.4 | Every key inside `data` MUST be defined by the extension's schema; unknown keys are errors. |
| E0C-2.5 | Each extension is then validated by its own chapter in [`spec/extensions/`](../../extensions/README.md). |
| E0C-2.6 | `data` is a JSON string, so no pointer reaches inside it: every E0C diagnostic about an entry is located at that entry's `data` member, except `E0C-04`, which is located at the entry that repeats (the later one). |
| E0C-2.7 | A member name repeated in any object inside `data` is an error (`E0C-06`), as a repeated YAML key is (E0A-2.3): a document with two readings violates P1. |
| E0C-2.8 | In the development format (MDL-2.4), `data` is a YAML object, read as the JSON value it represents, so every rule of E0C applies unchanged. A key that is not a string, a number with no JSON form (`.inf`, `.nan`) or a YAML value with no JSON form is `E0C-03`, located at the entry's `data` member. An integer is read as the release format's JSON reader reads the same digits, so both formats give the same code. A YAML `null` is the JSON `null`, not the string `"null"`. |

## E0C-3 Diagnostics

| Code | Condition |
|---|---|
| `E0C-01` | `data.version` missing, or not the integer `1` |
| `E0C-02` | Unknown key in an extension of ours |
| `E0C-03` | Wrong value type of an extension key of ours |
| `E0C-04` | More than one entry of ours on the same object |
| `E0C-05` | Extension key on the wrong object type |
| `E0C-06` | Member name repeated inside `data` of an entry of ours |
| `W0C-01` | `COMMON` entry whose `data` is not valid JSON; kept, not interpreted |

Extension-specific codes are defined in each extension chapter.
