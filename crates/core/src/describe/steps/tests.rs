use menura_model::internal_model::NormalizedName;

use super::{Path, Steps};

fn names(texts: &[&str]) -> Vec<NormalizedName> {
    texts.iter().map(|text| NormalizedName::new(text)).collect()
}

fn relationships(path: &Path<'_>) -> Vec<String> {
    path.iter()
        .map(|(relationship, _)| relationship.as_str().to_owned())
        .collect()
}

/// `count` diamonds in a chain: `D<i>` reaches `D<i+1>` through `A<i>` and through `B<i>`,
/// so `D0` has 2^count paths to `D<count>`.
fn diamonds(count: usize) -> (Vec<NormalizedName>, Vec<NormalizedName>) {
    let mut datasets = Vec::new();
    let mut edges = Vec::new();
    for index in 0..count {
        let next = index + 1;
        datasets.extend(names(&[&format!("A{index:02}"), &format!("B{index:02}")]));
        edges.extend(names(&[
            &format!("D{index:02}_A"),
            &format!("D{index:02}"),
            &format!("A{index:02}"),
            &format!("A{index:02}_D"),
            &format!("A{index:02}"),
            &format!("D{next:02}"),
            &format!("D{index:02}_B"),
            &format!("D{index:02}"),
            &format!("B{index:02}"),
            &format!("B{index:02}_D"),
            &format!("B{index:02}"),
            &format!("D{next:02}"),
        ]));
    }
    (datasets, edges)
}

fn graph(edges: &[NormalizedName]) -> Steps<'_> {
    Steps::new(edges.chunks(3).map(|edge| (&edge[0], &edge[1], &edge[2])))
}

#[test]
fn dsc_3_9_a_chain_of_diamonds_gives_its_two_smallest_paths_without_enumerating_them() {
    let (_, edges) = diamonds(40);
    let steps = graph(&edges);
    let from = NormalizedName::new("D00");
    let to = NormalizedName::new("D40");
    let found = steps.two_smallest(&from, &to);
    assert_eq!(found.len(), 2);
    let mut first: Vec<String> = Vec::new();
    for index in 0..40 {
        first.extend([format!("D{index:02}_A"), format!("A{index:02}_D")]);
    }
    let mut second = first.clone();
    second[78] = "D39_B".to_owned();
    second[79] = "B39_D".to_owned();
    assert_eq!(relationships(&found[0]), first);
    assert_eq!(relationships(&found[1]), second);
}

#[test]
fn dsc_3_9_shorter_paths_come_first_then_relationship_names() {
    let edges = names(&[
        "Z_DIRECT",
        "ORDERS",
        "STORES", //
        "A_CUSTOMERS",
        "ORDERS",
        "CUSTOMERS", //
        "B_STORES",
        "CUSTOMERS",
        "STORES", //
        "Y_DIRECT",
        "ORDERS",
        "STORES",
    ]);
    let steps = graph(&edges);
    let found = steps.two_smallest(&edges[1], &edges[2]);
    assert_eq!(relationships(&found[0]), ["Y_DIRECT"]);
    assert_eq!(relationships(&found[1]), ["Z_DIRECT"]);
}

#[test]
fn e0e_2_4_a_path_never_repeats_a_dataset() {
    let edges = names(&[
        "AB", "A", "B", //
        "BA", "B", "A", //
        "BC", "B", "C",
    ]);
    let steps = graph(&edges);
    let found = steps.two_smallest(&edges[1], &edges[8]);
    assert_eq!(found.len(), 1);
    assert_eq!(relationships(&found[0]), ["AB", "BC"]);
}

#[test]
fn dsc_3_9_the_second_path_avoids_the_datasets_before_its_deviation() {
    let edges = names(&[
        "A1", "S", "X", //
        "A2", "X", "T", //
        "B1", "S", "Y", //
        "B2", "Y", "X",
    ]);
    let steps = graph(&edges);
    let found = steps.two_smallest(&edges[1], &edges[5]);
    assert_eq!(relationships(&found[0]), ["A1", "A2"]);
    assert_eq!(relationships(&found[1]), ["B1", "B2", "A2"]);
}

#[test]
fn e3_3_1_a_dataset_reaches_itself_and_what_its_steps_lead_to() {
    let edges = names(&["AB", "A", "B", "CD", "C", "D"]);
    let steps = graph(&edges);
    let reached: Vec<&str> = steps
        .reached_from(&edges[1])
        .into_iter()
        .map(NormalizedName::as_str)
        .collect();
    assert_eq!(reached, ["A", "B"]);
}
