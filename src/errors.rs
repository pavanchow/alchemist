use std::fmt;

#[derive(Debug, Clone)]
pub enum CompileError {
    Lex(String),
    Parse(String),
    UndefinedVariable { name: String, line: usize },
    UndefinedFunction { name: String, line: usize },
    ArityMismatch { name: String, expected: usize, got: usize, line: usize },
    DuplicateFunction { name: String },
    ReturnOutsideFunction,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Lex(m) => write!(f, "{}", m),
            CompileError::Parse(m) => write!(f, "{}", m),
            CompileError::UndefinedVariable { name, line } => {
                write!(f, "compile error at line {}: undefined variable '{}'", line, name)
            }
            CompileError::UndefinedFunction { name, line } => {
                write!(f, "compile error at line {}: undefined function '{}'", line, name)
            }
            CompileError::ArityMismatch { name, expected, got, line } => write!(
                f,
                "compile error at line {}: function '{}' expects {} argument(s), got {}",
                line, name, expected, got
            ),
            CompileError::DuplicateFunction { name } => {
                write!(f, "compile error: function '{}' is already defined", name)
            }
            CompileError::ReturnOutsideFunction => {
                write!(f, "compile error: 'return' used outside of a function")
            }
        }
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    StackUnderflow,
    DivisionByZero,
    TypeError(String),
    UnknownFunction(usize),
    BadJumpTarget(usize),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow => write!(f, "runtime error: stack underflow"),
            RuntimeError::DivisionByZero => write!(f, "runtime error: division by zero"),
            RuntimeError::TypeError(m) => write!(f, "runtime error: type error: {}", m),
            RuntimeError::UnknownFunction(i) => {
                write!(f, "runtime error: call to unknown function index {}", i)
            }
            RuntimeError::BadJumpTarget(a) => {
                write!(f, "runtime error: jump target {} out of range", a)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}
