//! The command-line syntax of ARC-5.5 to ARC-5.10.

use super::*;

fn arguments(text: &[&str]) -> Vec<OsString> {
    text.iter().map(OsString::from).collect()
}

#[test]
fn arc_5_5_validate_reads_its_three_flags_in_any_order() {
    let command = parse(&arguments(&[
        "validate",
        "--load-options",
        "o.json",
        "--catalog-schema",
        "c.json",
        "--model",
        "m.yaml",
    ]))
    .unwrap();
    assert_eq!(
        command,
        Command::Validate(ModelFiles {
            model: PathBuf::from("m.yaml"),
            catalog_schema: PathBuf::from("c.json"),
            load_options: Some(PathBuf::from("o.json")),
        })
    );
}

#[test]
fn arc_5_5_the_load_options_flag_is_optional() {
    let Command::Validate(files) = parse(&arguments(&[
        "validate",
        "--model",
        "m.yaml",
        "--catalog-schema",
        "c.json",
    ]))
    .unwrap() else {
        panic!("validate");
    };
    assert_eq!(files.load_options, None);
}

#[test]
fn arc_5_6_diff_reads_its_five_flags_in_any_order() {
    let command = parse(&arguments(&[
        "diff",
        "--new-catalog-schema",
        "nc.json",
        "--old-model",
        "o.yaml",
        "--new-model",
        "n.yaml",
        "--old-catalog-schema",
        "oc.json",
    ]))
    .unwrap();
    assert_eq!(
        command,
        Command::Diff(DiffFiles {
            old_model: PathBuf::from("o.yaml"),
            old_catalog_schema: PathBuf::from("oc.json"),
            new_model: PathBuf::from("n.yaml"),
            new_catalog_schema: PathBuf::from("nc.json"),
            load_options: None,
        })
    );
}

#[test]
fn arc_5_6_dependencies_takes_the_flags_of_validate() {
    let command = parse(&arguments(&[
        "dependencies",
        "--model",
        "m.yaml",
        "--catalog-schema",
        "c.json",
    ]))
    .unwrap();
    assert!(matches!(command, Command::Dependencies(_)));
    assert!(parse(&arguments(&["dependencies", "--old-model", "m"])).is_err());
}

#[test]
fn arc_5_5_help_wins_over_every_other_argument() {
    assert_eq!(
        parse(&arguments(&["validate", "--x", "--help"])),
        Ok(Command::Help)
    );
}

#[test]
fn arc_5_7_compile_reads_its_eight_flags_and_explain_refuses_audit_record() {
    let line = [
        "--audit-record",
        "r.json",
        "--query",
        "q.json",
        "--model",
        "m.yaml",
        "--compile-options",
        "co.json",
        "--security-context",
        "s.json",
        "--catalog-schema",
        "c.json",
        "--catalog-state",
        "st.json",
        "--load-options",
        "o.json",
    ];
    let mut compile = vec!["compile"];
    compile.extend(line);
    let Command::Compile(files) = parse(&arguments(&compile)).unwrap() else {
        panic!("compile");
    };
    assert_eq!(files.audit_record, Some(PathBuf::from("r.json")));
    assert_eq!(files.query, PathBuf::from("q.json"));
    assert_eq!(files.compile_options, Some(PathBuf::from("co.json")));
    assert_eq!(files.handle.catalog_state, Some(PathBuf::from("st.json")));
    let mut explain = vec!["explain"];
    explain.extend(line);
    assert!(parse(&arguments(&explain)).is_err());
    explain.drain(1..3);
    assert!(matches!(
        parse(&arguments(&explain)),
        Ok(Command::Explain(_))
    ));
}

#[test]
fn arc_5_8_integrity_checks_defaults_to_duckdb_and_rejects_an_unknown_dialect() {
    let base = ["integrity-checks", "--model", "m", "--catalog-schema", "c"];
    let Command::IntegrityChecks(files) = parse(&arguments(&base)).unwrap() else {
        panic!("integrity-checks");
    };
    assert_eq!(files.dialect, Dialect::Duckdb);
    let mut line = base.to_vec();
    line.extend(["--dialect", "duckdb"]);
    assert!(parse(&arguments(&line)).is_ok());
    line.pop();
    line.push("postgres");
    assert!(parse(&arguments(&line)).is_err());
}

#[test]
fn arc_5_9_migrate_takes_only_the_model_and_load_options() {
    assert_eq!(
        parse(&arguments(&["migrate", "--model", "m"])),
        Ok(Command::Migrate(MigrateFiles {
            model: PathBuf::from("m"),
            load_options: None,
        }))
    );
    assert!(
        parse(&arguments(&[
            "migrate",
            "--model",
            "m",
            "--catalog-schema",
            "c"
        ]))
        .is_err()
    );
    assert!(parse(&arguments(&["migrate"])).is_err());
}

#[test]
fn arc_5_8_describe_requires_the_security_context() {
    assert!(
        parse(&arguments(&[
            "describe",
            "--model",
            "m",
            "--catalog-schema",
            "c"
        ]))
        .is_err()
    );
    assert!(matches!(
        parse(&arguments(&[
            "describe",
            "--model",
            "m",
            "--catalog-schema",
            "c",
            "--security-context",
            "s"
        ])),
        Ok(Command::Describe(_))
    ));
}

#[test]
fn arc_5_5_a_command_line_outside_the_syntax_is_a_usage_error() {
    for line in [
        &[][..],
        &["describe-model"][..],
        &["validate", "--model", "m.yaml"][..],
        &[
            "validate",
            "--model",
            "m",
            "--model",
            "m",
            "--catalog-schema",
            "c",
        ][..],
        &[
            "validate",
            "--model",
            "m",
            "--catalog-schema",
            "c",
            "--dialect",
            "d",
        ][..],
        &["validate", "--model", "m", "--catalog-schema", "c", "x"][..],
        &["validate", "--model", "m", "--catalog-schema"][..],
        &["validate", "--model=m", "--catalog-schema", "c"][..],
    ] {
        assert!(parse(&arguments(line)).is_err(), "{line:?}");
    }
}
