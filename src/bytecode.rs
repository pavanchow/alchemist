use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    PushInt(i64),
    PushBool(bool),
    PushStr(String),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Not,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Load(usize),
    Store(usize),
    Jump(usize),
    JumpIfFalse(usize),
    Call(usize, usize),
    Ret,
    Print,
    Halt,
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::PushInt(n) => write!(f, "PUSH_INT {}", n),
            Instr::PushBool(b) => write!(f, "PUSH_BOOL {}", b),
            Instr::PushStr(s) => write!(f, "PUSH_STR {}", encode_str(s)),
            Instr::Pop => write!(f, "POP"),
            Instr::Add => write!(f, "ADD"),
            Instr::Sub => write!(f, "SUB"),
            Instr::Mul => write!(f, "MUL"),
            Instr::Div => write!(f, "DIV"),
            Instr::Mod => write!(f, "MOD"),
            Instr::Neg => write!(f, "NEG"),
            Instr::Not => write!(f, "NOT"),
            Instr::Eq => write!(f, "EQ"),
            Instr::NotEq => write!(f, "NEQ"),
            Instr::Lt => write!(f, "LT"),
            Instr::Gt => write!(f, "GT"),
            Instr::LtEq => write!(f, "LE"),
            Instr::GtEq => write!(f, "GE"),
            Instr::And => write!(f, "AND"),
            Instr::Or => write!(f, "OR"),
            Instr::Load(i) => write!(f, "LOAD {}", i),
            Instr::Store(i) => write!(f, "STORE {}", i),
            Instr::Jump(a) => write!(f, "JUMP {}", a),
            Instr::JumpIfFalse(a) => write!(f, "JUMP_IF_FALSE {}", a),
            Instr::Call(idx, argc) => write!(f, "CALL {} {}", idx, argc),
            Instr::Ret => write!(f, "RET"),
            Instr::Print => write!(f, "PRINT"),
            Instr::Halt => write!(f, "HALT"),
        }
    }
}

fn encode_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn decode_str(s: &str) -> Option<String> {
    let s = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[derive(Debug, Clone)]
pub struct FuncMeta {
    pub name: String,
    pub arity: usize,
    pub num_locals: usize,
    pub start: usize,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub funcs: Vec<FuncMeta>,
    pub main_locals: usize,
}

impl Chunk {
    /// Serialize into the human-readable .abc text format.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("MAIN_LOCALS {}\n", self.main_locals));
        out.push_str(&format!("FUNCS {}\n", self.funcs.len()));
        for f in &self.funcs {
            out.push_str(&format!(
                "FUNC {} {} {} {}\n",
                f.name, f.arity, f.num_locals, f.start
            ));
        }
        out.push_str(&format!("CODE {}\n", self.code.len()));
        for (i, instr) in self.code.iter().enumerate() {
            out.push_str(&format!("{:>5}: {}\n", i, instr));
        }
        out
    }

    pub fn from_text(text: &str) -> Result<Chunk, String> {
        let mut lines = text.lines();
        let main_locals = parse_kv_line(lines.next().ok_or("missing MAIN_LOCALS")?, "MAIN_LOCALS")?;
        let func_count: usize = parse_kv_line(lines.next().ok_or("missing FUNCS")?, "FUNCS")?;

        let mut funcs = Vec::new();
        for _ in 0..func_count {
            let line = lines.next().ok_or("missing FUNC line")?;
            let rest = line.strip_prefix("FUNC ").ok_or("expected FUNC line")?;
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 4 {
                return Err(format!("malformed FUNC line: {}", line));
            }
            funcs.push(FuncMeta {
                name: parts[0].to_string(),
                arity: parts[1].parse().map_err(|_| "bad arity")?,
                num_locals: parts[2].parse().map_err(|_| "bad num_locals")?,
                start: parts[3].parse().map_err(|_| "bad start")?,
            });
        }

        let code_count: usize = parse_kv_line(lines.next().ok_or("missing CODE")?, "CODE")?;
        let mut code = Vec::new();
        for _ in 0..code_count {
            let line = lines.next().ok_or("missing code line")?;
            let body = match line.split_once(':') {
                Some((_, rest)) => rest.trim(),
                None => line.trim(),
            };
            code.push(parse_instr(body)?);
        }

        Ok(Chunk { code, funcs, main_locals })
    }
}

fn parse_kv_line(line: &str, key: &str) -> Result<usize, String> {
    let prefix = format!("{} ", key);
    let rest = line.strip_prefix(&prefix).ok_or(format!("expected '{}' line", key))?;
    rest.trim().parse().map_err(|_| format!("bad value for '{}'", key))
}

fn parse_instr(s: &str) -> Result<Instr, String> {
    let mut parts = s.splitn(2, ' ');
    let op = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();
    let instr = match op {
        "PUSH_INT" => Instr::PushInt(rest.parse().map_err(|_| "bad int operand")?),
        "PUSH_BOOL" => Instr::PushBool(rest == "true"),
        "PUSH_STR" => Instr::PushStr(decode_str(rest).ok_or("bad string operand")?),
        "POP" => Instr::Pop,
        "ADD" => Instr::Add,
        "SUB" => Instr::Sub,
        "MUL" => Instr::Mul,
        "DIV" => Instr::Div,
        "MOD" => Instr::Mod,
        "NEG" => Instr::Neg,
        "NOT" => Instr::Not,
        "EQ" => Instr::Eq,
        "NEQ" => Instr::NotEq,
        "LT" => Instr::Lt,
        "GT" => Instr::Gt,
        "LE" => Instr::LtEq,
        "GE" => Instr::GtEq,
        "AND" => Instr::And,
        "OR" => Instr::Or,
        "LOAD" => Instr::Load(rest.parse().map_err(|_| "bad operand")?),
        "STORE" => Instr::Store(rest.parse().map_err(|_| "bad operand")?),
        "JUMP" => Instr::Jump(rest.parse().map_err(|_| "bad operand")?),
        "JUMP_IF_FALSE" => Instr::JumpIfFalse(rest.parse().map_err(|_| "bad operand")?),
        "CALL" => {
            let mut it = rest.split_whitespace();
            let idx = it.next().ok_or("missing call idx")?.parse().map_err(|_| "bad call idx")?;
            let argc = it.next().ok_or("missing call argc")?.parse().map_err(|_| "bad call argc")?;
            Instr::Call(idx, argc)
        }
        "RET" => Instr::Ret,
        "PRINT" => Instr::Print,
        "HALT" => Instr::Halt,
        other => return Err(format!("unknown opcode '{}'", other)),
    };
    Ok(instr)
}
