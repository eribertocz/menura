//! `WQRY-01` for the former contract name and the former extension namespace (E1-2.4).

use menura_contracts::types::query::{Namespace, QueryContract, QueryDocument, QueryForm};
use menura_contracts::{ContractKind, FORMER_NAMESPACE, NAMESPACE};

use crate::error::ParseWarning;
use crate::pointer;

/// The `WQRY-01` warnings of an admitted query: `/contract` first, then each extension read
/// through the former namespace, in index order.
pub(crate) fn former_name_warnings(document: &QueryDocument) -> Vec<ParseWarning> {
    let contract = (document.contract == QueryContract::FormerQuery).then(|| ParseWarning {
        pointer: "/contract".to_owned(),
        message: format!(
            "the contract name `{}` is deprecated and removed at 1.0.0; use `{}`",
            ContractKind::Query.former_contract_name(),
            ContractKind::Query.contract_name()
        ),
    });
    let extensions = match &document.form {
        QueryForm::WideTable(query) => query.nonportable_extensions.as_deref().unwrap_or_default(),
        QueryForm::Sql(_) => &[],
    };
    let namespaces = extensions
        .iter()
        .enumerate()
        .filter(|(_, extension)| extension.data.namespace == Namespace::FormerName)
        .map(|(index, _)| ParseWarning {
            pointer: pointer::extension_member(index, "namespace"),
            message: format!(
                "the extension namespace `{FORMER_NAMESPACE}` is deprecated and removed at \
                 1.0.0; use `{NAMESPACE}`"
            ),
        });
    contract.into_iter().chain(namespaces).collect()
}
