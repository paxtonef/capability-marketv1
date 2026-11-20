//! Classification d'un enonce isole.
//!
//! Fonction pure : memes entrees, memes sorties. Elle ne produit que des
//! positions dans l'enonce ; aucun texte n'est fabrique.

use crate::lexicon::{self, CharSpan, Folded};
use crate::model::{Category, ConditionKind, UncertaintyKind};

/// Intervalle en octets dans l'enonce analyse.
pub type ByteSpan = (usize, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    pub kind: UncertaintyKind,
    pub marker: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionMatch {
    pub kind: ConditionKind,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Analysis {
    /// Categorie normative, ou `None` si l'enonce n'en porte pas.
    pub category: Option<Category>,
    /// Marqueur de modalite retenu.
    pub modality: Option<ByteSpan>,
    /// Vrai si ce marqueur est nominal ou adjectival (« obligatoire »,
    /// « optional ») : seul, il ne suffit pas a faire d'un titre une exigence.
    pub nominal: bool,
    /// Clauses conditionnelles, dans l'ordre du texte.
    pub conditions: Vec<ConditionMatch>,
    /// Signaux d'incertitude, dans un ordre fixe.
    pub flags: Vec<Flag>,
}

/// Classes de modalite, de la plus prioritaire a la moins prioritaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Modality {
    Exemption,
    Prohibition,
    Obligation,
    Optional,
}

struct Hit {
    modality: Modality,
    span: CharSpan,
    nominal: bool,
}

pub fn classify(text: &str) -> Analysis {
    let folded = Folded::new(text);
    let mut hits: Vec<Hit> = Vec::new();
    // Tous les candidats deja examines, retenus ou non : un marqueur qui
    // recouvre un candidat plus prioritaire est ecarte, meme si ce candidat a
    // lui-meme ete ecarte (« shall » dans « shall not be required »).
    let mut seen: Vec<CharSpan> = Vec::new();
    let mut ambiguous_negation: Option<CharSpan> = None;

    // 1. Negations francaises « ne + modal + pas/jamais/aucun... ».
    for phrase in lexicon::NE_MODAL {
        for span in folded.find_all(phrase) {
            match resolve_negation(&folded, span) {
                Some(hit) => push_hit(&mut hits, &mut seen, hit),
                None => {
                    // Portee indeterminee : le modal n'est pas interprete,
                    // l'enonce est seulement signale.
                    seen.push(span);
                    if ambiguous_negation.is_none_or(|current| span.0 < current.0) {
                        ambiguous_negation = Some(span);
                    }
                }
            }
        }
    }

    // 2. Marqueurs lexicaux, par priorite decroissante : un marqueur recouvert
    //    par un marqueur plus prioritaire est ignore (« doit » dans « ne doit pas »).
    let tables = [
        (Modality::Exemption, lexicon::EXEMPTION),
        (Modality::Prohibition, lexicon::PROHIBITION),
        (Modality::Obligation, lexicon::OBLIGATION),
        (Modality::Optional, lexicon::OPTIONAL),
    ];
    for (modality, markers) in tables {
        for marker in markers {
            for span in folded.find_all(marker) {
                if *marker == "may" && is_month_name(text, &folded, span) {
                    continue;
                }
                let nominal = lexicon::NOMINAL.contains(marker);
                push_hit(
                    &mut hits,
                    &mut seen,
                    Hit {
                        modality,
                        span,
                        nominal,
                    },
                );
            }
        }
    }
    hits.sort_by_key(|hit| (hit.modality, hit.span));

    let primary = hits.first().map(|hit| hit.modality);
    let secondary = hits.iter().find(|hit| Some(hit.modality) != primary);

    // 3. Signaux d'incertitude.
    let pending = first_match(&folded, lexicon::PENDING);
    let weak = first_match(&folded, lexicon::WEAK);
    let vague_spans = all_matches(&folded, lexicon::VAGUE);

    let mut flags = Vec::new();
    let mut push_flag = |kind, span: CharSpan| {
        flags.push(Flag {
            kind,
            marker: folded.bytes(span),
        })
    };
    if let Some(span) = pending {
        push_flag(UncertaintyKind::PendingInformation, span);
    }
    if let Some(span) = weak {
        push_flag(UncertaintyKind::WeakModality, span);
    }
    // Un terme vague isole, sans portee normative, n'est pas signale.
    if (primary.is_some() || weak.is_some())
        && let Some(&span) = vague_spans.first()
    {
        push_flag(UncertaintyKind::VagueTerm, span);
    }
    if let Some(span) = ambiguous_negation {
        push_flag(UncertaintyKind::AmbiguousNegation, span);
    }
    if let Some(hit) = secondary {
        push_flag(UncertaintyKind::MultipleModalities, hit.span);
    }

    let Some(primary) = primary else {
        return Analysis {
            flags,
            ..Analysis::default()
        };
    };

    // 4. Conditions.
    let conditions = find_conditions(text, &folded, &vague_spans);

    let category = match primary {
        Modality::Exemption => Category::Exemption,
        Modality::Prohibition => Category::Prohibition,
        Modality::Optional => Category::Optional,
        Modality::Obligation if conditions.is_empty() => Category::Obligation,
        Modality::Obligation => Category::ConditionalObligation,
    };

    Analysis {
        category: Some(category),
        modality: Some(folded.bytes(hits[0].span)),
        nominal: hits.iter().all(|hit| hit.nominal),
        conditions,
        flags,
    }
}

fn overlaps(a: CharSpan, b: CharSpan) -> bool {
    a.0 < b.1 && b.0 < a.1
}

fn push_hit(hits: &mut Vec<Hit>, seen: &mut Vec<CharSpan>, hit: Hit) {
    let shadowed = seen.iter().any(|&span| overlaps(span, hit.span));
    seen.push(hit.span);
    if !shadowed {
        hits.push(hit);
    }
}

/// Cherche le second terme de la negation apres « ne + modal ».
/// Renvoie `None` si la portee de la negation ne peut pas etre etablie.
fn resolve_negation(folded: &Folded, span: CharSpan) -> Option<Hit> {
    let words = folded.words_after(span.1, lexicon::NEGATION_WINDOW + 1);
    let window = &words[..words.len().min(lexicon::NEGATION_WINDOW)];
    let position = window
        .iter()
        .position(|(word, _)| lexicon::NEGATORS.contains(&word.as_str()))?;
    let (negator, negator_span) = &window[position];

    if negator == "pas"
        && let Some((adverb, adverb_span)) = words.get(position + 1)
        && lexicon::EXEMPTING_ADVERBS.contains(&adverb.as_str())
    {
        return Some(Hit {
            modality: Modality::Exemption,
            span: (span.0, adverb_span.1),
            nominal: false,
        });
    }
    Some(Hit {
        modality: Modality::Prohibition,
        span: (span.0, negator_span.1),
        nominal: false,
    })
}

/// « May » avec une majuscule au milieu d'une phrase designe le mois.
fn is_month_name(text: &str, folded: &Folded, span: CharSpan) -> bool {
    let (start, end) = folded.bytes(span);
    let word = &text[start..end];
    span.0 > 0 && word.starts_with('M') && word != "MAY"
}

fn all_matches(folded: &Folded, markers: &[&str]) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = markers
        .iter()
        .flat_map(|marker| folded.find_all(marker))
        .collect();
    spans.sort();
    spans
}

fn first_match(folded: &Folded, markers: &[&str]) -> Option<CharSpan> {
    all_matches(folded, markers).into_iter().next()
}

fn find_conditions(text: &str, folded: &Folded, vague: &[CharSpan]) -> Vec<ConditionMatch> {
    let tables = [
        (ConditionKind::If, lexicon::CONDITION_IF),
        (ConditionKind::When, lexicon::CONDITION_WHEN),
        (
            ConditionKind::ProvidedThat,
            lexicon::CONDITION_PROVIDED_THAT,
        ),
        (ConditionKind::Exception, lexicon::CONDITION_EXCEPTION),
    ];
    let mut candidates: Vec<(ConditionKind, CharSpan)> = Vec::new();
    for (kind, markers) in tables {
        for marker in markers {
            for span in folded.find_all(marker) {
                candidates.push((kind, span));
            }
        }
    }
    // Le marqueur le plus long l'emporte (« sauf si » avant « si »).
    candidates.sort_by_key(|&(_, span)| (std::cmp::Reverse(span.1 - span.0), span));

    let mut kept: Vec<(ConditionKind, CharSpan)> = Vec::new();
    for (kind, span) in candidates {
        // « si possible », « des que possible » : termes vagues, pas des conditions.
        if vague.iter().any(|&v| overlaps(v, span)) {
            continue;
        }
        if kept.iter().any(|&(_, other)| overlaps(other, span)) {
            continue;
        }
        kept.push((kind, span));
    }
    kept.sort_by_key(|&(_, span)| span);

    kept.into_iter()
        .filter_map(|(kind, marker)| {
            let end = clause_end(folded, marker.1);
            if end <= marker.1 {
                return None;
            }
            let (start_byte, end_byte) = folded.bytes((marker.0, end));
            let clause = &text[start_byte..end_byte];
            (clause.trim_end().len() > folded.bytes(marker).1 - start_byte).then(|| {
                ConditionMatch {
                    kind,
                    span: (start_byte, start_byte + clause.trim_end().len()),
                }
            })
        })
        .collect()
}

/// Fin de la clause : premiere virgule suivie d'un blanc, sinon fin de
/// l'enonce hors ponctuation finale.
fn clause_end(folded: &Folded, from: usize) -> usize {
    let length = folded.len();
    for index in from..length {
        if folded.char_at(index) == Some(',') && folded.char_at(index + 1) == Some(' ') {
            return index;
        }
    }
    let mut end = length;
    while end > from
        && folded
            .char_at(end - 1)
            .is_some_and(|c| matches!(c, '.' | ';' | '!' | '?' | ' '))
    {
        end -= 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(text: &str) -> Option<Category> {
        classify(text).category
    }

    fn kinds(text: &str) -> Vec<UncertaintyKind> {
        classify(text).flags.into_iter().map(|f| f.kind).collect()
    }

    fn condition_texts(text: &str) -> Vec<(ConditionKind, String)> {
        classify(text)
            .conditions
            .into_iter()
            .map(|c| (c.kind, text[c.span.0..c.span.1].to_string()))
            .collect()
    }

    fn modality(text: &str) -> String {
        let (start, end) = classify(text).modality.unwrap();
        text[start..end].to_string()
    }

    // ------------------------------------------------------------ obligations

    #[test]
    fn obligations_in_french_and_english() {
        for text in [
            "Le prestataire doit garantir la confidentialit\u{e9}.",
            "Les candidats devront fournir une attestation.",
            "La visite du site est obligatoire.",
            "Le titulaire est tenu de respecter les d\u{e9}lais.",
            "The supplier shall provide a quality plan.",
            "Bidders must submit three references.",
            "Attendance at the site visit is mandatory.",
        ] {
            assert_eq!(category(text), Some(Category::Obligation), "{text}");
        }
    }

    #[test]
    fn marker_followed_by_punctuation_is_detected() {
        assert_eq!(
            category("Le candidat doit, sans d\u{e9}lai, informer l'acheteur."),
            Some(Category::Obligation)
        );
    }

    #[test]
    fn descriptive_sentences_carry_no_category() {
        for text in [
            "La prestation concerne plusieurs langues europ\u{e9}ennes.",
            "Objet : services linguistiques multilingues.",
            "Offers are due by 15 May 2026.",
            "Il sera peut-\u{ea}tre n\u{e9}cessaire de pr\u{e9}voir une r\u{e9}union.",
        ] {
            assert_eq!(category(text), None, "{text}");
        }
    }

    // --------------------------------------------------------------- negations

    #[test]
    fn french_negations_are_prohibitions() {
        for text in [
            "Le prestataire ne doit pas transmettre les documents \u{e0} des tiers.",
            "Le titulaire ne doit en aucun cas sous-traiter la prestation.",
            "Les candidats ne peuvent pas modifier le bordereau.",
            "Le prestataire ne devra jamais conserver de copie.",
            "Le prix ne pourra, sous aucun pr\u{e9}texte, \u{ea}tre r\u{e9}vis\u{e9}.",
            "Il est interdit de communiquer les donn\u{e9}es.",
            "La sous-traitance est strictement interdite.",
            "Le candidat n\u{2019}est pas autoris\u{e9} \u{e0} pr\u{e9}senter une variante.",
        ] {
            assert_eq!(category(text), Some(Category::Prohibition), "{text}");
        }
    }

    #[test]
    fn english_negations_are_prohibitions() {
        for text in [
            "The supplier must not disclose the documents.",
            "The contractor shall not subcontract the services.",
            "Bidders may not alter the price schedule.",
            "Subcontracting is strictly prohibited.",
            "The supplier cannot assign the contract.",
        ] {
            assert_eq!(category(text), Some(Category::Prohibition), "{text}");
        }
    }

    #[test]
    fn negation_keeps_the_whole_marker_as_modality() {
        assert_eq!(
            modality("Le titulaire ne doit en aucun cas sous-traiter."),
            "ne doit en aucun"
        );
        assert_eq!(modality("The supplier MUST NOT disclose."), "MUST NOT");
    }

    #[test]
    fn negated_obligation_is_not_reported_as_obligation() {
        let analysis = classify("Le prestataire ne doit pas transmettre les documents.");
        assert_eq!(analysis.category, Some(Category::Prohibition));
        assert!(analysis.flags.is_empty());
    }

    #[test]
    fn negation_of_necessity_is_an_exemption_not_a_prohibition() {
        assert_eq!(
            category("Le candidat ne doit pas n\u{e9}cessairement disposer d'un bureau local."),
            Some(Category::Exemption)
        );
        assert_eq!(
            category("The supplier shall not be required to provide a bank guarantee."),
            Some(Category::Exemption)
        );
        assert!(
            classify("The supplier shall not be required to provide a bank guarantee.")
                .flags
                .is_empty()
        );
    }

    // -------------------------------------------------------------- exemptions

    #[test]
    fn exemptions_in_french_and_english() {
        for text in [
            "Il n'est pas obligatoire de disposer d'un bureau local.",
            "La certification ISO 9001 n\u{2019}est pas requise.",
            "Les PME sont dispens\u{e9}es de fournir une garantie bancaire.",
            "Le candidat n'a pas \u{e0} fournir de traduction asserment\u{e9}e.",
            "La visite est non obligatoire.",
            "A bank guarantee is not required.",
            "Small businesses are exempt from the turnover threshold.",
            "Bidders need not provide certified translations.",
            "A local office is not mandatory.",
        ] {
            let analysis = classify(text);
            assert_eq!(analysis.category, Some(Category::Exemption), "{text}");
            assert!(analysis.flags.is_empty(), "{text}");
        }
    }

    // ------------------------------------------------------------- facultatif

    #[test]
    fn optional_provisions_in_french_and_english() {
        for text in [
            "Le candidat peut proposer des prestations compl\u{e9}mentaires.",
            "La visite du site est facultative.",
            "Les candidats ont la possibilit\u{e9} de pr\u{e9}senter une variante.",
            "The bidder may submit variants.",
            "A site visit is optional.",
        ] {
            assert_eq!(category(text), Some(Category::Optional), "{text}");
        }
    }

    #[test]
    fn nominal_markers_are_identified() {
        assert!(classify("5. Possibilit\u{e9}s facultatives").nominal);
        assert!(classify("Assurance obligatoire").nominal);
        assert!(!classify("Le candidat doit fournir une attestation obligatoire").nominal);
        assert!(!classify("3. Le prestataire doit livrer le rapport").nominal);
    }

    // -------------------------------------------------------------- conditions

    #[test]
    fn leading_condition_makes_a_conditional_obligation() {
        let text =
            "Si le volume d\u{e9}passe 100 000 mots, le prestataire doit proposer un calendrier.";
        assert_eq!(category(text), Some(Category::ConditionalObligation));
        assert_eq!(
            condition_texts(text),
            vec![(
                ConditionKind::If,
                "Si le volume d\u{e9}passe 100 000 mots".to_string()
            )]
        );
    }

    #[test]
    fn trailing_condition_runs_to_the_end_of_the_sentence() {
        let text = "The supplier shall notify the buyer if a delay exceeds 2,5 days.";
        assert_eq!(category(text), Some(Category::ConditionalObligation));
        assert_eq!(
            condition_texts(text),
            vec![(ConditionKind::If, "if a delay exceeds 2,5 days".to_string())]
        );
    }

    #[test]
    fn condition_kinds_are_distinguished() {
        assert_eq!(
            condition_texts("Lorsqu'un retard survient, le titulaire doit pr\u{e9}venir.")[0].0,
            ConditionKind::When
        );
        assert_eq!(
            condition_texts("En cas de retard, le titulaire doit pr\u{e9}venir l'acheteur.")[0].0,
            ConditionKind::If
        );
        assert_eq!(
            condition_texts("The supplier must deliver, provided that the order is confirmed.")[0],
            (
                ConditionKind::ProvidedThat,
                "provided that the order is confirmed".to_string()
            )
        );
        assert_eq!(
            condition_texts("Le titulaire doit livrer sous 5 jours, sauf si l'acheteur y renonce.")
                [0],
            (
                ConditionKind::Exception,
                "sauf si l'acheteur y renonce".to_string()
            )
        );
        assert_eq!(
            condition_texts("The supplier shall deliver on site unless otherwise agreed.")[0],
            (
                ConditionKind::Exception,
                "unless otherwise agreed".to_string()
            )
        );
    }

    #[test]
    fn conditions_attach_to_other_categories_without_renaming_them() {
        let prohibition =
            "En cas de sous-traitance, le titulaire ne doit pas transf\u{e9}rer les donn\u{e9}es.";
        assert_eq!(category(prohibition), Some(Category::Prohibition));
        assert_eq!(condition_texts(prohibition).len(), 1);

        let optional = "If the volume is below 10,000 words, the supplier may invoice a flat fee.";
        assert_eq!(category(optional), Some(Category::Optional));
        assert_eq!(
            condition_texts(optional)[0].1,
            "If the volume is below 10,000 words"
        );
    }

    #[test]
    fn unconditional_obligation_has_no_condition() {
        let analysis = classify("Le prestataire doit garantir la confidentialit\u{e9}.");
        assert_eq!(analysis.category, Some(Category::Obligation));
        assert!(analysis.conditions.is_empty());
    }

    // ------------------------------------------------------------- ambiguites

    #[test]
    fn pending_information_is_an_uncertainty_only() {
        let analysis = classify("La date limite de d\u{e9}p\u{f4}t reste \u{e0} confirmer.");
        assert_eq!(analysis.category, None);
        assert_eq!(
            kinds("La date limite de d\u{e9}p\u{f4}t reste \u{e0} confirmer."),
            vec![UncertaintyKind::PendingInformation]
        );
        assert_eq!(
            kinds("The delivery address is TBD."),
            vec![UncertaintyKind::PendingInformation]
        );
    }

    #[test]
    fn weak_modality_is_never_promoted_to_an_obligation() {
        for text in [
            "Le prestataire devrait fournir un rapport mensuel.",
            "The supplier should provide a monthly report.",
            "Il est recommand\u{e9} de visiter le site.",
        ] {
            let analysis = classify(text);
            assert_eq!(analysis.category, None, "{text}");
            assert_eq!(
                analysis.flags[0].kind,
                UncertaintyKind::WeakModality,
                "{text}"
            );
        }
    }

    #[test]
    fn vague_term_inside_a_requirement_is_flagged() {
        let text = "Le prestataire doit, si possible, livrer sous 48 heures.";
        let analysis = classify(text);
        assert_eq!(analysis.category, Some(Category::Obligation));
        assert!(
            analysis.conditions.is_empty(),
            "« si possible » n'est pas une condition"
        );
        assert_eq!(kinds(text), vec![UncertaintyKind::VagueTerm]);
        let (start, end) = analysis.flags[0].marker;
        assert_eq!(&text[start..end], "si possible");

        assert_eq!(
            kinds("The supplier shall respond within a reasonable time."),
            vec![UncertaintyKind::VagueTerm]
        );
    }

    #[test]
    fn vague_term_without_normative_content_is_not_flagged() {
        assert!(kinds("Le march\u{e9} porte sur un volume raisonnable.").is_empty());
    }

    #[test]
    fn pending_information_inside_a_requirement_is_flagged() {
        let text = "Le prestataire doit livrer \u{e0} une adresse \u{e0} pr\u{e9}ciser.";
        assert_eq!(category(text), Some(Category::Obligation));
        assert_eq!(kinds(text), vec![UncertaintyKind::PendingInformation]);
    }

    #[test]
    fn several_modalities_in_one_sentence_are_flagged() {
        let text = "Le candidat doit fournir un m\u{e9}moire et peut joindre des annexes.";
        let analysis = classify(text);
        assert_eq!(analysis.category, Some(Category::Obligation));
        assert_eq!(kinds(text), vec![UncertaintyKind::MultipleModalities]);
        let (start, end) = analysis.flags[0].marker;
        assert_eq!(&text[start..end], "peut");
    }

    #[test]
    fn negation_without_second_term_is_flagged_not_guessed() {
        let text = "Le contrat ne peut \u{ea}tre modifi\u{e9} que par avenant \u{e9}crit.";
        let analysis = classify(text);
        assert_eq!(analysis.category, None);
        assert_eq!(kinds(text), vec![UncertaintyKind::AmbiguousNegation]);
        let (start, end) = analysis.flags[0].marker;
        assert_eq!(&text[start..end], "ne peut");

        let restrictive = "Le titulaire ne doit utiliser que les outils valid\u{e9}s.";
        assert_eq!(category(restrictive), None);
        assert_eq!(kinds(restrictive), vec![UncertaintyKind::AmbiguousNegation]);
    }
}
