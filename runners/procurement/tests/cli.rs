//! Tests de l'interface en ligne de commande : usage, profils, sorties de
//! reference, validite du JSON et determinisme.

mod common;

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use common::{BIN, fixture, repo_root, run, stdout};
use serde_json::Value;

const ROOT_FIELDS: [&str; 5] = [
    "tender_id",
    "title",
    "requirements",
    "evidence",
    "uncertainties",
];

fn complex() -> String {
    fixture("tender_complex_v1.txt").display().to_string()
}

fn modalities() -> String {
    fixture("tender_v03_modalities.txt").display().to_string()
}

// ------------------------------------------------------------------- usage

#[test]
fn v02_invocation_still_works_and_emits_valid_json() {
    let output = stdout(&["CM-COMPLEX-001", "Services linguistiques", &complex()]);
    let value: Value = serde_json::from_str(&output).expect("JSON valide");
    let object = value.as_object().unwrap();
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys.len(), 5, "aucun champ racine ajoute: {keys:?}");
    for field in ROOT_FIELDS {
        assert!(object.contains_key(field), "champ manquant: {field}");
    }
    assert!(output.ends_with('\n') && !output.trim_end().contains('\n'));
}

#[test]
fn wrong_usage_exits_with_code_2() {
    for args in [
        &[][..],
        &["T", "Titre"][..],
        &["T", "Titre", "a.txt", "b.txt"][..],
        &["--inconnu", "T", "Titre", "a.txt"][..],
        &["--contract-version", "9.9.9", "T", "Titre", "a.txt"][..],
        &["T", "Titre", "a.txt", "--output"][..],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Usage: procurement"));
    }
}

#[test]
fn unreadable_source_exits_with_code_1() {
    let output = run(&["T", "Titre", "/nonexistent/tender.txt"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("Erreur de lecture"));
}

#[test]
fn non_utf8_source_is_refused_rather_than_altered() {
    let path = std::env::temp_dir().join(format!("procurement-latin1-{}.txt", std::process::id()));
    fs::write(&path, b"Le candidat doit fournir une r\xe9f\xe9rence.").unwrap();
    let output = run(&["T", "Titre", path.to_str().unwrap()]);
    fs::remove_file(&path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("UTF-8"));
}

#[test]
fn help_and_version_are_available() {
    assert!(stdout(&["--help"]).contains("--contract-version"));
    let version = stdout(&["--version"]);
    assert!(version.starts_with("procurement 0.3.0"), "{version}");
}

#[test]
fn reads_standard_input_and_writes_output_file() {
    let source = fs::read(fixture("tender_complex_v1.txt")).unwrap();
    let mut child = Command::new(BIN)
        .args(["CM-COMPLEX-001", "Titre", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&source).unwrap();
    let from_stdin = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    let from_file = stdout(&["CM-COMPLEX-001", "Titre", &complex()]);
    assert_eq!(from_stdin, from_file);

    let path = std::env::temp_dir().join(format!("procurement-out-{}.json", std::process::id()));
    let silent = stdout(&[
        "-o",
        path.to_str().unwrap(),
        "CM-COMPLEX-001",
        "Titre",
        &complex(),
    ]);
    let written = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(silent.is_empty());
    assert_eq!(written, from_file);
}

#[test]
fn pretty_and_compact_outputs_carry_the_same_data() {
    let compact: Value = serde_json::from_str(&stdout(&["T", "Titre", &modalities()])).unwrap();
    let pretty: Value =
        serde_json::from_str(&stdout(&["--pretty", "T", "Titre", &modalities()])).unwrap();
    assert_eq!(compact, pretty);
}

#[test]
fn title_and_identifier_are_escaped_not_interpreted() {
    let output = stdout(&["ID \"1\"", "Titre\navec \\ et \u{e9}", &complex()]);
    let value: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["tender_id"], "ID \"1\"");
    assert_eq!(value["title"], "Titre\navec \\ et \u{e9}");
}

// ------------------------------------------------------ sorties de reference

#[test]
fn profile_1_0_reproduces_the_v02_output_byte_for_byte() {
    let output = stdout(&[
        "--contract-version",
        "1.0.0",
        "CM-COMPLEX-001",
        "Services linguistiques multilingues",
        &complex(),
    ]);
    let expected = fs::read_to_string(repo_root().join("procurement_output_v02.json")).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn profile_1_0_reproduces_the_v01_requirements() {
    let source = repo_root().join("runners/procurement/test_tender.txt");
    let output = stdout(&[
        "--contract-version=1.0",
        "CM-TEST-001",
        "Services linguistiques",
        source.to_str().unwrap(),
    ]);
    let actual: Value = serde_json::from_str(&output).unwrap();
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("procurement_output.json")).unwrap(),
    )
    .unwrap();
    // La sortie V0.1 n'avait pas de categorie : on compare le reste.
    assert_eq!(actual["evidence"], expected["evidence"]);
    assert_eq!(actual["uncertainties"], expected["uncertainties"]);
    let strip = |value: &Value| -> Vec<(Value, Value)> {
        value["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].clone(), r["text"].clone()))
            .collect()
    };
    assert_eq!(strip(&actual), strip(&expected));
}

#[test]
fn profile_1_1_matches_the_reference_output_of_the_complex_tender() {
    let output = stdout(&[
        "CM-COMPLEX-001",
        "Services linguistiques multilingues",
        &complex(),
    ]);
    let expected = fs::read_to_string(repo_root().join("procurement_output_v03.json")).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn profile_1_1_matches_the_reference_output_of_the_modalities_tender() {
    let output = stdout(&[
        "--pretty",
        "CM-V03-001",
        "Services de traduction et d'interpr\u{e9}tation",
        &modalities(),
    ]);
    let expected = fs::read_to_string(fixture("tender_v03_modalities.expected.json")).unwrap();
    assert_eq!(output, expected);
}

// ------------------------------------------------------------ determinisme

#[test]
fn repeated_runs_are_byte_identical() {
    for version in ["1.0.0", "1.1.0"] {
        for source in [complex(), modalities()] {
            let args = [
                "--contract-version",
                version,
                "T-DET",
                "Determinisme",
                &source,
            ];
            let first = run(&args).stdout;
            assert!(!first.is_empty());
            for _ in 0..20 {
                assert_eq!(run(&args).stdout, first, "{version} {source}");
            }
        }
    }
}

#[test]
fn output_does_not_depend_on_environment() {
    let args = ["T-DET", "Determinisme", &modalities()];
    let reference = run(&args).stdout;
    let other = Command::new(BIN)
        .args(args)
        .env_clear()
        .env("LANG", "C")
        .env("TZ", "Pacific/Auckland")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert_eq!(other.stdout, reference);
}
