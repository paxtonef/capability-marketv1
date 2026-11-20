//! PROCUREMENT V0.3 — extraction structuree et tracable des exigences
//! contenues dans les appels d'offres (Capability Market V1).
//!
//! Chaine de traitement, une etape par module :
//!
//! | etape           | module       |
//! |-----------------|--------------|
//! | lecture         | [`reader`]   |
//! | segmentation    | [`segmenter`]|
//! | classification  | [`classifier`] (lexique : [`lexicon`]) |
//! | extraction      | [`extractor`]|
//! | serialisation   | [`serializer`] (types : [`model`]) |
//!
//! [`legacy`] conserve les regles et la sortie de V0.2 (profil 1.0.0).

pub mod classifier;
pub mod extractor;
pub mod legacy;
pub mod lexicon;
pub mod model;
pub mod reader;
pub mod segmenter;
pub mod serializer;

pub use legacy::extract;
pub use model::ContractVersion;
pub use reader::SourceDocument;
pub use serializer::Format;

pub const RUNNER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Execute la chaine complete sur un document deja lu et renvoie le JSON.
pub fn run(
    tender_id: &str,
    title: &str,
    document: &SourceDocument,
    version: ContractVersion,
    format: Format,
) -> String {
    match version {
        ContractVersion::V1_0 => serializer::to_json(
            &legacy::extract_document(tender_id, title, document.text()),
            format,
        ),
        ContractVersion::V1_1 => serializer::to_json(
            &extractor::extract_requirements(tender_id, title, document),
            format,
        ),
    }
}
