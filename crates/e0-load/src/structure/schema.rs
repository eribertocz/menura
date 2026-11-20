//! The JSON Schemas of the two model formats, embedded at compile time with the record of where
//! they come from: the official Apache Ossie 0.1.1 schema (E0B-1.1) and the development-line
//! schema at the tracked commit (OSS-5.1).

use menura_contracts::types::audit_record::OssieFormat;

/// Where an embedded JSON Schema was copied from.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the record of where the embedded schema comes from is checked only by the structure tests"
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaSource {
    /// The repository.
    pub repository: &'static str,
    /// The tag the file was read at, when it was read at a tag.
    pub tag: Option<&'static str>,
    /// The commit the file was read at.
    pub commit: &'static str,
    /// The path of the file inside the repository.
    pub path: &'static str,
}

/// The source of the official 0.1.1 schema (E0B-1.1, OSS table of section 4.1).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the record of where the embedded schema comes from is checked only by the structure tests"
    )
)]
pub const OFFICIAL_SCHEMA_SOURCE: SchemaSource = SchemaSource {
    repository: "https://github.com/apache/ossie",
    tag: Some("osi-0.1.1-rc1"),
    commit: "faf581054dcf7964d5fe0ceae7d6f415c8ce32a5",
    path: "core-spec/osi-schema.json",
};

/// The source of the development-line schema, at the tracked commit of
/// `conformance/ossie/upstream.json` (OSS-5.1).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the record of where the embedded schema comes from is checked only by the structure tests"
    )
)]
pub const DEVELOPMENT_SCHEMA_SOURCE: SchemaSource = SchemaSource {
    repository: "https://github.com/apache/ossie",
    tag: None,
    commit: "aa500d37894c3d62c96cdce4192cbecea4d7f06a",
    path: "core-spec/ossie-schema.json",
};

/// The text of the official 0.1.1 schema, byte for byte as published.
pub(crate) const OFFICIAL_SCHEMA_TEXT: &str =
    include_str!("../../schemas/ossie-0.1.1/osi-schema.json");

/// The text of the development-line schema, byte for byte as published.
pub(crate) const DEVELOPMENT_SCHEMA_TEXT: &str =
    include_str!("../../schemas/ossie-development-aa500d3/ossie-schema.json");

/// The text of the structure of reference of `format` (MDL-1.1).
pub(crate) fn schema_text(format: OssieFormat) -> &'static str {
    match format {
        OssieFormat::Release => OFFICIAL_SCHEMA_TEXT,
        OssieFormat::Development => DEVELOPMENT_SCHEMA_TEXT,
    }
}
