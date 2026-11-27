# Platforms

Menura is used on servers and on the machines of modelers and developers. This chapter states the
platforms it supports and what stays the same on all of them. Its rules join the coverage scope at
the gate of the first release whose compatibility matrix lists them (CI-4.2, 7.1a).

## PLT-1 Supported platforms

| ID | Rule |
|---|---|
| PLT-1.1 | **Platforms.** The Rust crate `core`, the CLI `menura` and the MCP server `menura-mcp` support Linux x86_64, macOS x86_64, macOS arm64 and Windows x86_64. The Python package supports the platforms of ARC-6.3. Support is claimed only through the compatibility matrix (CI-4.4). |
| PLT-1.2 | **Same results.** For the same input bytes and options, every public function, CLI command and MCP tool returns byte-identical output on every supported platform (P3). |
| PLT-1.3 | **Input bytes.** A document is read as exactly the bytes given. No platform translates line endings or text encoding on input; a carriage return in a document is part of the document. |
| PLT-1.4 | **Output bytes.** The CLI and the MCP server write their output with line feed (`U+000A`) line endings only, never carriage return and line feed, on every platform (ARC-5.4, ARC-6.6). |
| PLT-1.5 | **Paths.** A CLI path argument is passed to the operating system unchanged, in its native form (drive letters and `\` on Windows included, and names outside ASCII). The document read, and therefore the output, does not depend on how its path is written. |
| PLT-1.6 | **Repository files.** Every text file of the repository is stored and checked out with line feed endings on every platform, so golden files compare byte for byte everywhere (CS-2.3, CS-2.10). |
