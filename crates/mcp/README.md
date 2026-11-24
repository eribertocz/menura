# menura-mcp

`menura-mcp` is the MCP server of Menura (ARC-6.1): a binary that speaks JSON-RPC 2.0 and MCP
revision `2025-06-18` over standard input and output, one message per line. It is a wrapper
like `menura`, the CLI: it has no logic of its own, depends only on `menura-core`, writes
nothing but protocol messages to standard output and opens no file or network connection.
Every document arrives as an argument.

It serves modelers and developers, and may show the whole compiled plan, SQL, parameters and
policy conditions included. End users and their agents query through the platform's semantic
service instead (ARC-6.2).

## Methods

- `initialize` answers `protocolVersion` `2025-06-18` whatever the client asks for,
  `serverInfo` `{"name": "menura-mcp", "version": <Menura version>}` and `capabilities`
  `{"tools": {"listChanged": false}}` (ARC-6.6).
- `ping` answers `{}`; `tools/list` answers the tools below; `tools/call` runs one.
- Notifications, such as `notifications/initialized` and `notifications/cancelled`, are not
  answered. Every call is answered on its own: no result depends on the order of calls.

## Results and errors

A tool returns one text content item, the exact text `menura` writes to standard output for the
same command without its final line feed, and `isError: true` exactly when `menura` would exit
`1`. When the compile option `audit` is true and `compile` ran, the audit record follows as a
second text content item; a model that does not load gives no record (ARC-6.8).

An unknown tool, arguments that are not an object, a missing or unknown member, a document of
the wrong type or `null`, or any argument `menura` would refuse with a usage error is a JSON-RPC
error `-32602`. Its message is the usage reason of `core` with member names (`model` for
`--model`), without the CLI's usage text; an unknown tool gives ``unknown tool `<name>` ``
(ARC-6.7). An options member of any type is read as the CLI reads an options file, so a value
that is not an object gives the CLI's reason. `compile` has no `audit_record` member: it returns
the record instead.

## Tools

One tool per CLI command (ARC-6.7). Documents are members of type `string` holding the file's
text; options are members of type `object`.

| Tool | Required members | Optional members | Description |
|---|---|---|---|
| `validate` | `model`, `catalog_schema` | `load_options` | Validates an Apache Ossie model against a catalog schema and returns the validation report. |
| `compile` | `model`, `catalog_schema`, `query`, `security_context` | `catalog_state`, `compile_options`, `load_options` | Compiles a query and returns the compiled plan, or the compile error; with the compile option `audit: true`, the audit record follows. |
| `explain` | `model`, `catalog_schema`, `query`, `security_context` | `catalog_state`, `compile_options`, `load_options` | Returns the explanation of a query: the compiled plan without SQL or parameters, or the compile error. |
| `describe` | `model`, `catalog_schema`, `security_context` | `catalog_state`, `load_options` | Returns the model description a security context sees, or the compile error. |
| `integrity_checks` | `model`, `catalog_schema` | `dialect`, `load_options` | Returns the integrity-check document of a dialect (`clickhouse` or `duckdb`, the default), or the compile error. |
| `dependencies` | `model`, `catalog_schema` | `load_options` | Returns the model-dependency document of a model, or the compile error. |
| `migrate` | `model` | `load_options` | Rewrites the deprecated forms of a model and returns the migration report, or the compile error; it takes no catalog. |
| `diff` | `old_model`, `old_catalog_schema`, `new_model`, `new_catalog_schema` | `load_options` | Loads two models with the same load options, old first, and returns the diff of their dependencies, or the compile error. |

The exact listing `tools/list` returns, names, input schemas and descriptions (ARC-6.9); a test
keeps it equal to the server's:

```json
{
  "tools": [
    {
      "description": "Validates an Apache Ossie model against a catalog schema and returns the validation report.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema"
        ],
        "type": "object"
      },
      "name": "validate"
    },
    {
      "description": "Compiles a query and returns the compiled plan, or the compile error; with the compile option `audit: true`, the audit record follows.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "catalog_state": {
            "description": "The catalog state document (JSON) as text.",
            "type": "string"
          },
          "compile_options": {
            "description": "The compile options; an omitted option takes its default.",
            "type": "object"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          },
          "query": {
            "description": "The query document (JSON) as text.",
            "type": "string"
          },
          "security_context": {
            "description": "The security context document (JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema",
          "query",
          "security_context"
        ],
        "type": "object"
      },
      "name": "compile"
    },
    {
      "description": "Returns the explanation of a query: the compiled plan without SQL or parameters, or the compile error.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "catalog_state": {
            "description": "The catalog state document (JSON) as text.",
            "type": "string"
          },
          "compile_options": {
            "description": "The compile options; an omitted option takes its default.",
            "type": "object"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          },
          "query": {
            "description": "The query document (JSON) as text.",
            "type": "string"
          },
          "security_context": {
            "description": "The security context document (JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema",
          "query",
          "security_context"
        ],
        "type": "object"
      },
      "name": "explain"
    },
    {
      "description": "Returns the model description a security context sees, or the compile error.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "catalog_state": {
            "description": "The catalog state document (JSON) as text.",
            "type": "string"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          },
          "security_context": {
            "description": "The security context document (JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema",
          "security_context"
        ],
        "type": "object"
      },
      "name": "describe"
    },
    {
      "description": "Returns the integrity-check document of a dialect (`clickhouse` or `duckdb`, the default), or the compile error.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "dialect": {
            "description": "The dialect, `clickhouse` or `duckdb` (the default).",
            "type": "string"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema"
        ],
        "type": "object"
      },
      "name": "integrity_checks"
    },
    {
      "description": "Returns the model-dependency document of a model, or the compile error.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "catalog_schema": {
            "description": "The catalog schema document (JSON) as text.",
            "type": "string"
          },
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model",
          "catalog_schema"
        ],
        "type": "object"
      },
      "name": "dependencies"
    },
    {
      "description": "Rewrites the deprecated forms of a model and returns the migration report, or the compile error; it takes no catalog.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "model": {
            "description": "The model document (YAML or JSON) as text.",
            "type": "string"
          }
        },
        "required": [
          "model"
        ],
        "type": "object"
      },
      "name": "migrate"
    },
    {
      "description": "Loads two models with the same load options, old first, and returns the diff of their dependencies, or the compile error.",
      "inputSchema": {
        "additionalProperties": false,
        "properties": {
          "load_options": {
            "description": "The load options; an omitted option takes its default.",
            "type": "object"
          },
          "new_catalog_schema": {
            "description": "The catalog schema of the new model as text.",
            "type": "string"
          },
          "new_model": {
            "description": "The new model document as text.",
            "type": "string"
          },
          "old_catalog_schema": {
            "description": "The catalog schema of the old model as text.",
            "type": "string"
          },
          "old_model": {
            "description": "The old model document as text.",
            "type": "string"
          }
        },
        "required": [
          "old_model",
          "old_catalog_schema",
          "new_model",
          "new_catalog_schema"
        ],
        "type": "object"
      },
      "name": "diff"
    }
  ]
}
```
