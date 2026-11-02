use clap::{Parser, Subcommand};
#[cfg(test)]
mod conformance_tests;
mod parse;
mod render;
mod runtime;
mod validate;

use crate::{
    parse::load_diagram,
    runtime::{apply_choice, apply_report, Run},
    validate::validate_diagram,
};
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "path", about = "Validate and follow Path diagrams")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Validate {
        file: PathBuf,
    },
    Render {
        file: PathBuf,
        #[arg(long)]
        ascii: bool,
    },
    Run {
        #[arg(long, global = true)]
        state_dir: Option<PathBuf>,
        #[command(subcommand)]
        command: RunCommand,
    },
}
#[derive(Subcommand)]
enum RunCommand {
    Start {
        file: PathBuf,
    },
    Status {
        run_id: String,
    },
    Next {
        run_id: String,
    },
    Choose {
        run_id: String,
        activation_id: String,
        choice_id: String,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        note: Option<String>,
    },
    Report {
        run_id: String,
        activation_id: String,
        status: String,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        note: Option<String>,
    },
    History {
        run_id: String,
    },
    Render {
        run_id: String,
        #[arg(long)]
        ascii: bool,
    },
}
enum CommandOutput {
    Data(Value),
    Text(String),
}
#[derive(Debug)]
struct CliError {
    code: String,
    message: String,
    details: Value,
    exit: i32,
}
impl CliError {
    fn new(code: &str, message: impl Into<String>, details: Value, exit: i32) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details,
            exit,
        }
    }
    fn io(error: impl std::fmt::Display) -> Self {
        Self::new("io-error", error.to_string(), json!({}), 1)
    }
    fn corrupt(message: impl Into<String>) -> Self {
        Self::new("corrupt-run", message, json!({}), 1)
    }
}
impl From<runtime::RuntimeError> for CliError {
    fn from(e: runtime::RuntimeError) -> Self {
        let exit = match e.code.as_str() {
            "revision-conflict" | "run-completed" => 3,
            "invalid-choice" | "invalid-report" | "inactive-activation" | "operation-mismatch" => 2,
            _ => 1,
        };
        Self::new(&e.code, e.message, e.details, exit)
    }
}
fn document(path: &Path) -> Result<Value, CliError> {
    let encoding = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        _ => {
            return Err(CliError::new(
                "invalid-document",
                "expected .json, .yaml, or .yml",
                json!({"diagnostics":[{
            "code":"invalid-encoding","message":"unsupported file extension","pointer":""}]}),
                2,
            ))
        }
    };
    let source = fs::read_to_string(path).map_err(CliError::io)?;
    let value = load_diagram(&source, encoding).map_err(|diagnostics| {
        CliError::new(
            "invalid-document",
            "invalid diagram encoding",
            json!({"diagnostics":diagnostics}),
            2,
        )
    })?;
    validate_diagram(&value).map_err(|diagnostics| {
        CliError::new(
            "invalid-document",
            "invalid diagram",
            json!({"diagnostics":diagnostics}),
            2,
        )
    })?;
    Ok(value)
}
fn state_dir(dir: Option<PathBuf>) -> Result<PathBuf, CliError> {
    match dir {
        Some(path) => Ok(path),
        None => Ok(std::env::current_dir()
            .map_err(CliError::io)?
            .join(".path/runs")),
    }
}
fn run_path(dir: &Path, id: &str) -> Result<PathBuf, CliError> {
    if id.is_empty()
        || !id
            .as_bytes()
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
    {
        return Err(CliError::new(
            "invalid-arguments",
            "invalid run ID",
            json!({}),
            2,
        ));
    }
    Ok(dir.join(format!("{id}.json")))
}
struct Lock(File);
impl Lock {
    fn acquire(dir: &Path, id: &str) -> Result<Self, CliError> {
        let path = dir.join(format!("{id}.lock"));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(CliError::io)?;
        file.lock().map_err(CliError::io)?;
        Ok(Self(file))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn read_run(dir: &Path, id: &str) -> Result<Run, CliError> {
    let path = run_path(dir, id)?;
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(CliError::new(
                "run-not-found",
                format!("run {id} not found"),
                json!({}),
                3,
            ))
        }
        Err(e) => return Err(CliError::io(e)),
    };
    let run: Run = serde_json::from_slice(&bytes).map_err(|e| CliError::corrupt(e.to_string()))?;
    if run.run_id != id {
        return Err(CliError::corrupt("stored run ID mismatch"));
    }
    run.check_stored()
        .map_err(|e| CliError::new(&e.code, e.message, e.details, 1))?;
    Ok(run)
}
fn save_run(dir: &Path, run: &Run, new_run: bool) -> Result<(), CliError> {
    let path = run_path(dir, &run.run_id)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(CliError::io)?;
    serde_json::to_writer(&mut temp, run).map_err(CliError::io)?;
    temp.write_all(b"\n").map_err(CliError::io)?;
    temp.as_file().sync_all().map_err(CliError::io)?;
    if new_run {
        temp.persist_noclobber(&path).map_err(CliError::io)?;
    } else {
        temp.persist(&path).map_err(CliError::io)?;
    }
    File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(CliError::io)?;
    Ok(())
}
fn run_command(command: RunCommand, dir: &Path) -> Result<CommandOutput, CliError> {
    match command {
        RunCommand::Start { file } => {
            let diagram = document(&file)?;
            fs::create_dir_all(dir).map_err(CliError::io)?;
            for _ in 0..10 {
                let id = Uuid::new_v4().to_string();
                let _lock = Lock::acquire(dir, &id)?;
                if run_path(dir, &id)?.exists() {
                    continue;
                }
                let run = Run::start(diagram.clone(), id)?;
                save_run(dir, &run, true)?;
                return Ok(CommandOutput::Data(
                    json!({"diagram":run.diagram,"view":run.view()}),
                ));
            }
            Err(CliError::new(
                "io-error",
                "could not allocate a unique run ID",
                json!({}),
                1,
            ))
        }
        RunCommand::Status { run_id } => {
            let run = read_run(dir, &run_id)?;
            Ok(CommandOutput::Data(
                json!({"diagram":run.diagram,"view":run.view()}),
            ))
        }
        RunCommand::Next { run_id } => Ok(CommandOutput::Data(read_run(dir, &run_id)?.next())),
        RunCommand::History { run_id } => {
            Ok(CommandOutput::Data(read_run(dir, &run_id)?.history()))
        }
        RunCommand::Render { run_id, ascii } => {
            let run = read_run(dir, &run_id)?;
            Ok(CommandOutput::Text(render::render(
                &run.diagram,
                Some(&run.view()),
                ascii,
            )))
        }
        RunCommand::Choose {
            run_id,
            activation_id,
            choice_id,
            revision,
            note,
        } => {
            if !run_path(dir, &run_id)?.exists() {
                return Err(CliError::new(
                    "run-not-found",
                    "run not found",
                    json!({}),
                    3,
                ));
            }
            let _lock = Lock::acquire(dir, &run_id)?;
            let mut run = read_run(dir, &run_id)?;
            apply_choice(
                &mut run,
                revision,
                &activation_id,
                &choice_id,
                note.as_deref(),
            )?;
            save_run(dir, &run, false)?;
            Ok(CommandOutput::Data(
                json!({"diagram":run.diagram,"view":run.view()}),
            ))
        }
        RunCommand::Report {
            run_id,
            activation_id,
            status,
            revision,
            note,
        } => {
            if !run_path(dir, &run_id)?.exists() {
                return Err(CliError::new(
                    "run-not-found",
                    "run not found",
                    json!({}),
                    3,
                ));
            }
            let _lock = Lock::acquire(dir, &run_id)?;
            let mut run = read_run(dir, &run_id)?;
            apply_report(&mut run, revision, &activation_id, &status, note.as_deref())?;
            save_run(dir, &run, false)?;
            Ok(CommandOutput::Data(
                json!({"diagram":run.diagram,"view":run.view()}),
            ))
        }
    }
}
fn execute(cli: Cli) -> Result<CommandOutput, CliError> {
    match cli.command {
        Command::Validate { file } => {
            let diagram = document(&file)?;
            Ok(CommandOutput::Data(
                json!({"valid":true,"diagramId":diagram["id"],"kind":diagram["kind"],
                "formatVersion":diagram["formatVersion"]}),
            ))
        }
        Command::Render { file, ascii } => {
            let diagram = document(&file)?;
            Ok(CommandOutput::Text(render::render(&diagram, None, ascii)))
        }
        Command::Run {
            state_dir: dir,
            command,
        } => run_command(command, &state_dir(dir)?),
    }
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let wants_json = args.iter().any(|s| s == "--json");
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            if wants_json {
                println!(
                    "{}",
                    json!({"ok":false,"error":{"code":"invalid-arguments",
                    "message":e.to_string(),"details":{}}})
                );
            } else {
                eprint!("{e}");
            }
            std::process::exit(2);
        }
    };
    let output = execute(cli);
    match output {
        Ok(CommandOutput::Data(data)) => {
            if wants_json {
                println!("{}", json!({"ok":true,"data":data}));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).expect("JSON output")
                );
            }
        }
        Ok(CommandOutput::Text(output)) => {
            if wants_json {
                println!("{}", json!({"ok":true,"data":{"text":output}}));
            } else {
                print!("{output}");
            }
        }
        Err(e) => {
            if wants_json {
                println!(
                    "{}",
                    json!({"ok":false,"error":{"code":e.code,
                "message":e.message,"details":e.details}})
                );
            } else {
                eprintln!("{}: {}", e.code, e.message);
            }
            std::process::exit(e.exit);
        }
    }
}
