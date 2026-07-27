use std::process::ExitCode;

use cards::{TomlSchemaKind, validate_toml_path};

fn main() -> ExitCode {
    let mut kind = TomlSchemaKind::Card;
    let mut paths = Vec::new();

    for arg in std::env::args().skip(1) {
        if arg == "--token" {
            kind = TomlSchemaKind::Token;
            continue;
        }

        paths.push(arg);
    }

    if paths.is_empty() {
        eprintln!("usage: validate_card_toml [--token] <paths...>");
        return ExitCode::FAILURE;
    }

    let mut valid = true;
    for path in paths {
        if let Err(errors) = validate_toml_path(kind, &path) {
            valid = false;
            for error in errors {
                eprintln!("{error}");
            }
        }
    }

    if valid {
        return ExitCode::SUCCESS;
    }

    ExitCode::FAILURE
}
