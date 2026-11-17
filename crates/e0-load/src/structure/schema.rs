//! The official Apache Ossie 0.1.1 JSON Schema (E0B-1.1), embedded at compile time with the
//! record of where it comes from.

/// Where an embedded JSON Schema was copied from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaSource {
    /// The repository.
    pub repository: &'static str,
    /// The tag the file was read at.
    pub tag: &'static str,
    /// The commit the tag names.
    pub commit: &'static str,
    /// The path of the file inside the repository.
    pub path: &'static str,
}

/// The source of the official 0.1.1 schema (E0B-1.1, OSS table of section 4.1).
pub const OFFICIAL_SCHEMA_SOURCE: SchemaSource = SchemaSource {
    repository: "https://github.com/apache/ossie",
    tag: "osi-0.1.1-rc1",
    commit: "faf581054dcf7964d5fe0ceae7d6f415c8ce32a5",
    path: "core-spec/osi-schema.json",
};

/// The text of the official 0.1.1 schema, byte for byte as published.
pub(crate) const OFFICIAL_SCHEMA_TEXT: &str =
    include_str!("../../schemas/ossie-0.1.1/osi-schema.json");
