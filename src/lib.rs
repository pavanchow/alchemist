pub mod ast;
pub mod bytecode;
pub mod codegen;
pub mod errors;
pub mod lexer;
pub mod parser;
pub mod vm;

use bytecode::Chunk;
use errors::CompileError;

/// Lex, parse, and lower source text down to a bytecode chunk.
pub fn compile_source(src: &str) -> Result<Chunk, CompileError> {
    let toks = lexer::lex(src).map_err(|e| CompileError::Lex(e.to_string()))?;
    let program = parser::parse(toks).map_err(|e| CompileError::Parse(e.to_string()))?;
    codegen::compile(&program)
}

/// Compile and execute source text, returning the lines printed by `print`.
pub fn run_source(src: &str) -> Result<Vec<String>, String> {
    let chunk = compile_source(src).map_err(|e| e.to_string())?;
    let mut machine = vm::Vm::new(&chunk);
    machine.run().map_err(|e| e.to_string())?;
    Ok(machine.output)
}
