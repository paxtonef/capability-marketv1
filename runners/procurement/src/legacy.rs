//! Profil `1.0.0` : regles de PROCUREMENT V0.2, conservees a l'identique.
//!
//! Ce module reproduit la sortie V0.2 (une exigence par ligne, exemptions
//! ecartees, trois categories). Seule la generation du JSON a change : elle
//! passe par la serialisation typee.

use crate::model::{LegacyDocument, LegacyEvidence, LegacyRequirement, LegacyUncertainty};
use crate::serializer::{self, Format};

pub const NO_REQUIREMENT_NOTICE: &str = "Aucune exigence explicite detectee par les regles V0.2";

const EXEMPTION: &[&str] = &[
    "n'est pas obligatoire",
    "n\u{2019}est pas obligatoire",
    "non obligatoire",
    "not required",
    "not mandatory",
];

const UNCERTAINTY: &[&str] = &[
    "\u{e0} confirmer",
    "a confirmer",
    "\u{e0} pr\u{e9}ciser",
    "a preciser",
    "reste \u{e0} confirmer",
    "to be confirmed",
    "to be determined",
];

const OBLIGATION: &[&str] = &[
    "doit ",
    "doivent ",
    "devra ",
    "devront ",
    "obligatoire",
    "must ",
    "shall ",
];

const PROHIBITION: &[&str] = &[
    "ne doit pas",
    "ne doivent pas",
    "ne devra pas",
    "must not",
    "shall not",
];

const CONDITIONAL_PREFIX: &[&str] = &[
    "si ",
    "lorsque ",
    "\u{e0} condition que ",
    "a condition que ",
    "if ",
    "provided that ",
];

pub fn extract_document(tender_id: &str, title: &str, source: &str) -> LegacyDocument {
    let mut requirements = Vec::new();
    let mut evidence = Vec::new();
    let mut uncertainties = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let lower = text.to_lowercase();
        let contains = |markers: &[&str]| markers.iter().any(|marker| lower.contains(marker));

        if contains(UNCERTAINTY) {
            uncertainties.push(LegacyUncertainty::Located {
                source_line: index + 1,
                text: text.to_string(),
            });
        }
        if contains(EXEMPTION) || !contains(OBLIGATION) {
            continue;
        }

        let category = if contains(PROHIBITION) {
            "PROHIBITION"
        } else if CONDITIONAL_PREFIX
            .iter()
            .any(|marker| lower.starts_with(marker))
        {
            "CONDITIONAL_OBLIGATION"
        } else {
            "OBLIGATION"
        };

        let id = format!("REQ-{:04}", requirements.len() + 1);
        requirements.push(LegacyRequirement {
            id: id.clone(),
            category,
            text: text.to_string(),
        });
        evidence.push(LegacyEvidence {
            requirement_id: id,
            source_line: index + 1,
            excerpt: text.to_string(),
        });
    }

    if requirements.is_empty() {
        uncertainties.push(LegacyUncertainty::Notice(NO_REQUIREMENT_NOTICE.to_string()));
    }

    LegacyDocument {
        tender_id: tender_id.to_string(),
        title: title.to_string(),
        requirements,
        evidence,
        uncertainties,
    }
}

/// Point d'entree historique de V0.2 : JSON compact, profil 1.0.0.
pub fn extract(tender_id: &str, title: &str, source: &str) -> String {
    serializer::to_json(&extract_document(tender_id, title, source), Format::Compact)
}

// Les cinq tests ci-dessous sont ceux de PROCUREMENT V0.2, inchanges
// (d'ou `rustfmt::skip` : leur mise en forme d'origine est conservee).
#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn detects_french_requirement() {
        let output = extract(
            "T-001",
            "Test",
            "Le fournisseur doit presenter une assurance."
        );
        assert!(output.contains("REQ-0001"));
        assert!(output.contains("\"source_line\":1"));
    }

    #[test]
    fn detects_english_requirement() {
        let output = extract(
            "T-002",
            "Test",
            "The supplier shall provide documentation."
        );
        assert!(output.contains("REQ-0001"));
    }

    #[test]
    fn reports_no_detected_requirements() {
        let output = extract("T-003", "Test", "Presentation generale.");
        assert!(output.contains("Aucune exigence explicite"));
        assert!(output.contains("\"requirements\":[]"));
    }

    #[test]
    fn escapes_json_special_characters() {
        let output = extract(
            "T-004",
            "Test",
            "Le candidat doit fournir un document \"certifie\"."
        );
        assert!(output.contains("\\\"certifie\\\""));
    }

    #[test]
    fn classifies_complex_tender_v02() {
        let source = include_str!(
            "../../../tests/fixtures/procurement/tender_complex_v1.txt"
        );
        let output = extract("CM-COMPLEX-001", "Services linguistiques", source);

        assert_eq!(output.matches("\"category\":\"OBLIGATION\"").count(), 3);
        assert_eq!(output.matches("\"category\":\"PROHIBITION\"").count(), 1);
        assert_eq!(
            output.matches("\"category\":\"CONDITIONAL_OBLIGATION\"").count(),
            1
        );
        assert!(!output.contains("bureau local"));
        assert!(output.contains("\"source_line\":12"));
        assert!(output.contains("reste à confirmer"));
    }

}
