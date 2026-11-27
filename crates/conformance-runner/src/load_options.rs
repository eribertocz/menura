//! Resolves the load options of a case (CS-2.7) to the options the load call receives, giving
//! each omitted option its default from architecture section 3.5.

use menura_core::LoadOptions;

use crate::case::CaseLoadOptions;

/// The default extension namespace (architecture section 3.5, MDL-2.3).
#[cfg(feature = "duckdb")]
pub(crate) const DEFAULT_NAMESPACE: &str = "menura";

/// The load options of a case: each option the case sets, and the default for each it omits.
///
/// # Example
///
/// ```
/// use menura_conformance_runner::load_options::resolve_load_options;
/// use menura_core::OssieFormat;
///
/// let options = resolve_load_options(None);
/// assert_eq!(options.ossie_format, OssieFormat::Release);
/// assert_eq!(options.namespace, "menura");
/// ```
#[must_use]
pub fn resolve_load_options(case_options: Option<&CaseLoadOptions>) -> LoadOptions {
    let defaults = LoadOptions::default();
    let Some(case_options) = case_options else {
        return defaults;
    };
    LoadOptions {
        grant_authenticated_to_all: case_options
            .grant_authenticated_to_all
            .unwrap_or(defaults.grant_authenticated_to_all),
        ossie_format: case_options.ossie_format.unwrap_or(defaults.ossie_format),
        namespace: case_options.namespace.clone().unwrap_or(defaults.namespace),
        model_name: case_options.model_name.clone(),
        data_source: None,
    }
}

#[cfg(test)]
mod tests {
    use menura_core::OssieFormat;

    use super::*;

    #[test]
    fn cs_2_7_an_omitted_load_options_member_gives_every_default() {
        let options = resolve_load_options(None);
        assert_eq!(
            options,
            LoadOptions {
                grant_authenticated_to_all: false,
                ossie_format: OssieFormat::Release,
                namespace: "menura".to_owned(),
                model_name: None,
                data_source: None,
            }
        );
    }

    #[test]
    fn cs_2_7_an_omitted_option_takes_its_default() {
        let case_options = CaseLoadOptions {
            ossie_format: Some(OssieFormat::Development),
            ..CaseLoadOptions::default()
        };
        let options = resolve_load_options(Some(&case_options));
        assert_eq!(options.ossie_format, OssieFormat::Development);
        assert!(!options.grant_authenticated_to_all);
        assert_eq!(options.namespace, "menura");
        assert_eq!(options.model_name, None);
    }

    #[test]
    fn cs_2_7_every_option_the_case_sets_is_kept() {
        let case_options = CaseLoadOptions {
            ossie_format: Some(OssieFormat::Development),
            model_name: Some("shop".to_owned()),
            namespace: Some("acme".to_owned()),
            grant_authenticated_to_all: Some(true),
        };
        let options = resolve_load_options(Some(&case_options));
        assert_eq!(
            options,
            LoadOptions {
                grant_authenticated_to_all: true,
                ossie_format: OssieFormat::Development,
                namespace: "acme".to_owned(),
                model_name: Some("shop".to_owned()),
                data_source: None,
            }
        );
    }
}
