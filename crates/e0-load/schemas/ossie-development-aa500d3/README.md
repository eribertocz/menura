# Apache Ossie development-line model schema

`ossie-schema.json` is the structure of reference of the development-line format (OSS-5.1,
MDL-1.1), read with the load option `ossie_format: development`. It is copied byte for byte
from the Apache Ossie repository at the tracked commit (`conformance/ossie/upstream.json`), and
embedded in the compiler at compile time by `src/structure/schema.rs`, which records the same
source.

| | |
|---|---|
| Repository | https://github.com/apache/ossie |
| Commit | `aa500d37894c3d62c96cdce4192cbecea4d7f06a` |
| Path | `core-spec/ossie-schema.json` |
| Git blob | `1f60cfada4e351f4ccc03991da0f0ef7110287a8` |
| License | Apache-2.0 (see `conformance/ossie/LICENSE` and `NOTICE`) |

Never edit this file. A new tracked commit is a new folder, added by the change that adopts it.
