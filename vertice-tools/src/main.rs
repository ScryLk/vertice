//! CLI: `cargo run -p vertice-tools -- <validate|generate>`.
//!
//! Sem clap, sem configuração: dois comandos não justificam dependência.

use std::process::ExitCode;

use vertice_tools::{generate, validate, workspace_root};

const USAGE: &str = "uso: cargo run -p vertice-tools -- <validate|generate>";

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    let root = workspace_root();

    let outcome = match command.as_deref() {
        Some("validate") => validate::run(&root),
        Some("generate") => generate::run(&root),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    match outcome {
        Ok(summary) => {
            println!("OK: {summary}.");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("FALHA: {} problema(s) no catálogo.", errors.len());
            for error in &errors {
                eprintln!("  - {error}");
            }
            ExitCode::FAILURE
        }
    }
}
