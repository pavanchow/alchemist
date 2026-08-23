use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i64),
    Str(String),
    Ident(String),
    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    Print,
    True,
    False,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Int(n) => write!(f, "{}", n),
            Tok::Str(s) => write!(f, "{:?}", s),
            Tok::Ident(s) => write!(f, "{}", s),
            Tok::Eof => write!(f, "<eof>"),
            other => write!(f, "{:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error at {}:{}: {}", self.line, self.col, self.message)
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut out = Vec::new();

    macro_rules! adv {
        () => {{
            if i < chars.len() && chars[i] == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
        }};
    }

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            adv!();
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                adv!();
            }
            continue;
        }

        let start_line = line;
        let start_col = col;

        if c.is_ascii_digit() {
            let mut num = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                num.push(chars[i]);
                adv!();
            }
            let val: i64 = num.parse().map_err(|_| LexError {
                message: format!("invalid integer literal '{}'", num),
                line: start_line,
                col: start_col,
            })?;
            out.push(Token { tok: Tok::Int(val), line: start_line, col: start_col });
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                adv!();
            }
            let tok = match ident.as_str() {
                "let" => Tok::Let,
                "fn" => Tok::Fn,
                "if" => Tok::If,
                "else" => Tok::Else,
                "while" => Tok::While,
                "return" => Tok::Return,
                "print" => Tok::Print,
                "true" => Tok::True,
                "false" => Tok::False,
                _ => Tok::Ident(ident),
            };
            out.push(Token { tok, line: start_line, col: start_col });
            continue;
        }

        if c == '"' {
            adv!();
            let mut s = String::new();
            loop {
                if i >= chars.len() {
                    return Err(LexError {
                        message: "unterminated string literal".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                }
                let ch = chars[i];
                if ch == '"' {
                    adv!();
                    break;
                }
                if ch == '\\' {
                    adv!();
                    if i >= chars.len() {
                        return Err(LexError {
                            message: "unterminated escape sequence".to_string(),
                            line,
                            col,
                        });
                    }
                    let esc = chars[i];
                    let real = match esc {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => {
                            return Err(LexError {
                                message: format!("unknown escape sequence '\\{}'", other),
                                line,
                                col,
                            })
                        }
                    };
                    s.push(real);
                    adv!();
                    continue;
                }
                s.push(ch);
                adv!();
            }
            out.push(Token { tok: Tok::Str(s), line: start_line, col: start_col });
            continue;
        }

        macro_rules! two {
            ($second:expr, $two_tok:expr, $one_tok:expr) => {{
                adv!();
                if i < chars.len() && chars[i] == $second {
                    adv!();
                    out.push(Token { tok: $two_tok, line: start_line, col: start_col });
                } else {
                    out.push(Token { tok: $one_tok, line: start_line, col: start_col });
                }
            }};
        }

        match c {
            '+' => {
                adv!();
                out.push(Token { tok: Tok::Plus, line: start_line, col: start_col });
            }
            '-' => {
                adv!();
                out.push(Token { tok: Tok::Minus, line: start_line, col: start_col });
            }
            '*' => {
                adv!();
                out.push(Token { tok: Tok::Star, line: start_line, col: start_col });
            }
            '/' => {
                adv!();
                out.push(Token { tok: Tok::Slash, line: start_line, col: start_col });
            }
            '%' => {
                adv!();
                out.push(Token { tok: Tok::Percent, line: start_line, col: start_col });
            }
            '=' => two!('=', Tok::Eq, Tok::Assign),
            '!' => two!('=', Tok::NotEq, Tok::Bang),
            '<' => two!('=', Tok::LtEq, Tok::Lt),
            '>' => two!('=', Tok::GtEq, Tok::Gt),
            '&' => {
                adv!();
                if i < chars.len() && chars[i] == '&' {
                    adv!();
                    out.push(Token { tok: Tok::AndAnd, line: start_line, col: start_col });
                } else {
                    return Err(LexError {
                        message: "unexpected character '&'".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                }
            }
            '|' => {
                adv!();
                if i < chars.len() && chars[i] == '|' {
                    adv!();
                    out.push(Token { tok: Tok::OrOr, line: start_line, col: start_col });
                } else {
                    return Err(LexError {
                        message: "unexpected character '|'".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                }
            }
            '(' => {
                adv!();
                out.push(Token { tok: Tok::LParen, line: start_line, col: start_col });
            }
            ')' => {
                adv!();
                out.push(Token { tok: Tok::RParen, line: start_line, col: start_col });
            }
            '{' => {
                adv!();
                out.push(Token { tok: Tok::LBrace, line: start_line, col: start_col });
            }
            '}' => {
                adv!();
                out.push(Token { tok: Tok::RBrace, line: start_line, col: start_col });
            }
            ',' => {
                adv!();
                out.push(Token { tok: Tok::Comma, line: start_line, col: start_col });
            }
            ';' => {
                adv!();
                out.push(Token { tok: Tok::Semi, line: start_line, col: start_col });
            }
            other => {
                return Err(LexError {
                    message: format!("unexpected character '{}'", other),
                    line: start_line,
                    col: start_col,
                })
            }
        }
    }

    out.push(Token { tok: Tok::Eof, line, col });
    Ok(out)
}
