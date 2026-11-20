//! Extraction : assemble segments et classification en un document du
//! contrat `cm.procurement.requirements.v1`, profil 1.1.0.
//!
//! Invariant : tout texte emis (`text`, `excerpt`, `modality`, `marker`,
//! condition) est une tranche exacte du document source. Seul l'avis
//! `NO_REQUIREMENT_DETECTED` est un message du runner, reconnaissable a sa
//! localisation nulle.

use crate::classifier::{self, ByteSpan};
use crate::model::{
    Condition, Evidence, Requirement, RequirementsDocument, Uncertainty, UncertaintyKind,
};
use crate::reader::SourceDocument;
use crate::segmenter::{self, Segment};

pub const NO_REQUIREMENT_NOTICE: &str = "Aucune exigence explicite detectee par les regles V0.3";

pub fn extract_requirements(
    tender_id: &str,
    title: &str,
    document: &SourceDocument,
) -> RequirementsDocument {
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut evidence: Vec<Evidence> = Vec::new();
    let mut uncertainties: Vec<Uncertainty> = Vec::new();

    for segment in segmenter::segment(document) {
        let analysis = classifier::classify(segment.text);
        // Un titre de section dont la seule modalite est nominale n'est pas
        // un enonce normatif : il est ignore.
        if segment.heading && analysis.nominal {
            continue;
        }

        let requirement_id = analysis
            .category
            .map(|_| format!("REQ-{:04}", requirements.len() + 1));

        let mut uncertainty_ids = Vec::new();
        for flag in &analysis.flags {
            let id = format!("UNC-{:04}", uncertainties.len() + 1);
            uncertainties.push(Uncertainty {
                id: id.clone(),
                kind: flag.kind,
                source_line: Some(segment.line),
                text: segment.text.to_string(),
                marker: Some(slice(&segment, flag.marker).to_string()),
                byte_start: Some(segment.byte_start),
                byte_end: Some(segment.byte_end),
                requirement_id: requirement_id.clone(),
            });
            uncertainty_ids.push(id);
        }

        let (Some(category), Some(id)) = (analysis.category, requirement_id) else {
            continue;
        };
        let evidence_id = format!("EV-{:04}", evidence.len() + 1);

        requirements.push(Requirement {
            id: id.clone(),
            category,
            text: segment.text.to_string(),
            binding: category.is_binding(),
            modality: analysis
                .modality
                .map(|span| slice(&segment, span).to_string())
                .unwrap_or_default(),
            conditions: analysis
                .conditions
                .iter()
                .map(|condition| Condition {
                    kind: condition.kind,
                    text: slice(&segment, condition.span).to_string(),
                    source_line: segment.line,
                    byte_start: segment.byte_start + condition.span.0,
                    byte_end: segment.byte_start + condition.span.1,
                })
                .collect(),
            evidence_ids: vec![evidence_id.clone()],
            uncertainty_ids,
        });

        evidence.push(Evidence {
            id: evidence_id,
            requirement_id: id,
            source_line: segment.line,
            excerpt: segment.text.to_string(),
            column_start: segment.column,
            byte_start: segment.byte_start,
            byte_end: segment.byte_end,
        });
    }

    if requirements.is_empty() {
        uncertainties.push(Uncertainty {
            id: format!("UNC-{:04}", uncertainties.len() + 1),
            kind: UncertaintyKind::NoRequirementDetected,
            source_line: None,
            text: NO_REQUIREMENT_NOTICE.to_string(),
            marker: None,
            byte_start: None,
            byte_end: None,
            requirement_id: None,
        });
    }

    RequirementsDocument {
        tender_id: tender_id.to_string(),
        title: title.to_string(),
        requirements,
        evidence,
        uncertainties,
    }
}

fn slice<'a>(segment: &Segment<'a>, span: ByteSpan) -> &'a str {
    &segment.text[span.0..span.1]
}
