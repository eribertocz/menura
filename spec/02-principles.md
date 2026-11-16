# 2. Principles

| ID | Principle | Rule |
|---|---|---|
| P1 | Correct or error | If the compiler cannot prove an answer is correct, it MUST return an error. |
| P2 | Pure function | Compilation MUST NOT perform I/O, read clocks, or access the network. |
| P3 | Deterministic | Identical inputs MUST produce byte-identical outputs. |
| P4 | Standards at the edges | Inputs and outputs MUST use open standards (Apache Ossie, JSON Schema, SQL, Apache Arrow downstream). |
| P5 | Neutral plan | The internal plan MUST be independent of any SQL dialect until generation. |
| P6 | Early errors | Every error MUST be raised at the earliest stage that can detect it. |
| P7 | Explainable | Every plan MUST carry an explanation of each choice made. |
| P8 | Independent stages | Each stage MUST have a defined input and output and be testable alone. |
| P9 | Deny by default | Access requires explicit authorization, followed by row-level rules. Absence of a rule never grants access. |
| P10 | Apache Ossie first | The latest official Apache Ossie release is normative for the model; parts no release contains yet follow the tracked development commit (section 4.1). Every deviation is declared (section 4.6). |
