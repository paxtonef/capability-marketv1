//! Interface en ligne de commande du runner PROCUREMENT.
//!
//! Autonome : ne depend ni de CPL ni d'une base de donnees.

use std::env;
use std::fs;
use std::process::ExitCode;

use procurement::{ContractVersion, Format, RUNNER_VERSION, model::SCHEMA_NAME, reader};

const USAGE: &str = "\
Usage: procurement [OPTIONS] TENDER_ID TITLE SOURCE_FILE

Extrait les exigences d'un appel d'offres (texte UTF-8) et ecrit un JSON
conforme au contrat cm.procurement.requirements.v1.

Arguments:
  TENDER_ID      Identifiant de l'appel d'offres
  TITLE          Intitule de l'appel d'offres
  SOURCE_FILE    Fichier texte source, ou - pour l'entree standard

Options:
      --contract-version <V>  Profil de sortie : 1.1.0 (defaut) ou 1.0.0 (sortie V0.2)
      --pretty                JSON indente
  -o, --output <FILE>         Ecrit dans FILE au lieu de la sortie standard
  -h, --help                  Affiche cette aide
  -V, --version               Affiche la version

Codes de sortie: 0 succes, 1 erreur de lecture ou d'ecriture, 2 erreur d'usage";

struct Options {
    positionals: Vec<String>,
    version: ContractVersion,
    format: Format,
    output: Option<String>,
}

enum Command {
    Run(Options),
    Help,
    Version,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut options = Options {
        positionals: Vec::new(),
        version: ContractVersion::V1_1,
        format: Format::Compact,
        output: None,
    };
    let mut args = args;
    let mut only_positionals = false;

    while let Some(arg) = args.next() {
        if only_positionals || arg == "-" || !arg.starts_with('-') {
            options.positionals.push(arg);
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => {
                (name.to_string(), Some(value.to_string()))
            }
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("l'option {name} attend une valeur"))
        };
        match name.as_str() {
            "--" => only_positionals = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--pretty" => options.format = Format::Pretty,
            "--contract-version" => {
                let raw = value("--contract-version")?;
                options.version = ContractVersion::parse(&raw).ok_or_else(|| {
                    format!("version de contrat inconnue: {raw} (attendu 1.0.0 ou 1.1.0)")
                })?;
            }
            "-o" | "--output" => options.output = Some(value("--output")?),
            other => return Err(format!("option inconnue: {other}")),
        }
    }

    if options.positionals.len() != 3 {
        return Err(format!(
            "3 arguments attendus, {} recu(s)",
            options.positionals.len()
        ));
    }
    Ok(Command::Run(options))
}

fn main() -> ExitCode {
    let options = match parse_args(env::args().skip(1)) {
        Ok(Command::Run(options)) => options,
        Ok(Command::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            println!("procurement {RUNNER_VERSION} ({SCHEMA_NAME} 1.0.0, 1.1.0)");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("Erreur: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let document = match reader::read_source(&options.positionals[2]) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("Erreur de lecture: {error}");
            return ExitCode::from(1);
        }
    };

    let json = procurement::run(
        &options.positionals[0],
        &options.positionals[1],
        &document,
        options.version,
        options.format,
    );

    match options.output {
        None => println!("{json}"),
        Some(path) => {
            if let Err(error) = fs::write(&path, format!("{json}\n")) {
                eprintln!("Erreur d'ecriture: {error}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
