//! Lexique des marqueurs et recherche insensible a la casse et aux accents.
//!
//! Les marqueurs sont ecrits en minuscules, sans accent, avec l'apostrophe
//! droite. La recherche se fait sur une copie « pliee » de l'enonce qui
//! conserve la correspondance caractere par caractere avec le texte source :
//! les positions trouvees designent donc toujours des tranches exactes.

/// Copie pliee d'un enonce (minuscules, sans accent, blancs unifies).
pub struct Folded {
    chars: Vec<char>,
    /// Decalage en octets de chaque caractere, plus une sentinelle finale.
    offsets: Vec<usize>,
}

/// Intervalle en indices de caracteres dans un [`Folded`].
pub type CharSpan = (usize, usize);

impl Folded {
    pub fn new(text: &str) -> Self {
        let mut chars = Vec::new();
        let mut offsets = Vec::new();
        for (offset, c) in text.char_indices() {
            chars.push(fold_char(c));
            offsets.push(offset);
        }
        offsets.push(text.len());
        Self { chars, offsets }
    }

    /// Convertit un intervalle de caracteres en intervalle d'octets.
    pub fn bytes(&self, span: CharSpan) -> (usize, usize) {
        (self.offsets[span.0], self.offsets[span.1])
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn char_at(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    /// Toutes les occurrences d'un marqueur, delimitees par des frontieres de mot.
    pub fn find_all(&self, marker: &str) -> Vec<CharSpan> {
        let pattern: Vec<char> = marker.chars().collect();
        let mut found = Vec::new();
        if pattern.is_empty() {
            return found;
        }
        let mut index = 0;
        while index < self.chars.len() {
            if let Some(end) = self.match_at(index, &pattern) {
                found.push((index, end));
                index = end;
            } else {
                index += 1;
            }
        }
        found
    }

    fn match_at(&self, start: usize, pattern: &[char]) -> Option<usize> {
        if is_word(pattern[0]) && !self.boundary_before(start) {
            return None;
        }
        let mut cursor = start;
        for &expected in pattern {
            if expected == ' ' {
                if self.chars.get(cursor) != Some(&' ') {
                    return None;
                }
                while self.chars.get(cursor) == Some(&' ') {
                    cursor += 1;
                }
            } else {
                if self.chars.get(cursor) != Some(&expected) {
                    return None;
                }
                cursor += 1;
            }
        }
        if is_word(pattern[pattern.len() - 1]) && !self.boundary_after(cursor) {
            return None;
        }
        Some(cursor)
    }

    fn boundary_before(&self, index: usize) -> bool {
        if index == 0 {
            return true;
        }
        let previous = self.chars[index - 1];
        if is_word(previous) {
            return false;
        }
        // « peut-etre », « c'est-a-dire » : le trait d'union lie les mots.
        !(previous == '-' && index >= 2 && is_word(self.chars[index - 2]))
    }

    fn boundary_after(&self, index: usize) -> bool {
        match self.chars.get(index) {
            None => true,
            Some(&next) if is_word(next) => false,
            Some(&'-') => !self.chars.get(index + 1).is_some_and(|&c| is_word(c)),
            Some(_) => true,
        }
    }

    /// Les `count` mots suivant la position `from`, avec leur intervalle.
    pub fn words_after(&self, from: usize, count: usize) -> Vec<(String, CharSpan)> {
        let mut words = Vec::new();
        let mut index = from;
        while index < self.chars.len() && words.len() < count {
            if is_word(self.chars[index]) {
                let start = index;
                while index < self.chars.len()
                    && (is_word(self.chars[index]) || self.chars[index] == '\'')
                {
                    index += 1;
                }
                words.push((self.chars[start..index].iter().collect(), (start, index)));
            } else {
                index += 1;
            }
        }
        words
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

fn fold_char(c: char) -> char {
    if c.is_whitespace() {
        return ' ';
    }
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        '\u{2019}' | '\u{2018}' | '\u{02bc}' | '`' => '\'',
        'à' | 'â' | 'ä' | 'á' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        other => other,
    }
}

// ------------------------------------------------------------------ modalites

/// Exemptions : l'enonce leve une obligation.
pub const EXEMPTION: &[&str] = &[
    // francais
    "pas obligatoire",
    "pas obligatoires",
    "non obligatoire",
    "non obligatoires",
    "n'est pas tenu",
    "n'est pas tenue",
    "ne sont pas tenus",
    "ne sont pas tenues",
    "pas requis",
    "pas requise",
    "pas requises",
    "pas exige",
    "pas exigee",
    "pas exiges",
    "pas exigees",
    "est dispense",
    "est dispensee",
    "sont dispenses",
    "sont dispensees",
    "est exempte",
    "est exemptee",
    "sont exemptes",
    "sont exemptees",
    "est exempt",
    "sont exempts",
    "n'a pas a",
    "n'ont pas a",
    "n'aura pas a",
    "n'auront pas a",
    "pas necessaire",
    "pas necessaires",
    "sans obligation",
    "aucune obligation",
    // anglais
    "not required",
    "not be required",
    "not mandatory",
    "not compulsory",
    "not obliged",
    "not obligated",
    "need not",
    "does not need to",
    "do not need to",
    "does not have to",
    "do not have to",
    "is exempt",
    "are exempt",
    "exempt from",
    "exempted from",
    "not necessary",
    "no obligation",
    "no requirement to",
    "is waived",
    "are waived",
];

/// Interdictions (hors tournures « ne + modal + negation », traitees a part).
pub const PROHIBITION: &[&str] = &[
    // francais
    "ne saurait",
    "ne sauraient",
    "interdit",
    "interdite",
    "interdits",
    "interdites",
    "interdiction de",
    "interdiction d'",
    "prohibe",
    "prohibee",
    "prohibes",
    "prohibees",
    "n'est pas autorise",
    "n'est pas autorisee",
    "ne sont pas autorises",
    "ne sont pas autorisees",
    "n'est pas admis",
    "n'est pas admise",
    "ne sont pas admis",
    "ne sont pas admises",
    "n'est pas permis",
    "en aucun cas",
    "sous aucun pretexte",
    "s'interdit",
    "s'interdisent",
    // anglais
    "must not",
    "shall not",
    "may not",
    "cannot",
    "can not",
    "mustn't",
    "must never",
    "shall never",
    "may never",
    "not permitted",
    "not allowed",
    "prohibited",
    "forbidden",
    "under no circumstances",
    "in no event",
];

/// Obligations.
pub const OBLIGATION: &[&str] = &[
    // francais
    "doit",
    "doivent",
    "devra",
    "devront",
    "est tenu",
    "est tenue",
    "sont tenus",
    "sont tenues",
    "il incombe",
    "incombe a",
    "il appartient",
    "obligatoire",
    "obligatoires",
    "obligatoirement",
    "imperativement",
    "imperatif",
    "est requis",
    "est requise",
    "sont requis",
    "sont requises",
    "est exige",
    "est exigee",
    "sont exiges",
    "sont exigees",
    "exige que",
    "exigent que",
    "s'engage a",
    "s'engagent a",
    "a l'obligation",
    "ont l'obligation",
    "sous peine de",
    "sous peine d'",
    "a peine de",
    // anglais
    "must",
    "shall",
    "is required",
    "are required",
    "required to",
    "mandatory",
    "compulsory",
    "is obliged to",
    "are obliged to",
    "has to",
    "have to",
    "is responsible for",
    "are responsible for",
    "undertakes to",
    "undertake to",
];

/// Possibilites facultatives.
pub const OPTIONAL: &[&str] = &[
    // francais
    "peut",
    "peuvent",
    "pourra",
    "pourront",
    "a la possibilite",
    "ont la possibilite",
    "a la faculte",
    "ont la faculte",
    "est autorise a",
    "est autorisee a",
    "sont autorises a",
    "sont autorisees a",
    "est libre de",
    "sont libres de",
    "facultatif",
    "facultative",
    "facultatifs",
    "facultatives",
    "optionnel",
    "optionnelle",
    "optionnels",
    "optionnelles",
    "en option",
    "il est possible de",
    "il est loisible",
    // anglais
    "may",
    "is allowed to",
    "are allowed to",
    "is permitted to",
    "are permitted to",
    "is entitled to",
    "are entitled to",
    "optional",
    "optionally",
    "at its discretion",
    "at their discretion",
    "is free to",
    "are free to",
];

/// Marqueurs nominaux ou adjectivaux, frequents dans les titres de section.
pub const NOMINAL: &[&str] = &[
    "obligatoire",
    "obligatoires",
    "non obligatoire",
    "non obligatoires",
    "sans obligation",
    "aucune obligation",
    "interdit",
    "interdite",
    "interdits",
    "interdites",
    "interdiction de",
    "interdiction d'",
    "prohibe",
    "prohibee",
    "prohibes",
    "prohibees",
    "facultatif",
    "facultative",
    "facultatifs",
    "facultatives",
    "optionnel",
    "optionnelle",
    "optionnels",
    "optionnelles",
    "en option",
    "mandatory",
    "compulsory",
    "prohibited",
    "forbidden",
    "optional",
    "no obligation",
];

/// Tournures « ne + modal » dont la negation est cherchee dans les mots suivants.
pub const NE_MODAL: &[&str] = &[
    "ne doit",
    "ne doivent",
    "ne devra",
    "ne devront",
    "ne peut",
    "ne peuvent",
    "ne pourra",
    "ne pourront",
];

/// Second terme d'une negation francaise.
pub const NEGATORS: &[&str] = &[
    "pas",
    "jamais",
    "plus",
    "aucun",
    "aucune",
    "nullement",
    "ni",
];

/// Apres « pas » : la negation porte sur la necessite, non sur l'action.
pub const EXEMPTING_ADVERBS: &[&str] = &["necessairement", "obligatoirement", "forcement"];

/// Nombre de mots examines apres « ne + modal » pour trouver la negation.
pub const NEGATION_WINDOW: usize = 4;

// --------------------------------------------------------------- incertitudes

pub const PENDING: &[&str] = &[
    // francais
    "a confirmer",
    "a preciser",
    "a definir",
    "a determiner",
    "a completer",
    "sera precise",
    "sera precisee",
    "seront precises",
    "seront precisees",
    "sera communique",
    "sera communiquee",
    "seront communiques",
    "seront communiquees",
    "sera confirme",
    "sera confirmee",
    "seront confirmes",
    "seront confirmees",
    "ulterieurement",
    "non encore defini",
    "non encore definie",
    "pas encore defini",
    "pas encore definie",
    "pas encore connu",
    "pas encore connue",
    // anglais
    "to be confirmed",
    "to be determined",
    "to be defined",
    "to be specified",
    "to be announced",
    "to be agreed",
    "tbc",
    "tbd",
    "tba",
    "not yet defined",
    "not yet known",
    "at a later date",
    "in due course",
];

pub const WEAK: &[&str] = &[
    // francais
    "devrait",
    "devraient",
    "souhaitable",
    "souhaitables",
    "recommande",
    "recommandee",
    "recommandes",
    "recommandees",
    "de preference",
    "il convient de",
    "il conviendra de",
    "idealement",
    // anglais
    "should",
    "ought to",
    "is expected to",
    "are expected to",
    "recommended",
    "preferably",
    "ideally",
    "is encouraged",
    "are encouraged",
    "desirable",
    "it is advisable",
];

pub const VAGUE: &[&str] = &[
    // francais
    "si possible",
    "dans la mesure du possible",
    "autant que possible",
    "le cas echeant",
    "si necessaire",
    "si besoin",
    "en tant que de besoin",
    "en principe",
    "eventuellement",
    "et/ou",
    "dans les meilleurs delais",
    "des que possible",
    "en temps utile",
    "raisonnable",
    "raisonnables",
    "raisonnablement",
    "suffisant",
    "suffisante",
    "suffisants",
    "suffisantes",
    "approprie",
    "appropriee",
    "appropries",
    "appropriees",
    "adequat",
    "adequats",
    "adequates",
    "etc",
    // anglais
    "if possible",
    "where possible",
    "if necessary",
    "if applicable",
    "where applicable",
    "where appropriate",
    "as appropriate",
    "as necessary",
    "as needed",
    "as required",
    "and/or",
    "as soon as possible",
    "in a timely manner",
    "reasonable",
    "reasonably",
    "sufficient",
    "appropriate",
    "adequate",
    "to the extent possible",
    "in principle",
];

// ----------------------------------------------------------------- conditions

pub const CONDITION_IF: &[&str] = &[
    "si",
    "s'il",
    "s'ils",
    "dans le cas ou",
    "au cas ou",
    "en cas de",
    "en cas d'",
    "dans l'hypothese ou",
    "if",
    "in case of",
    "in the event of",
    "in the event that",
];

pub const CONDITION_WHEN: &[&str] = &[
    "lorsque",
    "lorsqu'",
    "quand",
    "des que",
    "des qu'",
    "des lors que",
    "des lors qu'",
    "une fois que",
    "when",
    "whenever",
    "as soon as",
];

pub const CONDITION_PROVIDED_THAT: &[&str] = &[
    "a condition que",
    "a condition qu'",
    "a condition de",
    "a condition d'",
    "sous reserve",
    "pour autant que",
    "pour autant qu'",
    "pourvu que",
    "pourvu qu'",
    "provided that",
    "providing that",
    "subject to",
    "as long as",
    "on condition that",
];

pub const CONDITION_EXCEPTION: &[&str] = &[
    "sauf si",
    "sauf s'",
    "sauf",
    "a moins que",
    "a moins qu'",
    "a moins de",
    "a moins d'",
    "excepte",
    "hormis",
    "a l'exception",
    "unless",
    "except",
    "save where",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_regardless_of_case_accents_and_apostrophes() {
        let folded = Folded::new("Il N\u{2019}EST PAS Oblig\u{e9} ; \u{c0} CONFIRMER.");
        assert_eq!(folded.find_all("a confirmer").len(), 1);
        assert_eq!(folded.find_all("n'est pas").len(), 1);
    }

    #[test]
    fn respects_word_boundaries() {
        assert!(
            Folded::new("Le doitage est obligatoirement vu.")
                .find_all("doit")
                .is_empty()
        );
        assert!(
            Folded::new("obligatoirement")
                .find_all("obligatoire")
                .is_empty()
        );
        assert!(
            Folded::new("Il sera peut-\u{ea}tre pr\u{e9}sent.")
                .find_all("peut")
                .is_empty()
        );
        assert_eq!(
            Folded::new("Il doit, sans d\u{e9}lai.")
                .find_all("doit")
                .len(),
            1
        );
    }

    #[test]
    fn spans_map_back_to_exact_source_bytes() {
        let text = "\u{c9}t\u{e9} : le d\u{e9}p\u{f4}t DOIT \u{ea}tre fait.";
        let folded = Folded::new(text);
        let span = folded.find_all("doit")[0];
        let (start, end) = folded.bytes(span);
        assert_eq!(&text[start..end], "DOIT");
    }

    #[test]
    fn tolerates_repeated_whitespace_inside_markers() {
        assert_eq!(Folded::new("must \t not").find_all("must not").len(), 1);
    }
}
