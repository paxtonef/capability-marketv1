use std::env;
use std::fs;
use std::process;

fn escape_json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn extract(tender_id: &str, title: &str, source: &str) -> String {
    let mut requirements = Vec::new();
    let mut evidence = Vec::new();
    let mut uncertainties = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let text = line.trim();
        if text.is_empty() {
            continue;
        }

        let lower = text.to_lowercase();

        let exemption = [
            "n'est pas obligatoire",
            "n’est pas obligatoire",
            "non obligatoire",
            "not required",
            "not mandatory",
        ]
        .iter()
        .any(|marker| lower.contains(marker));

        let uncertainty = [
            "à confirmer",
            "a confirmer",
            "à préciser",
            "a preciser",
            "reste à confirmer",
            "to be confirmed",
            "to be determined",
        ]
        .iter()
        .any(|marker| lower.contains(marker));

        if uncertainty {
            uncertainties.push(format!(
                "{{\"source_line\":{},\"text\":{}}}",
                index + 1,
                escape_json(text)
            ));
        }

        if exemption {
            continue;
        }

        let obligation = [
            "doit ", "doivent ", "devra ", "devront ",
            "obligatoire", "must ", "shall ",
        ]
        .iter()
        .any(|marker| lower.contains(marker));

        if !obligation {
            continue;
        }

        let prohibition = [
            "ne doit pas",
            "ne doivent pas",
            "ne devra pas",
            "must not",
            "shall not",
        ]
        .iter()
        .any(|marker| lower.contains(marker));

        let conditional = [
            "si ",
            "lorsque ",
            "à condition que ",
            "a condition que ",
            "if ",
            "provided that ",
        ]
        .iter()
        .any(|marker| lower.starts_with(marker));

        let category = if prohibition {
            "PROHIBITION"
        } else if conditional {
            "CONDITIONAL_OBLIGATION"
        } else {
            "OBLIGATION"
        };

        let id = format!("REQ-{:04}", requirements.len() + 1);

        requirements.push(format!(
            "{{\"id\":{},\"category\":{},\"text\":{}}}",
            escape_json(&id),
            escape_json(category),
            escape_json(text)
        ));

        evidence.push(format!(
            "{{\"requirement_id\":{},\"source_line\":{},\"excerpt\":{}}}",
            escape_json(&id),
            index + 1,
            escape_json(text)
        ));
    }

    if requirements.is_empty() {
        uncertainties.push(escape_json(
            "Aucune exigence explicite detectee par les regles V0.2"
        ));
    }

    format!(
        "{{\"tender_id\":{},\"title\":{},\"requirements\":[{}],\"evidence\":[{}],\"uncertainties\":[{}]}}",
        escape_json(tender_id),
        escape_json(title),
        requirements.join(","),
        evidence.join(","),
        uncertainties.join(",")
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: procurement TENDER_ID TITLE SOURCE_FILE");
        process::exit(2);
    }

    let source = match fs::read_to_string(&args[3]) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("Erreur de lecture: {error}");
            process::exit(1);
        }
    };

    println!("{}", extract(&args[1], &args[2], &source));
}

#[cfg(test)]
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
