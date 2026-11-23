//! The capability profile of the release (DSC-4): a constant, derived from the roadmap row
//! of the slice the release compiles (DSC-4.2).

use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::capabilities::{
    Capabilities, CapabilityLimits, CompilerFunction, OssieSupport, QueryFormName,
    SupportedContract, SupportedExtension,
};
use menura_contracts::{ContractKind, ContractVersion, compare_names};

use crate::compile::InternalError;
use crate::producer::{OSSIE_NORMATIVE, OSSIE_TRACKED_COMMIT, producer};

/// The roadmap row whose slice the release compiles (7.1, VS-3).
const SLICE_VERSION: ContractVersion = ContractVersion {
    major: 0,
    minor: 9,
    patch: 0,
};

/// The extension keys whose elements the slice compiles, each in data version 1 (DSC-4.2).
const EXTENSIONS: [&str; 8] = [
    "access",
    "additivity",
    "aggregate",
    "context",
    "derived",
    "populations",
    "row_access",
    "security_context",
];

/// The longest identifier (E0D-1.2).
const MAX_IDENTIFIER_LENGTH: u64 = 128;

/// The largest decimal precision of a logical type.
const MAX_DECIMAL_PRECISION: u64 = 38;

/// The capability profile of the release (DSC-4): the same for every model and security
/// context (DSC-4.1), every list sorted without duplicates (DSC-4.3).
///
/// # Errors
///
/// An [`InternalError`] when the workspace version is not `MAJOR.MINOR.PATCH`.
pub(super) fn capabilities() -> Result<Capabilities, InternalError> {
    Ok(Capabilities {
        compiler_version: producer()?.compiler_version,
        slice_version: SLICE_VERSION,
        ossie: OssieSupport {
            normative: OSSIE_NORMATIVE.to_owned(),
            tracked_commit: OSSIE_TRACKED_COMMIT.to_owned(),
            formats: vec![OssieFormat::Development, OssieFormat::Release],
        },
        contracts: contracts(),
        dialects: vec!["duckdb".to_owned()],
        query_forms: vec![QueryFormName::Sql, QueryFormName::WideTable],
        extensions: EXTENSIONS
            .iter()
            .map(|key| SupportedExtension {
                key: (*key).to_owned(),
                versions: vec![1],
            })
            .collect(),
        functions: vec![
            CompilerFunction::Compile,
            CompilerFunction::Describe,
            CompilerFunction::Explain,
            CompilerFunction::IntegrityChecks,
            CompilerFunction::Load,
            CompilerFunction::Migrate,
            CompilerFunction::Validate,
            CompilerFunction::WithState,
        ],
        limits: CapabilityLimits {
            max_identifier_length: MAX_IDENTIFIER_LENGTH,
            max_decimal_precision: MAX_DECIMAL_PRECISION,
        },
    })
}

/// Every public contract the release reads or writes, with the latest version it implements,
/// sorted by name (DSC-4.3). The explanation has the schema of the compiled plan and the
/// validation report that of the compile error (architecture section 3.5).
fn contracts() -> Vec<SupportedContract> {
    let mut contracts: Vec<SupportedContract> = ContractKind::ALL
        .iter()
        .map(|kind| supported(kind.contract_name(), *kind))
        .chain([
            supported("menura.explanation", ContractKind::CompiledPlan),
            supported("menura.validation_report", ContractKind::CompileError),
        ])
        .collect();
    contracts.sort_by(|left, right| compare_names(&left.contract, &right.contract));
    contracts
}

fn supported(contract: &str, schema: ContractKind) -> SupportedContract {
    SupportedContract {
        contract: contract.to_owned(),
        version: schema.latest_version(),
    }
}

#[cfg(test)]
mod tests {
    use super::{EXTENSIONS, capabilities};

    #[test]
    fn dsc_4_3_every_list_is_sorted_without_duplicates() {
        let profile = capabilities().unwrap();
        let contracts: Vec<&str> = profile
            .contracts
            .iter()
            .map(|entry| entry.contract.as_str())
            .collect();
        assert!(contracts.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(contracts.len(), 14);
        assert!(contracts.contains(&"menura.migration_report"));
        assert!(EXTENSIONS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn dsc_4_2_the_0_8_0_slice_compiles_both_query_forms_and_the_context_extension() {
        let profile = capabilities().unwrap();
        assert_eq!(profile.query_forms.len(), 2);
        assert!(
            profile
                .extensions
                .iter()
                .any(|entry| entry.key == "context")
        );
    }
}
