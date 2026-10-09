use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::gui::apps::browser::dom::DomTree;

// ---------------------------------------------------------------------------
// 1. JS Values
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    DomElement(usize),
    Array(Vec<JsValue>),
    Object(BTreeMap<String, JsValue>),
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Builtin(&'static str),
}

impl JsValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Bool(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn to_string_val(&self) -> String {
        match self {
            JsValue::Undefined => String::from("undefined"),
            JsValue::Null => String::from("null"),
            JsValue::Bool(b) => String::from(if *b { "true" } else { "false" }),
            JsValue::Number(n) => {
                let as_i64 = *n as i64;
                if (as_i64 as f64) == *n {
                    format!("{}", as_i64)
                } else {
                    format!("{}", n)
                }
            }
            JsValue::String(s) => s.clone(),
            JsValue::DomElement(id) => format!("[object HTMLElement (node {})]", id),
            JsValue::Array(items) => {
                let s: Vec<String> = items.iter().map(|v| v.to_string_val()).collect();
                s.join(",")
            }
            JsValue::Object(_) => String::from("[object Object]"),
            JsValue::Function { .. } | JsValue::Builtin(_) => String::from("[function]"),
        }
    }

    pub fn to_number(&self) -> f64 {
        match self {
            JsValue::Undefined => f64::NAN,
            JsValue::Null => 0.0,
            JsValue::Bool(b) => if *b { 1.0 } else { 0.0 },
            JsValue::Number(n) => *n,
            JsValue::String(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
            _ => f64::NAN,
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Lexer / Tokenizer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Number(f64),
    String(String),
    Ident(String),
    // Keywords
    Var,
    Let,
    Const,
    Function,
    Return,
    If,
    Else,
    While,
    For,
    True,
    False,
    Null,
    Undefined,
    // Symbols
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    Eq,
    EqEq,
    EqEqEq,
    NotEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    PlusEq,
    MinusEq,
    Bang,
    AmpAmp,
    PipePipe,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
}

pub fn tokenize_js(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Single line comment //
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Multi-line comment /* */
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // String literals ("..." or '...')
        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            let mut s = String::new();
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                    match bytes[i] {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'\\' => s.push('\\'),
                        other => s.push(other as char),
                    }
                } else {
                    s.push(bytes[i] as char);
                }
                i += 1;
            }
            if i < len {
                i += 1; // Consume closing quote
            }
            tokens.push(Token::String(s));
            continue;
        }

        // Numbers
        if b.is_ascii_digit() {
            let start = i;
            let mut has_dot = false;
            while i < len && (bytes[i].is_ascii_digit() || (!has_dot && bytes[i] == b'.')) {
                if bytes[i] == b'.' {
                    has_dot = true;
                }
                i += 1;
            }
            if let Ok(n) = source[start..i].parse::<f64>() {
                tokens.push(Token::Number(n));
            }
            continue;
        }

        // Identifiers and Keywords
        if b.is_ascii_alphabetic() || b == b'_' || b == b'$' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                i += 1;
            }
            let word = &source[start..i];
            let tok = match word {
                "var" => Token::Var,
                "let" => Token::Let,
                "const" => Token::Const,
                "function" => Token::Function,
                "return" => Token::Return,
                "if" => Token::If,
                "else" => Token::Else,
                "while" => Token::While,
                "for" => Token::For,
                "true" => Token::True,
                "false" => Token::False,
                "null" => Token::Null,
                "undefined" => Token::Undefined,
                _ => Token::Ident(String::from(word)),
            };
            tokens.push(tok);
            continue;
        }

        // Multi-character operators
        if i + 2 < len {
            match &source[i..i + 3] {
                "===" => {
                    tokens.push(Token::EqEqEq);
                    i += 3;
                    continue;
                }
                "!==" => {
                    tokens.push(Token::NotEqEq);
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }

        if i + 1 < len {
            let two = &source[i..i + 2];
            let tok = match two {
                "==" => Some(Token::EqEq),
                "!=" => Some(Token::NotEq),
                "<=" => Some(Token::LtEq),
                ">=" => Some(Token::GtEq),
                "+=" => Some(Token::PlusEq),
                "-=" => Some(Token::MinusEq),
                "++" => Some(Token::PlusPlus),
                "--" => Some(Token::MinusMinus),
                "&&" => Some(Token::AmpAmp),
                "||" => Some(Token::PipePipe),
                _ => None,
            };
            if let Some(t) = tok {
                tokens.push(t);
                i += 2;
                continue;
            }
        }

        // Single character punctuation
        let single = match b {
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'%' => Token::Percent,
            b'=' => Token::Eq,
            b'<' => Token::Lt,
            b'>' => Token::Gt,
            b'!' => Token::Bang,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b';' => Token::Semicolon,
            b',' => Token::Comma,
            b'.' => Token::Dot,
            _ => {
                i += 1;
                continue;
            }
        };
        tokens.push(single);
        i += 1;
    }

    tokens
}

// ---------------------------------------------------------------------------
// 3. AST (Expressions & Statements)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateOp {
    Inc,
    Dec,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(JsValue),
    Identifier(String),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Update {
        op: UpdateOp,
        expr: Box<Expr>,
        prefix: bool,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Array(Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    VarDecl {
        name: String,
        init: Option<Expr>,
    },
    Expr(Expr),
    Block(Vec<Stmt>),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Box<Stmt>,
    },
    FunctionDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
}

// ---------------------------------------------------------------------------
// 4. Parser
// ---------------------------------------------------------------------------

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek() == Some(expected)
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_program(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while self.pos < self.tokens.len() {
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.advance(); // Skip unexpected token to recover
            }
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        match self.peek()? {
            Token::Var | Token::Let | Token::Const => {
                self.advance();
                let name = match self.advance()? {
                    Token::Ident(id) => id,
                    _ => return None,
                };
                let init = if self.match_token(&Token::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.match_token(&Token::Semicolon);
                Some(Stmt::VarDecl { name, init })
            }
            Token::Function => {
                self.advance();
                let name = match self.advance()? {
                    Token::Ident(id) => id,
                    _ => return None,
                };
                self.match_token(&Token::LParen);
                let mut params = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        if let Some(Token::Ident(p)) = self.advance() {
                            params.push(p);
                        }
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.match_token(&Token::RParen);
                let body = self.parse_block()?;
                Some(Stmt::FunctionDecl { name, params, body })
            }
            Token::Return => {
                self.advance();
                let expr = if !self.check(&Token::Semicolon) && !self.check(&Token::RBrace) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.match_token(&Token::Semicolon);
                Some(Stmt::Return(expr))
            }
            Token::If => {
                self.advance();
                self.match_token(&Token::LParen);
                let cond = self.parse_expr()?;
                self.match_token(&Token::RParen);
                let then_branch = Box::new(self.parse_stmt()?);
                let else_branch = if self.match_token(&Token::Else) {
                    Some(Box::new(self.parse_stmt()?))
                } else {
                    None
                };
                Some(Stmt::If {
                    cond,
                    then_branch,
                    else_branch,
                })
            }
            Token::While => {
                self.advance();
                self.match_token(&Token::LParen);
                let cond = self.parse_expr()?;
                self.match_token(&Token::RParen);
                let body = Box::new(self.parse_stmt()?);
                Some(Stmt::While { cond, body })
            }
            Token::For => {
                self.advance();
                self.match_token(&Token::LParen);
                let init = if self.match_token(&Token::Semicolon) {
                    None
                } else {
                    let s = self.parse_stmt()?;
                    Some(Box::new(s))
                };
                let cond = if self.check(&Token::Semicolon) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.match_token(&Token::Semicolon);
                let step = if self.check(&Token::RParen) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.match_token(&Token::RParen);
                let body = Box::new(self.parse_stmt()?);
                Some(Stmt::For {
                    init,
                    cond,
                    step,
                    body,
                })
            }
            Token::LBrace => {
                let stmts = self.parse_block()?;
                Some(Stmt::Block(stmts))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.match_token(&Token::Semicolon);
                Some(Stmt::Expr(expr))
            }
        }
    }

    fn parse_block(&mut self) -> Option<Vec<Stmt>> {
        self.match_token(&Token::LBrace);
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && self.pos < self.tokens.len() {
            if let Some(s) = self.parse_stmt() {
                stmts.push(s);
            } else {
                self.advance();
            }
        }
        self.match_token(&Token::RBrace);
        Some(stmts)
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Option<Expr> {
        let expr = self.parse_logical_or()?;

        if self.match_token(&Token::Eq) {
            let val = self.parse_assignment()?;
            return Some(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(val),
            });
        }
        if self.match_token(&Token::PlusEq) {
            let val = self.parse_assignment()?;
            return Some(Expr::Assign {
                target: Box::new(expr.clone()),
                value: Box::new(Expr::Binary {
                    op: BinOp::Add,
                    left: Box::new(expr),
                    right: Box::new(val),
                }),
            });
        }
        if self.match_token(&Token::MinusEq) {
            let val = self.parse_assignment()?;
            return Some(Expr::Assign {
                target: Box::new(expr.clone()),
                value: Box::new(Expr::Binary {
                    op: BinOp::Sub,
                    left: Box::new(expr),
                    right: Box::new(val),
                }),
            });
        }

        Some(expr)
    }

    fn parse_logical_or(&mut self) -> Option<Expr> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(&Token::PipePipe) {
            let right = self.parse_logical_and()?;
            expr = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_logical_and(&mut self) -> Option<Expr> {
        let mut expr = self.parse_equality()?;
        while self.match_token(&Token::AmpAmp) {
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                op: BinOp::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut expr = self.parse_relational()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::EqEq | Token::EqEqEq => BinOp::Eq,
                Token::NotEq | Token::NotEqEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_relational(&mut self) -> Option<Expr> {
        let mut expr = self.parse_additive()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Lt => BinOp::Lt,
                Token::LtEq => BinOp::LtEq,
                Token::Gt => BinOp::Gt,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        let mut expr = self.parse_multiplicative()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        let mut expr = self.parse_unary()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Some(expr)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.match_token(&Token::Bang) {
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.match_token(&Token::Minus) {
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
            });
        }
        if self.match_token(&Token::PlusPlus) {
            let expr = self.parse_postfix()?;
            return Some(Expr::Update {
                op: UpdateOp::Inc,
                expr: Box::new(expr),
                prefix: true,
            });
        }
        if self.match_token(&Token::MinusMinus) {
            let expr = self.parse_postfix()?;
            return Some(Expr::Update {
                op: UpdateOp::Dec,
                expr: Box::new(expr),
                prefix: true,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_call_member()?;
        if self.match_token(&Token::PlusPlus) {
            expr = Expr::Update {
                op: UpdateOp::Inc,
                expr: Box::new(expr),
                prefix: false,
            };
        } else if self.match_token(&Token::MinusMinus) {
            expr = Expr::Update {
                op: UpdateOp::Dec,
                expr: Box::new(expr),
                prefix: false,
            };
        }
        Some(expr)
    }

    fn parse_call_member(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&Token::Dot) {
                let prop = match self.advance()? {
                    Token::Ident(name) => name,
                    _ => return None,
                };
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: prop,
                };
            } else if self.match_token(&Token::LParen) {
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.match_token(&Token::RParen);
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }

        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.advance()? {
            Token::Number(n) => Some(Expr::Literal(JsValue::Number(n))),
            Token::String(s) => Some(Expr::Literal(JsValue::String(s))),
            Token::True => Some(Expr::Literal(JsValue::Bool(true))),
            Token::False => Some(Expr::Literal(JsValue::Bool(false))),
            Token::Null => Some(Expr::Literal(JsValue::Null)),
            Token::Undefined => Some(Expr::Literal(JsValue::Undefined)),
            Token::Ident(id) => Some(Expr::Identifier(id)),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.match_token(&Token::RParen);
                Some(expr)
            }
            Token::LBracket => {
                let mut items = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.match_token(&Token::RBracket);
                Some(Expr::Array(items))
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Interpreter Runtime Context
// ---------------------------------------------------------------------------

pub struct JsContext {
    scopes: Vec<BTreeMap<String, JsValue>>,
    pub alerts: Vec<String>,
    pub fuel: usize,
}

impl JsContext {
    pub fn new() -> Self {
        let mut global = BTreeMap::new();
        global.insert(String::from("undefined"), JsValue::Undefined);
        global.insert(String::from("null"), JsValue::Null);
        global.insert(String::from("alert"), JsValue::Builtin("alert"));
        global.insert(String::from("parseInt"), JsValue::Builtin("parseInt"));
        global.insert(String::from("parseFloat"), JsValue::Builtin("parseFloat"));
        global.insert(String::from("String"), JsValue::Builtin("String"));
        global.insert(String::from("Number"), JsValue::Builtin("Number"));

        // Global objects
        let mut doc = BTreeMap::new();
        doc.insert(String::from("getElementById"), JsValue::Builtin("document.getElementById"));
        doc.insert(String::from("title"), JsValue::String(String::from("Mouros Browser")));
        global.insert(String::from("document"), JsValue::Object(doc));

        let mut console = BTreeMap::new();
        console.insert(String::from("log"), JsValue::Builtin("console.log"));
        global.insert(String::from("console"), JsValue::Object(console));

        let mut math = BTreeMap::new();
        math.insert(String::from("floor"), JsValue::Builtin("Math.floor"));
        math.insert(String::from("ceil"), JsValue::Builtin("Math.ceil"));
        math.insert(String::from("abs"), JsValue::Builtin("Math.abs"));
        math.insert(String::from("round"), JsValue::Builtin("Math.round"));
        math.insert(String::from("min"), JsValue::Builtin("Math.min"));
        math.insert(String::from("max"), JsValue::Builtin("Math.max"));
        math.insert(String::from("random"), JsValue::Builtin("Math.random"));
        global.insert(String::from("Math"), JsValue::Object(math));

        JsContext {
            scopes: alloc::vec![global],
            alerts: Vec::new(),
            fuel: 50_000,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn get_var(&self, name: &str) -> JsValue {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
        }
        JsValue::Undefined
    }

    pub fn set_var(&mut self, name: &str, val: JsValue) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(String::from(name), val);
                return;
            }
        }
        // If not found in any scope, set in global scope (index 0)
        if let Some(global) = self.scopes.first_mut() {
            global.insert(String::from(name), val);
        }
    }

    pub fn declare_var(&mut self, name: &str, val: JsValue) {
        if let Some(current) = self.scopes.last_mut() {
            current.insert(String::from(name), val);
        }
    }

    pub fn eval_script(&mut self, tree: &mut DomTree, script: &str) -> JsValue {
        self.fuel = 50_000;
        let tokens = tokenize_js(script);
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();

        let mut last_val = JsValue::Undefined;
        for stmt in program {
            if self.fuel == 0 {
                break;
            }
            if let Some(ret) = self.exec_stmt(tree, &stmt) {
                return ret;
            }
            last_val = JsValue::Undefined;
        }
        last_val
    }

    fn check_fuel(&mut self) -> bool {
        if self.fuel > 0 {
            self.fuel -= 1;
            true
        } else {
            false
        }
    }

    fn exec_stmt(&mut self, tree: &mut DomTree, stmt: &Stmt) -> Option<JsValue> {
        if !self.check_fuel() {
            return None;
        }

        match stmt {
            Stmt::VarDecl { name, init } => {
                let val = if let Some(expr) = init {
                    self.eval_expr(tree, expr)
                } else {
                    JsValue::Undefined
                };
                self.declare_var(name, val);
                None
            }
            Stmt::Expr(expr) => {
                self.eval_expr(tree, expr);
                None
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                let mut ret = None;
                for s in stmts {
                    if let Some(val) = self.exec_stmt(tree, s) {
                        ret = Some(val);
                        break;
                    }
                }
                self.pop_scope();
                ret
            }
            Stmt::If { cond, then_branch, else_branch } => {
                let c = self.eval_expr(tree, cond);
                if c.is_truthy() {
                    self.exec_stmt(tree, then_branch)
                } else if let Some(el) = else_branch {
                    self.exec_stmt(tree, el)
                } else {
                    None
                }
            }
            Stmt::While { cond, body } => {
                while self.check_fuel() && self.eval_expr(tree, cond).is_truthy() {
                    if let Some(ret) = self.exec_stmt(tree, body) {
                        return Some(ret);
                    }
                }
                None
            }
            Stmt::For { init, cond, step, body } => {
                self.push_scope();
                if let Some(in_stmt) = init {
                    self.exec_stmt(tree, in_stmt);
                }
                while self.check_fuel() {
                    if let Some(c) = cond {
                        if !self.eval_expr(tree, c).is_truthy() {
                            break;
                        }
                    }
                    if let Some(ret) = self.exec_stmt(tree, body) {
                        self.pop_scope();
                        return Some(ret);
                    }
                    if let Some(st) = step {
                        self.eval_expr(tree, st);
                    }
                }
                self.pop_scope();
                None
            }
            Stmt::FunctionDecl { name, params, body } => {
                let func = JsValue::Function {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                };
                self.declare_var(name, func);
                None
            }
            Stmt::Return(expr) => {
                let val = if let Some(e) = expr {
                    self.eval_expr(tree, e)
                } else {
                    JsValue::Undefined
                };
                Some(val)
            }
        }
    }

    fn eval_expr(&mut self, tree: &mut DomTree, expr: &Expr) -> JsValue {
        if !self.check_fuel() {
            return JsValue::Undefined;
        }

        match expr {
            Expr::Literal(v) => v.clone(),
            Expr::Identifier(id) => self.get_var(id),
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(tree, expr);
                match op {
                    UnOp::Not => JsValue::Bool(!val.is_truthy()),
                    UnOp::Neg => JsValue::Number(-val.to_number()),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(tree, left);
                let r = self.eval_expr(tree, right);
                match op {
                    BinOp::Add => {
                        if let (JsValue::String(s1), _) = (&l, &r) {
                            JsValue::String(format!("{}{}", s1, r.to_string_val()))
                        } else if let (_, JsValue::String(s2)) = (&l, &r) {
                            JsValue::String(format!("{}{}", l.to_string_val(), s2))
                        } else {
                            JsValue::Number(l.to_number() + r.to_number())
                        }
                    }
                    BinOp::Sub => JsValue::Number(l.to_number() - r.to_number()),
                    BinOp::Mul => JsValue::Number(l.to_number() * r.to_number()),
                    BinOp::Div => {
                        let div = r.to_number();
                        if div == 0.0 {
                            JsValue::Number(f64::INFINITY)
                        } else {
                            JsValue::Number(l.to_number() / div)
                        }
                    }
                    BinOp::Mod => JsValue::Number(l.to_number() % r.to_number()),
                    BinOp::Eq => JsValue::Bool(l.to_string_val() == r.to_string_val()),
                    BinOp::NotEq => JsValue::Bool(l.to_string_val() != r.to_string_val()),
                    BinOp::Lt => JsValue::Bool(l.to_number() < r.to_number()),
                    BinOp::LtEq => JsValue::Bool(l.to_number() <= r.to_number()),
                    BinOp::Gt => JsValue::Bool(l.to_number() > r.to_number()),
                    BinOp::GtEq => JsValue::Bool(l.to_number() >= r.to_number()),
                    BinOp::And => {
                        if l.is_truthy() {
                            r
                        } else {
                            l
                        }
                    }
                    BinOp::Or => {
                        if l.is_truthy() {
                            l
                        } else {
                            r
                        }
                    }
                }
            }
            Expr::Update { op, expr, prefix } => {
                let current = self.eval_expr(tree, expr).to_number();
                let next = match op {
                    UpdateOp::Inc => current + 1.0,
                    UpdateOp::Dec => current - 1.0,
                };
                if let Expr::Identifier(ref id) = **expr {
                    self.set_var(id, JsValue::Number(next));
                }
                if *prefix {
                    JsValue::Number(next)
                } else {
                    JsValue::Number(current)
                }
            }
            Expr::Assign { target, value } => {
                let val = self.eval_expr(tree, value);
                match &**target {
                    Expr::Identifier(id) => {
                        self.set_var(id, val.clone());
                    }
                    Expr::Member { object, property } => {
                        let obj_val = self.eval_expr(tree, object);
                        match obj_val {
                            JsValue::DomElement(node_id) => {
                                // Element property assignment: el.innerText = val, el.className = val
                                if property == "innerText" || property == "textContent" {
                                    tree.set_inner_text(node_id, &val.to_string_val());
                                } else if property == "id" {
                                    tree.set_attribute(node_id, "id", &val.to_string_val());
                                } else if property == "className" {
                                    tree.set_attribute(node_id, "class", &val.to_string_val());
                                }
                            }
                            _ => {
                                // Nested style assignment: e.g. el.style.backgroundColor = "yellow"
                                if let Expr::Member {
                                    object: inner_obj,
                                    property: inner_prop,
                                } = &**object
                                {
                                    if inner_prop == "style" {
                                        let base = self.eval_expr(tree, inner_obj);
                                        if let JsValue::DomElement(node_id) = base {
                                            Self::update_element_style(tree, node_id, property, &val.to_string_val());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                val
            }
            Expr::Member { object, property } => {
                let obj = self.eval_expr(tree, object);
                match obj {
                    JsValue::DomElement(node_id) => {
                        match property.as_str() {
                            "innerText" | "textContent" => {
                                JsValue::String(tree.get_inner_text(node_id))
                            }
                            "id" => {
                                let id = tree.get_attribute(node_id, "id").unwrap_or("");
                                JsValue::String(String::from(id))
                            }
                            "className" => {
                                let c = tree.get_attribute(node_id, "class").unwrap_or("");
                                JsValue::String(String::from(c))
                            }
                            "style" => {
                                // Return proxy style object
                                let mut style_obj = BTreeMap::new();
                                if let Some(node) = tree.get_node(node_id) {
                                    style_obj.insert(
                                        String::from("cssText"),
                                        JsValue::String(node.inline_style.clone()),
                                    );
                                }
                                JsValue::Object(style_obj)
                            }
                            _ => JsValue::Undefined,
                        }
                    }
                    JsValue::Object(map) => {
                        map.get(property).cloned().unwrap_or(JsValue::Undefined)
                    }
                    JsValue::Array(arr) => {
                        if property == "length" {
                            JsValue::Number(arr.len() as f64)
                        } else if let Ok(idx) = property.parse::<usize>() {
                            arr.get(idx).cloned().unwrap_or(JsValue::Undefined)
                        } else {
                            JsValue::Undefined
                        }
                    }
                    JsValue::String(s) => {
                        if property == "length" {
                            JsValue::Number(s.len() as f64)
                        } else {
                            JsValue::Undefined
                        }
                    }
                    _ => JsValue::Undefined,
                }
            }
            Expr::Call { callee, args } => {
                let callee_val = self.eval_expr(tree, callee);
                let evaluated_args: Vec<JsValue> =
                    args.iter().map(|a| self.eval_expr(tree, a)).collect();

                match callee_val {
                    JsValue::Builtin(name) => self.call_builtin(tree, name, &evaluated_args),
                    JsValue::Function { params, body, .. } => {
                        self.push_scope();
                        for (idx, param) in params.iter().enumerate() {
                            let arg_val = evaluated_args
                                .get(idx)
                                .cloned()
                                .unwrap_or(JsValue::Undefined);
                            self.declare_var(param, arg_val);
                        }
                        let mut ret = JsValue::Undefined;
                        for stmt in &body {
                            if let Some(v) = self.exec_stmt(tree, stmt) {
                                ret = v;
                                break;
                            }
                        }
                        self.pop_scope();
                        ret
                    }
                    _ => JsValue::Undefined,
                }
            }
            Expr::Array(items) => {
                let evaluated = items.iter().map(|item| self.eval_expr(tree, item)).collect();
                JsValue::Array(evaluated)
            }
        }
    }

    fn call_builtin(&mut self, tree: &mut DomTree, name: &str, args: &[JsValue]) -> JsValue {
        match name {
            "alert" => {
                let msg = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                crate::serial_println!("[JS alert] {}", msg);
                self.alerts.push(msg);
                JsValue::Undefined
            }
            "console.log" => {
                let msgs: Vec<String> = args.iter().map(|v| v.to_string_val()).collect();
                crate::serial_println!("[JS console.log] {}", msgs.join(" "));
                JsValue::Undefined
            }
            "document.getElementById" => {
                if let Some(first) = args.first() {
                    let id_str = first.to_string_val();
                    if let Some(node_id) = tree.get_element_by_id(&id_str) {
                        return JsValue::DomElement(node_id);
                    }
                }
                JsValue::Null
            }
            "parseInt" => {
                if let Some(first) = args.first() {
                    let s = first.to_string_val();
                    let n = s.trim().parse::<i64>().unwrap_or(0);
                    JsValue::Number(n as f64)
                } else {
                    JsValue::Number(f64::NAN)
                }
            }
            "parseFloat" => {
                if let Some(first) = args.first() {
                    JsValue::Number(first.to_number())
                } else {
                    JsValue::Number(f64::NAN)
                }
            }
            "String" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                JsValue::String(s)
            }
            "Number" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                JsValue::Number(n)
            }
            "Math.floor" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                JsValue::Number(libm_floor(n))
            }
            "Math.ceil" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                JsValue::Number(libm_ceil(n))
            }
            "Math.round" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                JsValue::Number(libm_floor(n + 0.5))
            }
            "Math.abs" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                JsValue::Number(if n < 0.0 { -n } else { n })
            }
            "Math.min" => {
                let mut m = f64::INFINITY;
                for a in args {
                    let num = a.to_number();
                    if num < m {
                        m = num;
                    }
                }
                JsValue::Number(m)
            }
            "Math.max" => {
                let mut m = f64::NEG_INFINITY;
                for a in args {
                    let num = a.to_number();
                    if num > m {
                        m = num;
                    }
                }
                JsValue::Number(m)
            }
            "Math.random" => {
                // Simple pseudo-random using pit tick count
                let t = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed) as f64;
                let pseudo = ((t * 9301.0 + 49297.0) as u64 % 233280) as f64 / 233280.0;
                JsValue::Number(pseudo)
            }
            _ => JsValue::Undefined,
        }
    }

    fn update_element_style(tree: &mut DomTree, node_id: usize, prop_camel: &str, val: &str) {
        if let Some(node) = tree.get_node_mut(node_id) {
            let css_name = camel_to_kebab(prop_camel);
            // Append or update inline_style
            let mut parts: Vec<String> = node
                .inline_style
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter(|s| {
                    if let Some(colon) = s.find(':') {
                        let name = s[..colon].trim().to_lowercase();
                        name != css_name
                    } else {
                        true
                    }
                })
                .map(String::from)
                .collect();
            parts.push(format!("{}: {}", css_name, val));
            node.inline_style = parts.join("; ");
        }
    }
}

fn camel_to_kebab(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn libm_floor(x: f64) -> f64 {
    let i = x as i64;
    if (i as f64) > x {
        (i - 1) as f64
    } else {
        i as f64
    }
}

fn libm_ceil(x: f64) -> f64 {
    let i = x as i64;
    if (i as f64) < x {
        (i + 1) as f64
    } else {
        i as f64
    }
}

#[test_case]
fn test_js_interpreter_runtime() {
    let mut tree = DomTree::new();
    let div = tree.create_element("div");
    tree.append_child(tree.root, div);
    tree.set_attribute(div, "id", "card");
    tree.set_attribute(div, "style", "background-color: white;");

    let span = tree.create_element("span");
    tree.append_child(div, span);
    tree.set_attribute(span, "id", "counter");
    tree.set_inner_text(span, "0");

    let mut ctx = JsContext::new();
    let script = "
        var sum = 0;
        for (var i = 1; i <= 5; i++) {
            sum += i;
        }
        var el = document.getElementById('counter');
        el.innerText = 'Sum: ' + sum;

        var card = document.getElementById('card');
        card.style.backgroundColor = 'yellow';
        alert('Test Completed: ' + sum);
    ";
    ctx.eval_script(&mut tree, script);

    assert_eq!(tree.get_inner_text(span), "Sum: 15");
    assert!(tree.get_node(div).unwrap().inline_style.contains("background-color: yellow"));
    assert_eq!(ctx.alerts.len(), 1);
    assert_eq!(ctx.alerts[0], "Test Completed: 15");
}

