use crate::ast::{BinOp, Expr, FnDef, Program, Stmt, UnOp};
use crate::lexer::{Tok, Token};
use std::fmt;

const MAX_EXPR_DEPTH: usize = 200;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
}

type PResult<T> = Result<T, ParseError>;

pub fn parse(toks: Vec<Token>) -> PResult<Program> {
    let mut p = Parser { toks, pos: 0, depth: 0 };
    let mut stmts = Vec::new();
    while !p.check(&Tok::Eof) {
        stmts.push(p.statement()?);
    }
    Ok(Program { stmts })
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn check(&self, t: &Tok) -> bool {
        &self.peek().tok == t
    }

    fn advance(&mut self) -> Token {
        let tok = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        let t = self.peek();
        ParseError { message: message.into(), line: t.line, col: t.col }
    }

    fn expect(&mut self, t: Tok) -> PResult<Token> {
        if self.check(&t) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected {:?}, found {}", t, self.peek().tok)))
        }
    }

    fn ident(&mut self) -> PResult<String> {
        match self.peek().tok.clone() {
            Tok::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.err(format!("expected identifier, found {}", other))),
        }
    }

    fn enter(&mut self) -> PResult<()> {
        self.depth += 1;
        if self.depth > MAX_EXPR_DEPTH {
            return Err(self.err("expression nesting too deep"));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn peek_ahead_is_assign(&self) -> bool {
        matches!(self.toks.get(self.pos + 1).map(|t| &t.tok), Some(Tok::Assign))
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&Tok::RBrace) {
            if self.check(&Tok::Eof) {
                return Err(self.err("unterminated block, expected '}'"));
            }
            stmts.push(self.statement()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(stmts)
    }

    fn statement(&mut self) -> PResult<Stmt> {
        match self.peek().tok.clone() {
            Tok::Let => {
                self.advance();
                let name = self.ident()?;
                self.expect(Tok::Assign)?;
                let expr = self.expression()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Let(name, expr))
            }
            Tok::Print => {
                self.advance();
                self.expect(Tok::LParen)?;
                let expr = self.expression()?;
                self.expect(Tok::RParen)?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Print(expr))
            }
            Tok::If => {
                self.advance();
                self.expect(Tok::LParen)?;
                let cond = self.expression()?;
                self.expect(Tok::RParen)?;
                let then_body = self.block()?;
                let else_body = if self.check(&Tok::Else) {
                    self.advance();
                    if self.check(&Tok::If) {
                        Some(vec![self.statement()?])
                    } else {
                        Some(self.block()?)
                    }
                } else {
                    None
                };
                Ok(Stmt::If(cond, then_body, else_body))
            }
            Tok::While => {
                self.advance();
                self.expect(Tok::LParen)?;
                let cond = self.expression()?;
                self.expect(Tok::RParen)?;
                let body = self.block()?;
                Ok(Stmt::While(cond, body))
            }
            Tok::Return => {
                self.advance();
                if self.check(&Tok::Semi) {
                    self.advance();
                    Ok(Stmt::Return(None))
                } else {
                    let expr = self.expression()?;
                    self.expect(Tok::Semi)?;
                    Ok(Stmt::Return(Some(expr)))
                }
            }
            Tok::Fn => {
                self.advance();
                let name = self.ident()?;
                self.expect(Tok::LParen)?;
                let mut params = Vec::new();
                if !self.check(&Tok::RParen) {
                    loop {
                        params.push(self.ident()?);
                        if self.check(&Tok::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Tok::RParen)?;
                let body = self.block()?;
                Ok(Stmt::FnDef(FnDef { name, params, body }))
            }
            Tok::Ident(name) if self.peek_ahead_is_assign() => {
                let line = self.peek().line;
                self.advance();
                self.expect(Tok::Assign)?;
                let expr = self.expression()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Assign(name, expr, line))
            }
            _ => {
                let expr = self.expression()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn expression(&mut self) -> PResult<Expr> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.and_expr()?;
        while self.check(&Tok::OrOr) {
            self.advance();
            let right = self.and_expr()?;
            left = Expr::Binary(BinOp::Or, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn and_expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.equality()?;
        while self.check(&Tok::AndAnd) {
            self.advance();
            let right = self.equality()?;
            left = Expr::Binary(BinOp::And, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn equality(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.comparison()?;
        loop {
            let op = match self.peek().tok {
                Tok::Eq => BinOp::Eq,
                Tok::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.comparison()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn comparison(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.term()?;
        loop {
            let op = match self.peek().tok {
                Tok::Lt => BinOp::Lt,
                Tok::Gt => BinOp::Gt,
                Tok::LtEq => BinOp::LtEq,
                Tok::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.term()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn term(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.factor()?;
        loop {
            let op = match self.peek().tok {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.factor()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn factor(&mut self) -> PResult<Expr> {
        self.enter()?;
        let mut left = self.unary()?;
        loop {
            let op = match self.peek().tok {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        self.leave();
        Ok(left)
    }

    fn unary(&mut self) -> PResult<Expr> {
        self.enter()?;
        let out = match self.peek().tok {
            Tok::Minus => {
                self.advance();
                let inner = self.unary()?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(inner)))
            }
            Tok::Bang => {
                self.advance();
                let inner = self.unary()?;
                Ok(Expr::Unary(UnOp::Not, Box::new(inner)))
            }
            _ => self.primary(),
        };
        self.leave();
        out
    }

    fn primary(&mut self) -> PResult<Expr> {
        self.enter()?;
        let out = match self.peek().tok.clone() {
            Tok::Int(n) => {
                self.advance();
                Ok(Expr::Int(n))
            }
            Tok::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Tok::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Tok::Str(s) => {
                self.advance();
                Ok(Expr::Str(s))
            }
            Tok::Ident(name) => {
                let line = self.peek().line;
                self.advance();
                if self.check(&Tok::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Tok::RParen) {
                        loop {
                            args.push(self.expression()?);
                            if self.check(&Tok::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    Ok(Expr::Call(name, args, line))
                } else {
                    Ok(Expr::Var(name, line))
                }
            }
            Tok::LParen => {
                self.advance();
                let e = self.expression()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            other => Err(self.err(format!("unexpected token {}", other))),
        };
        self.leave();
        out
    }
}
