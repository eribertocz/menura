//! The names of the columns a check adds after its field columns (CHK-2.6).

/// The name of the added column `name` in a result whose field columns are named
/// `field_columns`: `name` itself, or, when a field column has that name, `name` with the
/// suffix `_n`, the smallest `n` from 1 that makes it unique (CHK-2.6).
pub(super) fn added_column_name(name: &str, field_columns: &[String]) -> String {
    let taken = |candidate: &str| field_columns.iter().any(|column| column == candidate);
    if !taken(name) {
        return name.to_owned();
    }
    // Among `field_columns.len() + 1` candidates at least one is free.
    (1..=field_columns.len() + 1)
        .map(|suffix| format!("{name}_{suffix}"))
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::added_column_name;

    fn names(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    #[test]
    fn chk_2_6_keeps_a_name_no_field_column_has() {
        assert_eq!(added_column_name("row_count", &names(&["ID"])), "row_count");
    }

    #[test]
    fn chk_2_6_takes_the_smallest_free_suffix() {
        let fields = names(&["row_count", "row_count_1", "row_count_3"]);
        assert_eq!(added_column_name("row_count", &fields), "row_count_2");
    }
}
