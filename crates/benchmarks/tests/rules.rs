//! One test per rule of section 5.6 (R1–R7), on the generated inputs and the pass conditions.

use std::time::Duration;

use menura_benchmarks::{
    InstructionCounts, MEASURED_COMPILATIONS, ModelInputs, ModelShape, Regression,
    WARM_UP_COMPILATIONS, catalog_schema_document, compile_query, model_document, percentile_99,
    r1_passes, r2_passes, r3_violations, reference_queries, scenario_queries,
    security_context_document, sweep_query, total_instructions,
};

fn model_text(shape: &ModelShape) -> String {
    String::from_utf8_lossy(&model_document(shape)).into_owned()
}

fn count_lines(text: &str, prefix: &str) -> usize {
    text.lines().filter(|line| line.starts_with(prefix)).count()
}

fn count_text(text: &str, part: &str) -> usize {
    text.matches(part).count()
}

#[test]
fn r1_model_has_500_datasets_and_5000_fields_and_loads() {
    let shape = ModelShape::r1();
    let text = model_text(&shape);
    assert_eq!(count_lines(&text, "        source: "), 500);
    assert_eq!(count_lines(&text, "          - name: "), 5000);
    ModelInputs::of(&shape).load().unwrap();
}

#[test]
fn r1_load_passes_under_two_seconds() {
    assert!(r1_passes(Duration::from_millis(1999)));
    assert!(!r1_passes(Duration::from_secs(2)));
}

#[test]
fn r2_compile_passes_under_ten_milliseconds() {
    assert!(r2_passes(Duration::from_micros(9999)));
    assert!(!r2_passes(Duration::from_millis(10)));
}

#[test]
fn r3_doubling_at_most_doubles_the_cost_plus_ten_percent() {
    assert!(r3_violations(&[(1, 100), (2, 220), (4, 484)]).is_empty());
    assert_eq!(r3_violations(&[(1, 100), (2, 221), (4, 300)]), vec![2]);
    let shape = ModelShape::reference();
    let sweep = sweep_query(&shape, 64);
    let text = String::from_utf8(sweep.document).unwrap();
    assert_eq!(count_text(&text, r#"{"metric": "#), 64);
    assert_eq!(count_text(&text, "\"star_030.star_030_attribute_"), 64);
    let small = String::from_utf8_lossy(&sweep_query(&shape, 1).document).into_owned();
    assert!(small.contains("\"star_030.star_030_attribute_01\""));
}

#[test]
fn r4_inputs_are_generated_from_fixed_parameters() {
    for shape in [ModelShape::reference(), ModelShape::r1()] {
        assert_eq!(model_document(&shape), model_document(&shape));
        assert_eq!(
            catalog_schema_document(&shape),
            catalog_schema_document(&shape)
        );
        assert_eq!(reference_queries(&shape), reference_queries(&shape));
    }
}

#[test]
fn r5_reference_model_has_the_listed_shape() {
    let shape = ModelShape::reference();
    let text = model_text(&shape);
    assert_eq!(count_lines(&text, "        source: "), 50);
    assert_eq!(count_lines(&text, "          - name: "), 500);
    let metrics = text.split("    metrics:\n").nth(1).unwrap();
    assert_eq!(count_lines(metrics, "      - name: "), 100);
    assert_eq!(count_text(metrics, "\"derived\""), 20);
    assert_eq!(count_text(metrics, "semi_additive"), 10);
    assert_eq!(count_text(metrics, "\"context\""), 10);
    assert_eq!(count_text(&text, "row_access"), 10);
    assert_eq!(count_text(&text, "\"aggregate\""), 2);
}

#[test]
fn r5_reference_queries_compile_with_one_security_context() {
    let shape = ModelShape::reference();
    let handle = ModelInputs::of(&shape).load().unwrap();
    let context = security_context_document();
    let queries = reference_queries(&shape);
    assert_eq!(queries.len(), 20);
    for query in &scenario_queries() {
        compile_query(&handle, query, &context).unwrap();
    }
}

#[test]
fn r6_percentile_of_200_measured_compilations() {
    assert_eq!((WARM_UP_COMPILATIONS, MEASURED_COMPILATIONS), (20, 200));
    let mut times: Vec<Duration> = (1..=200).rev().map(Duration::from_micros).collect();
    times.swap(0, 150);
    assert_eq!(percentile_99(&times), Duration::from_micros(198));
    assert_eq!(percentile_99(&[]), Duration::ZERO);
}

#[test]
fn r7_counts_more_than_ten_percent_above_the_baseline_fail() {
    let baseline = InstructionCounts(
        [
            ("a".to_owned(), 1000),
            ("b".to_owned(), 1000),
            ("d".to_owned(), 1000),
        ]
        .into(),
    );
    let measured = InstructionCounts(
        [
            ("a".to_owned(), 1100),
            ("b".to_owned(), 1101),
            ("c".to_owned(), 1),
        ]
        .into(),
    );
    assert_eq!(
        measured.regressions(&baseline),
        vec![
            Regression::AboveBaseline {
                scenario: "b".to_owned(),
                baseline: 1000,
                measured: 1101
            },
            Regression::MissingBaseline("c".to_owned()),
            Regression::StaleBaseline("d".to_owned()),
        ]
    );
    assert_eq!(
        InstructionCounts::parse(&baseline.render()).unwrap(),
        baseline
    );
    assert_eq!(total_instructions("events: Ir\ntotals: 42\n"), Some(42));
}

#[test]
fn r7_baseline_counts_every_scenario() {
    let baseline =
        InstructionCounts::parse(include_str!("../../../benchmarks/baseline.json")).unwrap();
    let names: Vec<String> = scenario_queries()
        .into_iter()
        .map(|query| query.name)
        .collect();
    let counted: Vec<String> = baseline.0.keys().cloned().collect();
    let mut expected = names.clone();
    expected.sort();
    assert_eq!(counted, expected);
    assert!(baseline.0.values().all(|count| *count > 0));
}
