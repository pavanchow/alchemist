use crate::ast::{BinOp, Expr, FnDef, Program, Stmt, UnOp};
use crate::bytecode::{Chunk, FuncMeta, Instr};
use crate::errors::CompileError;
use std::collections::HashMap;

struct FnScope {
    blocks: Vec<HashMap<String, usize>>,
    next_slot: usize,
    in_function: bool,
}

impl FnScope {
    fn new(in_function: bool) -> Self {
        FnScope { blocks: vec![HashMap::new()], next_slot: 0, in_function }
    }

    fn push_block(&mut self) {
        self.blocks.push(HashMap::new());
    }

    fn pop_block(&mut self) {
        self.blocks.pop();
    }

    fn declare(&mut self, name: &str) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.blocks.last_mut().unwrap().insert(name.to_string(), slot);
        slot
    }

    fn resolve(&self, name: &str) -> Option<usize> {
        for block in self.blocks.iter().rev() {
            if let Some(slot) = block.get(name) {
                return Some(*slot);
            }
        }
        None
    }
}

struct FuncSig {
    index: usize,
    arity: usize,
}

pub struct Codegen {
    code: Vec<Instr>,
    funcs: Vec<FuncMeta>,
    sigs: HashMap<String, FuncSig>,
}

pub fn compile(program: &Program) -> Result<Chunk, CompileError> {
    let mut cg = Codegen { code: Vec::new(), funcs: Vec::new(), sigs: HashMap::new() };

    let mut fn_defs: Vec<&FnDef> = Vec::new();
    for stmt in &program.stmts {
        if let Stmt::FnDef(f) = stmt {
            if cg.sigs.contains_key(&f.name) {
                return Err(CompileError::DuplicateFunction { name: f.name.clone() });
            }
            cg.sigs.insert(f.name.clone(), FuncSig { index: fn_defs.len(), arity: f.params.len() });
            fn_defs.push(f);
        }
    }
    // reserve placeholder metas, filled with real start addresses once compiled
    cg.funcs = fn_defs
        .iter()
        .map(|f| FuncMeta { name: f.name.clone(), arity: f.params.len(), num_locals: 0, start: 0 })
        .collect();

    let mut main_scope = FnScope::new(false);
    for stmt in &program.stmts {
        if matches!(stmt, Stmt::FnDef(_)) {
            continue;
        }
        cg.stmt(stmt, &mut main_scope)?;
    }
    let main_locals = main_scope.next_slot;
    cg.code.push(Instr::Halt);

    for f in &fn_defs {
        let start = cg.code.len();
        let mut scope = FnScope::new(true);
        for p in &f.params {
            scope.declare(p);
        }
        for stmt in &f.body {
            cg.stmt(stmt, &mut scope)?;
        }
        cg.code.push(Instr::PushInt(0));
        cg.code.push(Instr::Ret);

        let idx = cg.sigs.get(&f.name).unwrap().index;
        cg.funcs[idx].start = start;
        cg.funcs[idx].num_locals = scope.next_slot;
    }

    Ok(Chunk { code: cg.code, funcs: cg.funcs, main_locals })
}

impl Codegen {
    fn stmt(&mut self, stmt: &Stmt, scope: &mut FnScope) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let(name, expr) => {
                self.expr(expr, scope)?;
                let slot = scope.declare(name);
                self.code.push(Instr::Store(slot));
            }
            Stmt::Assign(name, expr, line) => {
                let slot = scope
                    .resolve(name)
                    .ok_or_else(|| CompileError::UndefinedVariable { name: name.clone(), line: *line })?;
                self.expr(expr, scope)?;
                self.code.push(Instr::Store(slot));
            }
            Stmt::ExprStmt(expr) => {
                self.expr(expr, scope)?;
                self.code.push(Instr::Pop);
            }
            Stmt::Print(expr) => {
                self.expr(expr, scope)?;
                self.code.push(Instr::Print);
            }
            Stmt::If(cond, then_body, else_body) => {
                self.expr(cond, scope)?;
                let jf = self.emit_placeholder();
                scope.push_block();
                for s in then_body {
                    self.stmt(s, scope)?;
                }
                scope.pop_block();
                if let Some(else_stmts) = else_body {
                    let jend = self.emit_placeholder();
                    let else_start = self.code.len();
                    self.patch(jf, Instr::JumpIfFalse(else_start));
                    scope.push_block();
                    for s in else_stmts {
                        self.stmt(s, scope)?;
                    }
                    scope.pop_block();
                    let end = self.code.len();
                    self.patch(jend, Instr::Jump(end));
                } else {
                    let end = self.code.len();
                    self.patch(jf, Instr::JumpIfFalse(end));
                }
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len();
                self.expr(cond, scope)?;
                let jf = self.emit_placeholder();
                scope.push_block();
                for s in body {
                    self.stmt(s, scope)?;
                }
                scope.pop_block();
                self.code.push(Instr::Jump(loop_start));
                let end = self.code.len();
                self.patch(jf, Instr::JumpIfFalse(end));
            }
            Stmt::Return(expr) => {
                if !scope.in_function {
                    return Err(CompileError::ReturnOutsideFunction);
                }
                match expr {
                    Some(e) => self.expr(e, scope)?,
                    None => self.code.push(Instr::PushInt(0)),
                }
                self.code.push(Instr::Ret);
            }
            Stmt::FnDef(_) => {
                return Err(CompileError::DuplicateFunction {
                    name: "functions can only be defined at top level".to_string(),
                });
            }
        }
        Ok(())
    }

    fn expr(&mut self, expr: &Expr, scope: &FnScope) -> Result<(), CompileError> {
        match expr {
            Expr::Int(n) => self.code.push(Instr::PushInt(*n)),
            Expr::Bool(b) => self.code.push(Instr::PushBool(*b)),
            Expr::Str(s) => self.code.push(Instr::PushStr(s.clone())),
            Expr::Var(name, line) => match scope.resolve(name) {
                Some(slot) => self.code.push(Instr::Load(slot)),
                None => {
                    return Err(CompileError::UndefinedVariable { name: name.clone(), line: *line })
                }
            },
            Expr::Unary(op, inner) => {
                self.expr(inner, scope)?;
                match op {
                    UnOp::Neg => self.code.push(Instr::Neg),
                    UnOp::Not => self.code.push(Instr::Not),
                }
            }
            Expr::Binary(op, lhs, rhs) => {
                self.expr(lhs, scope)?;
                self.expr(rhs, scope)?;
                self.code.push(match op {
                    BinOp::Add => Instr::Add,
                    BinOp::Sub => Instr::Sub,
                    BinOp::Mul => Instr::Mul,
                    BinOp::Div => Instr::Div,
                    BinOp::Mod => Instr::Mod,
                    BinOp::Eq => Instr::Eq,
                    BinOp::NotEq => Instr::NotEq,
                    BinOp::Lt => Instr::Lt,
                    BinOp::Gt => Instr::Gt,
                    BinOp::LtEq => Instr::LtEq,
                    BinOp::GtEq => Instr::GtEq,
                    BinOp::And => Instr::And,
                    BinOp::Or => Instr::Or,
                });
            }
            Expr::Call(name, args, line) => {
                let sig = self.sigs.get(name).ok_or_else(|| CompileError::UndefinedFunction {
                    name: name.clone(),
                    line: *line,
                })?;
                if sig.arity != args.len() {
                    return Err(CompileError::ArityMismatch {
                        name: name.clone(),
                        expected: sig.arity,
                        got: args.len(),
                        line: *line,
                    });
                }
                let index = sig.index;
                for a in args {
                    self.expr(a, scope)?;
                }
                self.code.push(Instr::Call(index, args.len()));
            }
        }
        Ok(())
    }

    fn emit_placeholder(&mut self) -> usize {
        self.code.push(Instr::JumpIfFalse(usize::MAX));
        self.code.len() - 1
    }

    fn patch(&mut self, at: usize, instr: Instr) {
        self.code[at] = instr;
    }
}
