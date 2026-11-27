//! PLT-1.3 to PLT-1.5: `menura` reads its documents as exactly the bytes given, from a path
//! passed to the operating system unchanged, and writes line feed endings only, on every
//! platform.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the test runs the command-line wrapper as a process and writes its input files"
)]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use menura_core::{LoadOptions, document_json, validate};

const QUERY: &str = r#"{"contract": "menura.query", "contract_version": "1.1.0",
    "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
const CONTEXT: &str = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
    "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;

/// A model whose lines end with carriage return and line feed, with an error on its third line.
const CRLF_MODEL: &[u8] = b"semantic_model:\r\n  - name: shop\r\n    datasets: [\r\n";

/// A file name outside ASCII (PLT-1.5).
const NON_ASCII_FOLDER: &str = "modèle-ñandú-模型";

fn shop(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/shop")
        .join(file)
}

/// The folder of `test` under the target directory, created empty.
fn folder(test: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("platforms")
        .join(test);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// The file `name` of `folder`, written with `content`.
fn file(folder: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = folder.join(name);
    fs::write(&path, content).unwrap();
    path
}

fn run<I, A>(arguments: I) -> Output
where
    I: IntoIterator<Item = A>,
    A: Into<OsString>,
{
    let arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    Command::new(env!("CARGO_BIN_EXE_menura"))
        .args(arguments)
        .output()
        .unwrap()
}

/// The command lines of every kind of output over the shop model: a validation report, a
/// compiled plan, an explanation, a compile error, a description, the integrity checks, the
/// model dependencies, a migration report and the usage text.
fn command_lines(test: &str) -> Vec<Vec<OsString>> {
    let folder = folder(test);
    let query = file(&folder, "query.json", QUERY.as_bytes());
    let bad_query = file(&folder, "bad-query.json", b"{}");
    let context = file(&folder, "context.json", CONTEXT.as_bytes());
    let model = || -> Vec<OsString> {
        vec![
            "--model".into(),
            shop("model.yaml").into(),
            "--catalog-schema".into(),
            shop("catalog-schema.json").into(),
        ]
    };
    let with = |command: &str, extra: &[&Path]| -> Vec<OsString> {
        let mut line: Vec<OsString> = vec![command.into()];
        line.extend(model());
        for pair in extra.chunks(2) {
            line.push(pair[0].as_os_str().to_owned());
            line.push(pair[1].as_os_str().to_owned());
        }
        line
    };
    let query_flag = Path::new("--query");
    let context_flag = Path::new("--security-context");
    vec![
        with("validate", &[]),
        with("compile", &[query_flag, &query, context_flag, &context]),
        with("explain", &[query_flag, &query, context_flag, &context]),
        with("compile", &[query_flag, &bad_query, context_flag, &context]),
        with("describe", &[context_flag, &context]),
        with("integrity-checks", &[]),
        with("dependencies", &[]),
        vec![
            "migrate".into(),
            "--model".into(),
            shop("model.yaml").into(),
        ],
        vec!["--help".into()],
    ]
}

/// Whether `bytes` ends with a line feed and holds no carriage return.
fn has_line_feeds_only(bytes: &[u8]) -> bool {
    bytes.ends_with(b"\n") && !bytes.contains(&b'\r')
}

#[test]
fn plt_1_4_every_command_writes_line_feed_endings_only() {
    for line in command_lines("line_feeds") {
        let output = run(&line);
        assert!(
            matches!(output.status.code(), Some(0 | 1)),
            "{line:?}: {output:?}"
        );
        assert!(has_line_feeds_only(&output.stdout), "{line:?}");
        assert!(output.stdout.contains(&b'\n'), "{line:?}");
    }
}

#[test]
fn plt_1_4_the_audit_record_file_has_line_feed_endings_only() {
    let folder = folder("audit_line_feeds");
    let query = file(&folder, "query.json", QUERY.as_bytes());
    let context = file(&folder, "context.json", CONTEXT.as_bytes());
    let options = file(&folder, "options.json", br#"{"audit": true}"#);
    let record = folder.join("record.json");
    let mut line: Vec<OsString> = vec![
        "compile".into(),
        "--model".into(),
        shop("model.yaml").into(),
        "--catalog-schema".into(),
        shop("catalog-schema.json").into(),
    ];
    for (flag, path) in [
        ("--query", &query),
        ("--security-context", &context),
        ("--compile-options", &options),
        ("--audit-record", &record),
    ] {
        line.push(flag.into());
        line.push(path.into());
    }
    let output = run(&line);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(has_line_feeds_only(&fs::read(&record).unwrap()));
}

#[test]
fn plt_1_3_a_document_with_crlf_endings_gives_what_core_gives_for_its_exact_bytes() {
    let folder = folder("crlf_input");
    let model = file(&folder, "model.yaml", CRLF_MODEL);
    let schema = shop("catalog-schema.json");
    let output = run([
        OsString::from("validate"),
        "--model".into(),
        model.into(),
        "--catalog-schema".into(),
        schema.clone().into(),
    ]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report = validate(
        CRLF_MODEL,
        &fs::read(&schema).unwrap(),
        &LoadOptions::default(),
    );
    let expected = format!("{}\n", document_json(&report).unwrap());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    let document: serde_json::Value = serde_json::from_str(&expected).unwrap();
    let location = &document["errors"][0]["location"];
    assert_eq!(document["errors"][0]["code"], "E0A-02");
    assert_eq!(
        (&location["line"], &location["column"]),
        (&3.into(), &15.into())
    );
}

/// The output of `dependencies` over the shop model copied to `model` and `schema`.
fn dependencies_of(model: impl Into<OsString>, schema: impl Into<OsString>) -> Output {
    run([
        OsString::from("dependencies"),
        "--model".into(),
        model.into(),
        "--catalog-schema".into(),
        schema.into(),
    ])
}

/// The shop model and catalog schema copied under a folder and file names outside ASCII.
fn non_ascii_copy(test: &str) -> (PathBuf, PathBuf) {
    let folder = folder(test).join(NON_ASCII_FOLDER);
    fs::create_dir_all(&folder).unwrap();
    let model = file(
        &folder,
        "modèle.yaml",
        &fs::read(shop("model.yaml")).unwrap(),
    );
    let schema = file(
        &folder,
        "schéma-ñ.json",
        &fs::read(shop("catalog-schema.json")).unwrap(),
    );
    (model, schema)
}

#[test]
fn plt_1_5_a_path_outside_ascii_reads_the_same_document() {
    let expected = dependencies_of(shop("model.yaml"), shop("catalog-schema.json"));
    assert_eq!(expected.status.code(), Some(0), "{expected:?}");
    let (model, schema) = non_ascii_copy("non_ascii");
    let output = dependencies_of(&model, &schema);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, expected.stdout);
}

#[test]
fn plt_1_5_the_output_does_not_depend_on_how_the_path_is_written() {
    let (model, schema) = non_ascii_copy("path_forms");
    let expected = dependencies_of(&model, &schema);
    let folder = model.parent().unwrap();
    let detour = folder.join("..").join(NON_ASCII_FOLDER).join("modèle.yaml");
    let output = dependencies_of(&detour, &schema);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, expected.stdout);
}

#[cfg(windows)]
#[test]
fn plt_1_5_a_windows_path_with_a_drive_letter_and_backslashes_reads_the_same_document() {
    let (model, schema) = non_ascii_copy("windows_paths");
    let expected = dependencies_of(&model, &schema);
    let native = |path: &Path| -> OsString {
        let absolute = std::path::absolute(path).unwrap();
        let text = absolute.to_str().unwrap().replace('/', "\\");
        assert!(text.as_bytes().get(1) == Some(&b':'), "{text}");
        OsString::from(text)
    };
    let output = dependencies_of(native(&model), native(&schema));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, expected.stdout);
    let forward = |path: &Path| -> OsString {
        OsString::from(native(path).to_str().unwrap().replace('\\', "/"))
    };
    let output = dependencies_of(forward(&model), forward(&schema));
    assert_eq!(output.stdout, expected.stdout);
}
