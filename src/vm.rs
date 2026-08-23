use crate::bytecode::{Chunk, Instr};
use crate::errors::RuntimeError;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "{}", s),
        }
    }
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
        }
    }
}

struct Frame {
    return_addr: usize,
    locals: Vec<Value>,
}

pub struct Vm<'a> {
    chunk: &'a Chunk,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    ip: usize,
    pub output: Vec<String>,
}

type VResult<T> = Result<T, RuntimeError>;

impl<'a> Vm<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        let main_frame = Frame { return_addr: chunk.code.len(), locals: vec![Value::Int(0); chunk.main_locals] };
        Vm { chunk, stack: Vec::new(), frames: vec![main_frame], ip: 0, output: Vec::new() }
    }

    fn pop(&mut self) -> VResult<Value> {
        self.stack.pop().ok_or(RuntimeError::StackUnderflow)
    }

    fn pop_int(&mut self) -> VResult<i64> {
        match self.pop()? {
            Value::Int(n) => Ok(n),
            other => Err(RuntimeError::TypeError(format!("expected int, found {}", other.type_name()))),
        }
    }

    fn pop_bool(&mut self) -> VResult<bool> {
        match self.pop()? {
            Value::Bool(b) => Ok(b),
            other => Err(RuntimeError::TypeError(format!("expected bool, found {}", other.type_name()))),
        }
    }

    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    pub fn run(&mut self) -> VResult<()> {
        loop {
            if self.ip >= self.chunk.code.len() {
                return Err(RuntimeError::BadJumpTarget(self.ip));
            }
            let instr = self.chunk.code[self.ip].clone();
            self.ip += 1;

            match instr {
                Instr::PushInt(n) => self.stack.push(Value::Int(n)),
                Instr::PushBool(b) => self.stack.push(Value::Bool(b)),
                Instr::PushStr(s) => self.stack.push(Value::Str(s)),
                Instr::Pop => {
                    self.pop()?;
                }
                Instr::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let result = match (a, b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x.wrapping_add(y)),
                        (Value::Str(x), Value::Str(y)) => Value::Str(x + &y),
                        (a, b) => {
                            return Err(RuntimeError::TypeError(format!(
                                "cannot add {} and {}",
                                a.type_name(),
                                b.type_name()
                            )))
                        }
                    };
                    self.stack.push(result);
                }
                Instr::Sub => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Int(a.wrapping_sub(b)));
                }
                Instr::Mul => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Int(a.wrapping_mul(b)));
                }
                Instr::Div => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    if b == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.stack.push(Value::Int(a.wrapping_div(b)));
                }
                Instr::Mod => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    if b == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.stack.push(Value::Int(a.wrapping_rem(b)));
                }
                Instr::Neg => {
                    let a = self.pop_int()?;
                    self.stack.push(Value::Int(-a));
                }
                Instr::Not => {
                    let a = self.pop_bool()?;
                    self.stack.push(Value::Bool(!a));
                }
                Instr::And => {
                    let b = self.pop_bool()?;
                    let a = self.pop_bool()?;
                    self.stack.push(Value::Bool(a && b));
                }
                Instr::Or => {
                    let b = self.pop_bool()?;
                    let a = self.pop_bool()?;
                    self.stack.push(Value::Bool(a || b));
                }
                Instr::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(a == b));
                }
                Instr::NotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(a != b));
                }
                Instr::Lt => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Bool(a < b));
                }
                Instr::Gt => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Bool(a > b));
                }
                Instr::LtEq => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Bool(a <= b));
                }
                Instr::GtEq => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.stack.push(Value::Bool(a >= b));
                }
                Instr::Load(slot) => {
                    let v = self
                        .frame()
                        .locals
                        .get(slot)
                        .cloned()
                        .ok_or(RuntimeError::TypeError(format!("no local at slot {}", slot)))?;
                    self.stack.push(v);
                }
                Instr::Store(slot) => {
                    let v = self.pop()?;
                    let frame = self.frame();
                    if slot >= frame.locals.len() {
                        frame.locals.resize(slot + 1, Value::Int(0));
                    }
                    frame.locals[slot] = v;
                }
                Instr::Jump(addr) => {
                    self.ip = addr;
                }
                Instr::JumpIfFalse(addr) => {
                    let cond = self.pop_bool()?;
                    if !cond {
                        self.ip = addr;
                    }
                }
                Instr::Call(func_idx, argc) => {
                    let meta = self
                        .chunk
                        .funcs
                        .get(func_idx)
                        .ok_or(RuntimeError::UnknownFunction(func_idx))?;
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let mut locals = vec![Value::Int(0); meta.num_locals];
                    for (i, v) in args.into_iter().enumerate() {
                        locals[i] = v;
                    }
                    self.frames.push(Frame { return_addr: self.ip, locals });
                    self.ip = meta.start;
                }
                Instr::Ret => {
                    let frame = self.frames.pop().ok_or(RuntimeError::StackUnderflow)?;
                    self.ip = frame.return_addr;
                }
                Instr::Print => {
                    let v = self.pop()?;
                    self.output.push(v.to_string());
                }
                Instr::Halt => return Ok(()),
            }
        }
    }
}
