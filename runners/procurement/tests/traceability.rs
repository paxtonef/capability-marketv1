//! Tracabilite et contrat : toute donnee emise doit pouvoir etre retrouvee
//! a l'identique dans le document source, et la charge utile doit respecter
//! le contrat `cm.procurement.requirements.v1` (1.0.0 et 1.1.0).

mod common;

use std::collections::BTreeSet;
use std::fs;

use common::{fixture, repo_root};
use procurement::{ContractVersion, Format, SourceDocument, run};
use serde_json::Value;

fn sources() -> Vec<Vec<u8>> {
    let mut sources = vec![
        fs::read(fixture("tender_complex_v1.txt")).unwrap(),
        fs::read(fixture("tender_v03_modalities.txt")).unwrap(),
        fs::read(repo_root().join("runners/procurement/test_tender.txt")).unwrap(),
    ];
    // Variantes : fins de ligne Windows, BOM UTF-8, indentation.
    let base = String::from_utf8(sources[1].clone()).unwrap();
    sources.push(base.replace('\n', "\r\n").into_bytes());
    sources.push(format!("\u{feff}{base}").into_bytes());
    sources.push(
        base.lines()
            .map(|line| format!("   \t{line}  \n"))
            .collect::<String>()
            .into_bytes(),
    );
    sources
}

fn extract(source: &[u8], version: ContractVersion) -> Value {
    let document = SourceDocument::from_bytes(source.to_vec()).unwrap();
    let json = run(
        "T-TRACE",
        "Tracabilite",
        &document,
        version,
        Format::Compact,
    );
    serde_json::from_str(&json).expect("JSON valide")
}

fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} doit etre un tableau"))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} doit etre une chaine"))
}

fn number(value: &Value, key: &str) -> usize {
    value[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} doit etre un entier")) as usize
}

fn line_of(source: &[u8], byte: usize) -> usize {
    source[..byte].iter().filter(|&&b| b == b'\n').count() + 1
}

// ------------------------------------------------------------------ preuves

#[test]
fn every_evidence_is_the_exact_source_slice_at_its_location() {
    for source in sources() {
        let output = extract(&source, ContractVersion::V1_1);
        assert!(!array(&output, "evidence").is_empty());
        for evidence in array(&output, "evidence") {
            let (start, end) = (number(evidence, "byte_start"), number(evidence, "byte_end"));
            assert_eq!(&source[start..end], text(evidence, "excerpt").as_bytes());
            assert_eq!(line_of(&source, start), number(evidence, "source_line"));
            assert_eq!(line_of(&source, end), number(evidence, "source_line"));

            let line_start = source[..start]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |index| index + 1);
            let prefix = String::from_utf8(source[line_start..start].to_vec()).unwrap();
            let prefix = prefix.trim_start_matches('\u{feff}');
            assert_eq!(prefix.chars().count() + 1, number(evidence, "column_start"));
        }
    }
}

#[test]
fn every_requirement_text_comes_verbatim_from_its_evidence() {
    for source in sources() {
        let output = extract(&source, ContractVersion::V1_1);
        let evidence = array(&output, "evidence");
        let requirements = array(&output, "requirements");
        assert_eq!(evidence.len(), requirements.len());
        for (requirement, proof) in requirements.iter().zip(evidence) {
            assert_eq!(requirement["text"], proof["excerpt"]);
            assert_eq!(requirement["id"], proof["requirement_id"]);
            assert_eq!(
                requirement["evidence_ids"],
                serde_json::json!([proof["id"]])
            );
            assert!(
                text(requirement, "text").contains(text(requirement, "modality")),
                "la modalite doit etre un extrait de l'enonce"
            );
            assert!(!text(requirement, "modality").is_empty());
        }
    }
}

#[test]
fn every_condition_is_an_exact_slice_inside_its_requirement() {
    let mut seen = 0;
    for source in sources() {
        let output = extract(&source, ContractVersion::V1_1);
        let evidence = array(&output, "evidence");
        for (requirement, proof) in array(&output, "requirements").iter().zip(evidence) {
            for condition in array(requirement, "conditions") {
                let (start, end) = (
                    number(condition, "byte_start"),
                    number(condition, "byte_end"),
                );
                assert_eq!(&source[start..end], text(condition, "text").as_bytes());
                assert!(start >= number(proof, "byte_start") && end <= number(proof, "byte_end"));
                assert_eq!(condition["source_line"], proof["source_line"]);
                seen += 1;
            }
            let conditional = requirement["category"] == "CONDITIONAL_OBLIGATION";
            let has_conditions = !array(requirement, "conditions").is_empty();
            assert!(!conditional || has_conditions);
            assert!(requirement["category"] != "OBLIGATION" || !has_conditions);
        }
    }
    assert!(seen >= 6, "les jeux d'essai doivent couvrir des conditions");
}

#[test]
fn every_located_uncertainty_is_an_exact_source_slice() {
    for source in sources() {
        let output = extract(&source, ContractVersion::V1_1);
        for uncertainty in array(&output, "uncertainties") {
            if uncertainty["source_line"].is_null() {
                assert_eq!(uncertainty["kind"], "NO_REQUIREMENT_DETECTED");
                continue;
            }
            let (start, end) = (
                number(uncertainty, "byte_start"),
                number(uncertainty, "byte_end"),
            );
            assert_eq!(&source[start..end], text(uncertainty, "text").as_bytes());
            assert_eq!(line_of(&source, start), number(uncertainty, "source_line"));
            assert!(text(uncertainty, "text").contains(text(uncertainty, "marker")));
        }
    }
}

#[test]
fn nothing_is_extracted_from_a_document_without_normative_statement() {
    let source = "Pr\u{e9}sentation g\u{e9}n\u{e9}rale du march\u{e9}.\nLe pouvoir adjudicateur est une commune.\n";
    let output = extract(source.as_bytes(), ContractVersion::V1_1);
    assert!(array(&output, "requirements").is_empty());
    assert!(array(&output, "evidence").is_empty());
    let uncertainties = array(&output, "uncertainties");
    assert_eq!(uncertainties.len(), 1);
    assert_eq!(uncertainties[0]["kind"], "NO_REQUIREMENT_DETECTED");
    assert!(uncertainties[0]["source_line"].is_null());

    let empty = extract(b"", ContractVersion::V1_1);
    assert!(array(&empty, "requirements").is_empty());
}

#[test]
fn section_headings_are_not_turned_into_requirements() {
    let output = extract(
        &fs::read(fixture("tender_v03_modalities.txt")).unwrap(),
        ContractVersion::V1_1,
    );
    for requirement in array(&output, "requirements") {
        assert_ne!(
            text(requirement, "text"),
            "5. Possibilit\u{e9}s facultatives"
        );
    }
}

// -------------------------------------------------------------- identifiants

#[test]
fn identifiers_are_sequential_unique_and_consistent() {
    for source in sources() {
        let output = extract(&source, ContractVersion::V1_1);
        let ids = |key: &str, prefix: &str| -> BTreeSet<String> {
            let items = array(&output, key);
            let set: BTreeSet<String> = items.iter().map(|i| text(i, "id").to_string()).collect();
            assert_eq!(set.len(), items.len(), "identifiants uniques dans {key}");
            for (index, item) in items.iter().enumerate() {
                assert_eq!(text(item, "id"), format!("{prefix}-{:04}", index + 1));
            }
            set
        };
        let requirement_ids = ids("requirements", "REQ");
        let evidence_ids = ids("evidence", "EV");
        let uncertainty_ids = ids("uncertainties", "UNC");

        for evidence in array(&output, "evidence") {
            assert!(requirement_ids.contains(text(evidence, "requirement_id")));
        }
        for requirement in array(&output, "requirements") {
            for id in array(requirement, "evidence_ids") {
                assert!(evidence_ids.contains(id.as_str().unwrap()));
            }
            for id in array(requirement, "uncertainty_ids") {
                assert!(uncertainty_ids.contains(id.as_str().unwrap()));
                let uncertainty = array(&output, "uncertainties")
                    .iter()
                    .find(|u| u["id"] == *id)
                    .unwrap();
                assert_eq!(uncertainty["requirement_id"], requirement["id"]);
            }
            let binding = matches!(
                text(requirement, "category"),
                "OBLIGATION" | "PROHIBITION" | "CONDITIONAL_OBLIGATION"
            );
            assert_eq!(requirement["binding"], binding);
        }
        for uncertainty in array(&output, "uncertainties") {
            if let Some(id) = uncertainty["requirement_id"].as_str() {
                let requirement = array(&output, "requirements")
                    .iter()
                    .find(|r| r["id"] == id)
                    .expect("exigence referencee");
                assert!(array(requirement, "uncertainty_ids").contains(&uncertainty["id"]));
            }
        }
    }
}

// ------------------------------------------------------------------- contrat

fn assert_satisfies_contract(payload: &Value, contract_file: &str) {
    let contract: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("contracts").join(contract_file)).unwrap(),
    )
    .unwrap();
    assert_eq!(contract["schema_name"], procurement::model::SCHEMA_NAME);
    for field in array(&contract, "required_fields") {
        assert!(
            payload.get(field.as_str().unwrap()).is_some(),
            "champ requis {field}"
        );
    }
    for (field, kind) in contract["field_types"].as_object().unwrap() {
        let Some(value) = payload.get(field) else {
            continue;
        };
        let ok = match kind.as_str().unwrap() {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            other => panic!("type inconnu: {other}"),
        };
        assert!(ok, "{field} doit etre de type {kind}");
    }
}

#[test]
fn both_profiles_satisfy_both_contract_versions() {
    for source in sources() {
        for version in [ContractVersion::V1_0, ContractVersion::V1_1] {
            let payload = extract(&source, version);
            assert_satisfies_contract(&payload, "procurement_requirements_v1.json");
            assert_satisfies_contract(&payload, "procurement_requirements_v1_1.json");
            assert_eq!(payload.as_object().unwrap().len(), 5);
        }
    }
}

#[test]
fn profile_1_1_is_a_superset_of_profile_1_0() {
    let source = fs::read(fixture("tender_complex_v1.txt")).unwrap();
    let old = extract(&source, ContractVersion::V1_0);
    let new = extract(&source, ContractVersion::V1_1);

    // Chaque exigence V0.2 se retrouve avec le meme identifiant, la meme
    // categorie et le meme texte ; chaque preuve avec la meme ligne.
    for (index, requirement) in array(&old, "requirements").iter().enumerate() {
        let extended = &array(&new, "requirements")[index];
        for key in ["id", "category", "text"] {
            assert_eq!(extended[key], requirement[key]);
        }
    }
    for (index, evidence) in array(&old, "evidence").iter().enumerate() {
        let extended = &array(&new, "evidence")[index];
        for key in ["requirement_id", "source_line", "excerpt"] {
            assert_eq!(extended[key], evidence[key]);
        }
    }
    for (index, uncertainty) in array(&old, "uncertainties").iter().enumerate() {
        let extended = &array(&new, "uncertainties")[index];
        for key in ["source_line", "text"] {
            assert_eq!(extended[key], uncertainty[key]);
        }
    }

    // Ce que V0.3 ajoute : la possibilite facultative et l'exemption.
    let added: Vec<&str> = array(&new, "requirements")[array(&old, "requirements").len()..]
        .iter()
        .map(|r| text(r, "category"))
        .collect();
    assert_eq!(added, ["OPTIONAL", "EXEMPTION"]);
    for requirement in &array(&new, "requirements")[5..] {
        assert_eq!(requirement["binding"], false);
    }
}

#[test]
fn library_runs_are_deterministic() {
    for source in sources() {
        let reference = extract(&source, ContractVersion::V1_1);
        for _ in 0..50 {
            assert_eq!(extract(&source, ContractVersion::V1_1), reference);
        }
    }
}

#[test]
fn unusual_input_never_breaks_exact_slicing() {
    let sources = [
        "ne doit",
        "Le candidat ne doit",
        "si",
        "Si, le candidat doit.",
        "\u{130}STANBUL doit \u{ea}tre mentionn\u{e9}e \u{1f600} ; le candidat ne peut pas\u{a0}refuser.",
        "Le candidat DOIT\u{202f}: fournir e\u{301}galement un RIB, sauf",
        "must not must not shall may MAY May may not not required",
        "\r\n\r\n   \r\n",
        "\u{feff}",
        "Le prestataire doit\tlivrer.\u{2028}Il ne doit pas facturer.",
        ". . . ; ; ; ? ! doit",
    ];
    for source in sources {
        let output = extract(source.as_bytes(), ContractVersion::V1_1);
        let bytes = source.as_bytes();
        for evidence in array(&output, "evidence") {
            let (start, end) = (number(evidence, "byte_start"), number(evidence, "byte_end"));
            assert_eq!(
                &bytes[start..end],
                text(evidence, "excerpt").as_bytes(),
                "{source:?}"
            );
        }
        for requirement in array(&output, "requirements") {
            for condition in array(requirement, "conditions") {
                let (start, end) = (
                    number(condition, "byte_start"),
                    number(condition, "byte_end"),
                );
                assert_eq!(
                    &bytes[start..end],
                    text(condition, "text").as_bytes(),
                    "{source:?}"
                );
            }
        }
        extract(source.as_bytes(), ContractVersion::V1_0);
    }
}
