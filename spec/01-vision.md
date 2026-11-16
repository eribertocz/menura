# 1. Vision

## 1.1 What it is

A **semantic compiler**: a pure function that receives

1. a semantic model (Apache Ossie document plus extensions),
2. a catalog (what physically exists and its current state),
3. a query (metrics, dimensions, filters, ordering, limit),
4. a security context (who is asking),

and returns a **compiled plan**: parameterized SQL for one target dialect, the result schema,
a cache key, and a human-readable explanation — or a precise error.

## 1.2 What it is not

- **Not a query engine.** It never opens a connection, never reads data, never stores credentials.
- **Not a new language.** Inputs are Apache Ossie, JSON and standard SQL; output is standard SQL.
- **Not a storage format.** Data stays in the warehouse or database the user already owns.
- **Not a user interface.** It is a headless component used by other modules.
- **Not an ETL/ELT tool.** It only reads the catalog state that such tools publish.

## 1.3 Goals

| Goal | Meaning |
|---|---|
| Correct | Every answer is right or is an error; never a silently wrong number. |
| Secure | Deny by default; security filters can never be removed by a query. |
| Simple | Few concepts, few functions, explicit rules. |
| Fast | Compilation in milliseconds; generated SQL lets the database do the heavy work. |
| Deterministic | Same inputs, same output, byte for byte. |
| Long-lived | Standards at every edge; versioned contracts; tracks Apache Ossie closely. |
