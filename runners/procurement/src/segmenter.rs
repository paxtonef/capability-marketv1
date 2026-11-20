//! Segmentation du document en enonces localises.
//!
//! Unite de base : la ligne physique, redecoupee en phrases. Chaque segment
//! est une tranche exacte du texte source, accompagnee de sa position.

use crate::reader::SourceDocument;

/// Enonce localise dans le document source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment<'a> {
    /// Ligne (base 1).
    pub line: u32,
    /// Colonne (base 1, en caracteres) du premier caractere.
    pub column: u32,
    /// Decalages en octets dans le fichier source.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Texte exact, sans blancs de bord.
    pub text: &'a str,
    /// Vrai si la ligne a la forme d'un titre de section (voir [`is_heading`]).
    pub heading: bool,
}

/// Abreviations (minuscules, sans accent) apres lesquelles un point ne
/// termine pas la phrase.
const ABBREVIATIONS: &[&str] = &[
    "al", "approx", "art", "cf", "chap", "co", "dr", "ed", "eg", "env", "ex", "fig", "ie", "inc",
    "ltd", "m", "max", "min", "mlle", "mm", "mme", "n", "nb", "no", "nos", "p", "par", "pp", "ref",
    "st", "ste", "tel", "vol", "vs",
];

pub fn segment(document: &SourceDocument) -> Vec<Segment<'_>> {
    let text = document.text();
    let base = document.base_offset();
    let mut segments = Vec::new();
    let mut line_start = 0usize;
    let mut line_number = 0u32;

    while line_start <= text.len() {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |index| line_start + index);
        line_number += 1;
        let line = text[line_start..line_end]
            .strip_suffix('\r')
            .unwrap_or(&text[line_start..line_end]);

        let heading = is_heading(line);
        for (start, end) in split_sentences(line) {
            if let Some((start, end)) = trim_range(line, start, end) {
                segments.push(Segment {
                    line: line_number,
                    column: line[..start].chars().count() as u32 + 1,
                    byte_start: base + line_start + start,
                    byte_end: base + line_start + end,
                    text: &line[start..end],
                    heading,
                });
            }
        }

        if line_end == text.len() {
            break;
        }
        line_start = line_end + 1;
    }
    segments
}

/// Mots introduisant un titre numerote (« Article 3 », « Annexe 2 »).
const HEADING_PREFIXES: &[&str] = &[
    "annex", "annexe", "article", "chapitre", "chapter", "lot", "part", "partie", "section",
    "titre",
];

/// Longueur maximale d'un titre, en mots, numerotation comprise.
const HEADING_MAX_WORDS: usize = 8;

/// Forme d'un titre de section : ligne numerotee (« 5. », « 3.2 », « Article
/// 4 »), courte et sans ponctuation finale. Ce critere est purement formel ;
/// l'extraction ne s'en sert que pour ecarter les titres dont la seule
/// modalite est nominale (« 5. Possibilites facultatives »).
pub fn is_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.ends_with(['.', ';', ':', '!', '?', ',']) {
        return false;
    }
    let mut words = trimmed.split_whitespace();
    let count = trimmed.split_whitespace().count();
    if count > HEADING_MAX_WORDS {
        return false;
    }
    let Some(mut first) = words.next() else {
        return false;
    };
    if HEADING_PREFIXES.contains(&first.to_lowercase().as_str()) {
        match words.next() {
            Some(next) => first = next,
            None => return false,
        }
    }
    first.starts_with(|c: char| c.is_ascii_digit())
        && first
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ')' | '-' | ':'))
}

fn trim_range(line: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let slice = &line[start..end];
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lead = slice.len() - slice.trim_start().len();
    Some((start + lead, start + lead + trimmed.len()))
}

/// Decoupe une ligne en phrases ; renvoie des intervalles d'octets.
///
/// Fin de phrase : `;` suivi d'un blanc, ou `.`, `!`, `?` suivi d'un blanc
/// puis d'une majuscule, sauf apres une abreviation, un nombre ou une
/// initiale (« art. 5 », « 3. Le prestataire », « M. Dupont »).
fn split_sentences(line: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut ranges = Vec::new();
    let mut start = 0usize;

    for (index, &(offset, c)) in chars.iter().enumerate() {
        if !matches!(c, '.' | '!' | '?' | ';') {
            continue;
        }
        let Some(&(_, next)) = chars.get(index + 1) else {
            continue;
        };
        if !next.is_whitespace() {
            continue;
        }
        let Some(&(_, following)) = chars[index + 1..].iter().find(|(_, c)| !c.is_whitespace())
        else {
            continue;
        };
        let boundary = match c {
            ';' => true,
            '.' => following.is_uppercase() && !is_non_terminal_period(&chars[..index]),
            _ => following.is_uppercase(),
        };
        if boundary {
            let end = offset + c.len_utf8();
            ranges.push((start, end));
            start = end;
        }
    }
    ranges.push((start, line.len()));
    ranges
}

fn is_non_terminal_period(before: &[(usize, char)]) -> bool {
    let word: String = before
        .iter()
        .rev()
        .map(|&(_, c)| c)
        .take_while(|c| c.is_alphanumeric())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if word.is_empty() {
        return false;
    }
    if word.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    if word.chars().count() == 1 {
        return true;
    }
    ABBREVIATIONS.contains(&word.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<String> {
        let document = SourceDocument::from_text(source);
        segment(&document)
            .into_iter()
            .map(|s| s.text.to_string())
            .collect()
    }

    #[test]
    fn splits_lines_and_skips_blank_ones() {
        assert_eq!(texts("Un.\n\n  Deux.  \n"), vec!["Un.", "Deux."]);
    }

    #[test]
    fn splits_sentences_on_one_line() {
        assert_eq!(
            texts("Le prestataire doit livrer. Il ne doit pas sous-traiter."),
            vec![
                "Le prestataire doit livrer.",
                "Il ne doit pas sous-traiter."
            ]
        );
    }

    #[test]
    fn splits_on_semicolon() {
        assert_eq!(
            texts("le rapport est obligatoire ; la visite est facultative."),
            vec!["le rapport est obligatoire ;", "la visite est facultative."]
        );
    }

    #[test]
    fn keeps_abbreviations_numbers_and_initials_together() {
        assert_eq!(
            texts("Selon l'art. L. 2141-1, le candidat doit attester."),
            vec!["Selon l'art. L. 2141-1, le candidat doit attester."]
        );
        assert_eq!(
            texts("3. Le prestataire doit livrer."),
            vec!["3. Le prestataire doit livrer."]
        );
        assert_eq!(
            texts("Le prix est de 1.5 EUR. Le delai est de 3 jours."),
            vec!["Le prix est de 1.5 EUR.", "Le delai est de 3 jours."]
        );
    }

    #[test]
    fn recognises_numbered_headings_by_form_only() {
        for line in [
            "5. Possibilit\u{e9}s facultatives",
            "3.2 Assurance obligatoire",
            "Article 4 - Sous-traitance interdite",
            "  2) Exemptions",
        ] {
            assert!(is_heading(line), "{line}");
        }
        for line in [
            "La visite est facultative.",
            "3. Le prestataire doit livrer le rapport.",
            "Assurance obligatoire",
            "2026 est l'ann\u{e9}e de r\u{e9}f\u{e9}rence du march\u{e9} pour le calcul des prix",
        ] {
            assert!(!is_heading(line), "{line}");
        }
    }

    #[test]
    fn positions_are_exact_slices_of_the_source() {
        let source =
            "\u{feff}Titre\r\n  Le d\u{e9}p\u{f4}t doit \u{ea}tre fait. Il est gratuit.\r\n";
        let document = SourceDocument::from_text(source);
        let segments = segment(&document);
        assert_eq!(segments.len(), 3);
        for s in &segments {
            assert_eq!(&source[s.byte_start..s.byte_end], s.text);
        }
        assert_eq!((segments[1].line, segments[1].column), (2, 3));
        assert_eq!((segments[2].line, segments[2].column), (2, 28));
    }
}
