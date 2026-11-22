//! The Levenshtein distance of PLN-5.4 step 3, counted in Unicode code points.

/// The Levenshtein distance between `left` and `right`, in Unicode code points, or `None` when
/// it is greater than `bound`. Stopping at `bound` keeps a long candidate cheap to reject.
pub(crate) fn bounded_distance(left: &str, right: &str, bound: usize) -> Option<usize> {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.len().abs_diff(right.len()) > bound {
        return None;
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_char) in left.iter().enumerate() {
        let mut current = Vec::with_capacity(right.len() + 1);
        current.push(row + 1);
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left_char != right_char);
            let deletion = previous[column + 1] + 1;
            let insertion = current[column] + 1;
            current.push(substitution.min(deletion).min(insertion));
        }
        if current.iter().all(|distance| *distance > bound) {
            return None;
        }
        previous = current;
    }
    previous
        .last()
        .copied()
        .filter(|distance| *distance <= bound)
}

#[cfg(test)]
mod tests {
    use super::bounded_distance;

    #[test]
    fn pln_5_4_counts_edits() {
        assert_eq!(bounded_distance("SALE_AT", "SALE_DATE", 2), Some(2));
        assert_eq!(bounded_distance("SALE_AT", "SOLD_AT", 2), Some(2));
        assert_eq!(bounded_distance("AMMOUNT", "AMOUNT", 2), Some(1));
        assert_eq!(bounded_distance("SAME", "SAME", 2), Some(0));
    }

    #[test]
    fn pln_5_4_rejects_beyond_the_bound() {
        assert_eq!(bounded_distance("SALES", "TARGETS", 2), None);
        assert_eq!(bounded_distance("A", "ABCD", 2), None);
    }

    #[test]
    fn pln_5_4_counts_code_points_not_bytes() {
        assert_eq!(bounded_distance("ÉTÉ", "ETE", 2), Some(2));
        assert_eq!(bounded_distance("日本", "日本語", 2), Some(1));
    }
}
