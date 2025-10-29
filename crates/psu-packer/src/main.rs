use std::path::PathBuf;

use argh::FromArgs;
use colored::Colorize;
use psu_packer::{load_manifest, pack_project, PackOptions, PsuPackError, TimestampPolicy};

#[derive(Debug, FromArgs)]
/// Package a folder described by psu.toml into a deterministic PSU archive.
struct Args {
    /// folder containing psu.toml
    #[argh(positional)]
    folder: String,

    /// output path for the generated PSU (defaults to <config.name>.psu)
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// override timestamp policy (fixed, source, now)
    #[argh(option, from_str_fn(parse_policy))]
    timestamp_policy: Option<TimestampPolicy>,

    /// verify the produced PSU structure after writing
    #[argh(switch)]
    verify: bool,
}

fn main() {
    let args: Args = argh::from_env();
    if let Err(err) = run(args) {
        report_error(err);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), PsuPackError> {
    let root = PathBuf::from(&args.folder);
    let manifest = load_manifest(&root)?;

    let pack = pack_project(
        &root,
        &manifest,
        PackOptions {
            timestamp_policy: args.timestamp_policy,
            verify: args.verify,
        },
    )?;

    let output = args
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(format!("{}.psu", manifest.name)));

    std::fs::write(&output, &pack.bytes).map_err(|source| PsuPackError::Io {
        path: output.clone(),
        source,
    })?;

    println!(
        "{} {} ({})",
        "Packed".green(),
        output.display(),
        format!("{} files", pack.files.len()).dimmed()
    );

    for file in &pack.files {
        println!("  {} {}", "•".dimmed(), file.name);
    }

    if args.verify {
        println!("{}", "Verification completed".green());
    }

    Ok(())
}

fn parse_policy(value: &str) -> Result<TimestampPolicy, String> {
    value
        .parse()
        .map_err(|_| "timestamp policy must be one of: fixed, source, now".to_string())
}

fn report_error(error: PsuPackError) {
    eprintln!("{} {}", "error:".red().bold(), error);
    if let Some(issues) = error.issues() {
        for issue in issues {
            eprintln!("  {} {}", issue.field.blue().bold(), issue.message.white());
            if let Some(help) = &issue.help {
                eprintln!("      {} {}", "hint:".yellow(), help);
            }
        }
    }
}
