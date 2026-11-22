"""Type stubs of the Python package `menura` (ARC-6.3 to ARC-6.5a).

Documents are `bytes` passed unchanged, or `str` encoded as UTF-8. Options are `dict` objects
read as the CLI reads its option files; an invalid one raises `ValueError`. An optional argument
passed as `None` is absent.
"""

from typing import Any, Mapping, Optional, Union, final

Document = Union[bytes, str]
Options = Mapping[str, Any]

@final
class Result:
    """The document of a call, as the CLI writes it without its final line feed."""

    @property
    def document(self) -> str: ...
    @property
    def is_error(self) -> bool:
        """True exactly when the CLI would exit 1."""
    @property
    def audit_record(self) -> Optional[str]:
        """The audit record of `Handle.compile` with `audit: true`, otherwise `None`."""

class CompileError(Exception):
    """Raised by `Handle.load` when the model does not load."""

    document: str

@final
class Handle:
    """An immutable loaded model, which may be shared between threads."""

    @property
    def warnings(self) -> str:
        """The JSON text of the load warnings, as the `warnings` member of a compiled plan."""
    @staticmethod
    def load(
        model: Document,
        catalog_schema: Document,
        catalog_state: Optional[Document] = None,
        load_options: Optional[Options] = None,
    ) -> Handle: ...
    def with_state(self, catalog_state: Document) -> Handle: ...
    def compile(
        self,
        query: Document,
        security_context: Document,
        compile_options: Optional[Options] = None,
    ) -> Result: ...
    def explain(
        self,
        query: Document,
        security_context: Document,
        compile_options: Optional[Options] = None,
    ) -> Result: ...
    def describe(self, security_context: Document) -> Result: ...
    def integrity_checks(self, dialect: Optional[str] = "duckdb") -> Result: ...
    def dependencies(self) -> Result: ...

def validate(
    model: Document,
    catalog_schema: Document,
    load_options: Optional[Options] = None,
) -> Result: ...
def migrate(model: Document, load_options: Optional[Options] = None) -> Result: ...
def diff(
    old_model: Document,
    old_catalog_schema: Document,
    new_model: Document,
    new_catalog_schema: Document,
    load_options: Optional[Options] = None,
) -> Result: ...
