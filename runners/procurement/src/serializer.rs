//! Serialisation JSON typee (serde).
//!
//! La sortie est deterministe : l'ordre des champs est celui des structures
//! de `model`, aucune table de hachage n'intervient.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Compact,
    Pretty,
}

pub fn to_json<T: Serialize>(value: &T, format: Format) -> String {
    let result = match format {
        Format::Compact => serde_json::to_string(value),
        Format::Pretty => serde_json::to_string_pretty(value),
    };
    // Les types du modele n'ont ni cle non textuelle ni `Serialize` faillible.
    result.expect("la serialisation des types du contrat ne peut pas echouer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, ConditionKind, UncertaintyKind};

    #[test]
    fn enumerations_use_contract_names() {
        assert_eq!(
            to_json(&Category::ConditionalObligation, Format::Compact),
            "\"CONDITIONAL_OBLIGATION\""
        );
        assert_eq!(
            to_json(&ConditionKind::ProvidedThat, Format::Compact),
            "\"PROVIDED_THAT\""
        );
        assert_eq!(
            to_json(&UncertaintyKind::NoRequirementDetected, Format::Compact),
            "\"NO_REQUIREMENT_DETECTED\""
        );
    }

    #[test]
    fn escapes_special_and_control_characters() {
        let json = to_json(&"a\"b\\c\n\t\u{1}\u{e9}", Format::Compact);
        assert_eq!(json, "\"a\\\"b\\\\c\\n\\t\\u0001\u{e9}\"");
        let back: String = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "a\"b\\c\n\t\u{1}\u{e9}");
    }
}
