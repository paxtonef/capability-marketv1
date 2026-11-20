//! Lecture du document source.
//!
//! Le texte est lu tel quel : aucune normalisation n'est appliquee, afin que
//! les preuves restent des extraits exacts. Un fichier qui n'est pas de
//! l'UTF-8 valide est refuse plutot que converti avec perte.

use std::fmt;
use std::fs;
use std::io::{self, Read};

const UTF8_BOM: &str = "\u{feff}";

#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    InvalidUtf8 { valid_up_to: usize },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::InvalidUtf8 { valid_up_to } => write!(
                f,
                "le fichier n'est pas de l'UTF-8 valide (octet {valid_up_to})"
            ),
        }
    }
}

impl std::error::Error for ReadError {}

/// Document source en memoire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDocument {
    text: String,
    /// Nombre d'octets precedant `text` dans le fichier (BOM UTF-8 eventuel).
    base_offset: usize,
}

impl SourceDocument {
    pub fn from_text(text: &str) -> Self {
        match text.strip_prefix(UTF8_BOM) {
            Some(rest) => Self {
                text: rest.to_string(),
                base_offset: UTF8_BOM.len(),
            },
            None => Self {
                text: text.to_string(),
                base_offset: 0,
            },
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ReadError> {
        let text = String::from_utf8(bytes).map_err(|error| ReadError::InvalidUtf8 {
            valid_up_to: error.utf8_error().valid_up_to(),
        })?;
        Ok(Self::from_text(&text))
    }

    /// Texte du document, sans BOM.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn base_offset(&self) -> usize {
        self.base_offset
    }
}

/// Lit un fichier ; le chemin `-` designe l'entree standard.
pub fn read_source(path: &str) -> Result<SourceDocument, ReadError> {
    let bytes = if path == "-" {
        let mut buffer = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut buffer)
            .map_err(ReadError::Io)?;
        buffer
    } else {
        fs::read(path).map_err(ReadError::Io)?
    };
    SourceDocument::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_bom_and_keeps_offset() {
        let document = SourceDocument::from_bytes(b"\xef\xbb\xbfLe texte".to_vec()).unwrap();
        assert_eq!(document.text(), "Le texte");
        assert_eq!(document.base_offset(), 3);
    }

    #[test]
    fn rejects_invalid_utf8() {
        let error = SourceDocument::from_bytes(vec![b'a', 0xff, b'b']).unwrap_err();
        assert!(matches!(error, ReadError::InvalidUtf8 { valid_up_to: 1 }));
    }

    #[test]
    fn reports_missing_file() {
        assert!(matches!(
            read_source("/nonexistent/procurement/source.txt"),
            Err(ReadError::Io(_))
        ));
    }
}
