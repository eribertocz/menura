//! Unit tests of the path search (E3-3.1, E3-3.2), named after the rule they check.

use menura_model::internal_model::InternalModel;

use super::search::{AuthorizedEdges, PathsFound};
use super::test_support::{graph_model, step, whole_subgraph};
use crate::test_support::name;

fn found(edges: &[(&str, &str, &str)], from: &str, to: &str) -> PathsFound {
    let model: InternalModel = graph_model(Vec::new(), edges);
    let subgraph = whole_subgraph(&model);
    AuthorizedEdges::new(&model.parts().graph, &subgraph).paths(&name(from), &name(to))
}

#[test]
fn e3_3_1_a_step_never_goes_against_the_many_to_one_direction() {
    assert_eq!(found(&[("B_A", "B", "A")], "A", "B"), PathsFound::None);
}

#[test]
fn e3_3_1_a_path_never_repeats_a_dataset_in_a_cycle() {
    let edges = [
        ("A_B", "A", "B"),
        ("B_C", "B", "C"),
        ("C_D", "C", "D"),
        ("D_B", "D", "B"),
    ];

    assert_eq!(
        found(&edges, "A", "C"),
        PathsFound::One(vec![step("A_B", "A", "B"), step("B_C", "B", "C")])
    );
}

#[test]
fn e3_3_1_a_second_path_through_a_dataset_already_on_the_path_does_not_count() {
    let edges = [("A_B", "A", "B"), ("B_C", "B", "C"), ("B_A", "B", "A")];

    assert_eq!(
        found(&edges, "A", "C"),
        PathsFound::One(vec![step("A_B", "A", "B"), step("B_C", "B", "C")])
    );
}

#[test]
fn e3_3_2_two_relationships_between_the_same_datasets_are_two_paths() {
    let edges = [("SHIP_DATE", "A", "B"), ("ORDER_DATE", "A", "B")];

    assert_eq!(found(&edges, "A", "B"), PathsFound::Several);
}

#[test]
fn e3_3_2_paths_that_part_after_the_first_step_are_several() {
    let edges = [
        ("A_B", "A", "B"),
        ("B_C", "B", "C"),
        ("B_D", "B", "D"),
        ("D_C", "D", "C"),
    ];

    assert_eq!(found(&edges, "A", "C"), PathsFound::Several);
}

#[test]
fn e3_3_2_a_second_path_inside_a_cycle_is_found() {
    let edges = [
        ("A_B", "A", "B"),
        ("B_C", "B", "C"),
        ("A_D", "A", "D"),
        ("D_C", "D", "C"),
        ("C_A", "C", "A"),
    ];

    assert_eq!(found(&edges, "A", "C"), PathsFound::Several);
}

#[test]
fn e3_3_2_the_search_stays_polynomial_on_a_dense_cyclic_graph() {
    let datasets: Vec<String> = (0..40).map(|index| format!("N{index:02}")).collect();
    let mut names = Vec::new();
    for from in &datasets {
        for to in &datasets {
            if from != to {
                names.push((format!("{from}_{to}"), from.clone(), to.clone()));
            }
        }
    }
    names.push((
        "N39_TARGET".to_owned(),
        "N39".to_owned(),
        "TARGET".to_owned(),
    ));
    let edges: Vec<(&str, &str, &str)> = names
        .iter()
        .map(|(relationship, from, to)| (relationship.as_str(), from.as_str(), to.as_str()))
        .collect();

    assert_eq!(found(&edges, "N00", "TARGET"), PathsFound::Several);
    assert_eq!(found(&edges, "TARGET", "N00"), PathsFound::None);
}
