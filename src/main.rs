use alchemist::{bytecode::Chunk, compile_source, vm::Vm};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "alchemist", version, about = "A from-scratch compiler that lowers a small language to bytecode and runs it on a bundled stack VM.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile and run a source file
    Run { file: PathBuf },
    /// Compile a source file to bytecode
    Build {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Show the instructions inside a compiled bytecode file
    Disasm { file: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { file } => run_cmd(&file),
        Command::Build { file, output } => build_cmd(&file, &output),
        Command::Disasm { file } => disasm_cmd(&file),
    }
}

fn read_source(path: &PathBuf) -> Result<String, ExitCode> {
    fs::read_to_string(path).map_err(|e| {
        eprintln!("error: could not read '{}': {}", path.display(), e);
        ExitCode::FAILURE
    })
}

fn run_cmd(file: &PathBuf) -> ExitCode {
    let src = match read_source(file) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let chunk = match compile_source(&src) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    let mut machine = Vm::new(&chunk);
    match machine.run() {
        Ok(()) => {
            for line in &machine.output {
                println!("{}", line);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            for line in &machine.output {
                println!("{}", line);
            }
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}

fn build_cmd(file: &PathBuf, output: &PathBuf) -> ExitCode {
    let src = match read_source(file) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let chunk = match compile_source(&src) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::write(output, chunk.to_text()) {
        eprintln!("error: could not write '{}': {}", output.display(), e);
        return ExitCode::FAILURE;
    }
    println!("wrote {}", output.display());
    ExitCode::SUCCESS
}

fn disasm_cmd(file: &PathBuf) -> ExitCode {
    let text = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", file.display(), e);
            return ExitCode::FAILURE;
        }
    };
    let chunk = match Chunk::from_text(&text) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: could not parse bytecode file: {}", e);
            return ExitCode::FAILURE;
        }
    };
    print!("{}", chunk.to_text());
    ExitCode::SUCCESS
}
