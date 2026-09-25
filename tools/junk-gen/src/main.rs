use std::process::ExitCode;

use junk_gen::{guard::clean_target, Guard, Options, Scale};

const USAGE: &str = "usage: junk-gen <target_dir> [--scale small|medium|large] [--seed N] [--dry-run] [--force]

Fills <target_dir> with a fake Windows-like tree full of junk for testing Fazasanj.
Refuses C:, the system drive, real Windows installs and folders that are not empty.
Point it at a drive root (for example after `subst T: D:\\junk`) so the rules see
T:\\Users\\Test\\... like a real profile.";

fn parse(args: &[String]) -> Result<Options, String> {
    let mut target = None;
    let mut scale = Scale::Small;
    let mut seed = 1u64;
    let mut dry_run = false;
    let mut force = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scale" => {
                let v = it.next().ok_or("--scale needs a value")?;
                scale = Scale::parse(v).ok_or_else(|| format!("unknown scale {v}"))?;
            }
            "--seed" => {
                let v = it.next().ok_or("--seed needs a value")?;
                seed = v.parse().map_err(|_| format!("bad seed {v}"))?;
            }
            "--dry-run" => dry_run = true,
            "--force" => force = true,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s if target.is_none() => target = Some(clean_target(s)),
            s => return Err(format!("unexpected argument {s}")),
        }
    }
    let target = target.ok_or("missing target folder")?;
    Ok(Options { target, scale, seed, dry_run, force })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse(&args) {
        Ok(o) => o,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match junk_gen::run(&opts, &Guard::from_env()) {
        Ok((_, summary)) => {
            println!("{}", opts.target.display());
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
