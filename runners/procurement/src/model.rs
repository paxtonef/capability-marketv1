//! Types du contrat `cm.procurement.requirements.v1`.
//!
//! Deux profils de sortie partagent les memes cinq champs racine
//! (`tender_id`, `title`, `requirements`, `evidence`, `uncertainties`) :
//!
//! * profil `1.0.0` : forme historique produite par PROCUREMENT V0.2 ;
//! * profil `1.1.0` : extension compatible (champs supplementaires dans les
//!   elements des tableaux uniquement, aucun champ racine ajoute).
//!
//! L'ordre des champs des structures est l'ordre de serialisation : il fait
//! partie du caractere deterministe de la sortie.

use serde::Serialize;

pub const SCHEMA_NAME: &str = "cm.procurement.requirements.v1";

/// Version du contrat demandee en sortie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractVersion {
    V1_0,
    V1_1,
}

impl ContractVersion {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "1.0" | "1.0.0" => Some(Self::V1_0),
            "1.1" | "1.1.0" => Some(Self::V1_1),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1_0 => "1.0.0",
            Self::V1_1 => "1.1.0",
        }
    }
}

// ---------------------------------------------------------------- profil 1.1.0

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Obligation,
    Prohibition,
    ConditionalObligation,
    Exemption,
    Optional,
}

impl Category {
    /// Vrai si l'enonce contraint le destinataire (obligation, interdiction).
    pub fn is_binding(self) -> bool {
        matches!(
            self,
            Self::Obligation | Self::Prohibition | Self::ConditionalObligation
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConditionKind {
    If,
    When,
    ProvidedThat,
    Exception,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UncertaintyKind {
    /// Information annoncee mais non encore fournie (« a confirmer », « TBD »).
    PendingInformation,
    /// Modalite non contraignante ou hesitante (« devrait », « should »).
    WeakModality,
    /// Terme laissant une marge d'interpretation (« si possible », « et/ou »).
    VagueTerm,
    /// Negation dont la portee n'a pas pu etre etablie par les regles.
    AmbiguousNegation,
    /// Plusieurs modalites normatives dans un meme enonce.
    MultipleModalities,
    /// Aucun enonce normatif n'a ete detecte dans le document.
    NoRequirementDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Condition {
    #[serde(rename = "type")]
    pub kind: ConditionKind,
    /// Texte exact de la clause conditionnelle, marqueur inclus.
    pub text: String,
    pub source_line: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Requirement {
    pub id: String,
    pub category: Category,
    /// Texte exact de l'enonce source.
    pub text: String,
    pub binding: bool,
    /// Marqueur de modalite tel qu'il apparait dans la source.
    pub modality: String,
    pub conditions: Vec<Condition>,
    pub evidence_ids: Vec<String>,
    pub uncertainty_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Evidence {
    pub id: String,
    pub requirement_id: String,
    /// Ligne (base 1) du document source.
    pub source_line: u32,
    /// Extrait exact : `source[byte_start..byte_end]`.
    pub excerpt: String,
    /// Colonne (base 1, en caracteres) du debut de l'extrait dans la ligne.
    pub column_start: u32,
    /// Decalages en octets dans le fichier source (UTF-8, BOM compris).
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Uncertainty {
    pub id: String,
    pub kind: UncertaintyKind,
    pub source_line: Option<u32>,
    /// Texte exact de l'enonce source (ou avis du runner si `source_line` est nul).
    pub text: String,
    /// Marqueur declencheur tel qu'il apparait dans la source.
    pub marker: Option<String>,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
    /// Exigence concernee, le cas echeant.
    pub requirement_id: Option<String>,
}

/// Charge utile `cm.procurement.requirements.v1`, profil 1.1.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementsDocument {
    pub tender_id: String,
    pub title: String,
    pub requirements: Vec<Requirement>,
    pub evidence: Vec<Evidence>,
    pub uncertainties: Vec<Uncertainty>,
}

// ---------------------------------------------------------------- profil 1.0.0

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyRequirement {
    pub id: String,
    pub category: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyEvidence {
    pub requirement_id: String,
    pub source_line: usize,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum LegacyUncertainty {
    Located { source_line: usize, text: String },
    Notice(String),
}

/// Charge utile `cm.procurement.requirements.v1`, profil 1.0.0 (forme V0.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyDocument {
    pub tender_id: String,
    pub title: String,
    pub requirements: Vec<LegacyRequirement>,
    pub evidence: Vec<LegacyEvidence>,
    pub uncertainties: Vec<LegacyUncertainty>,
}
