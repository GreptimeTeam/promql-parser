// Copyright 2022 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::parser::token::*;
use lazy_static::lazy_static;
use lrlex::{DefaultLexeme, LRNonStreamingLexer};
use lrpar::Lexeme;
use std::{collections::HashSet, fmt::Debug};

lazy_static! {
    static ref DEC_DIGITS_SET: HashSet<char> = "0123456789".chars().into_iter().collect();
    static ref HEX_DIGITS_SET: HashSet<char> =
        "0123456789abcdefABCDEF".chars().into_iter().collect();
    static ref ALL_DURATION_UNITS: HashSet<char> = HashSet::from(['s', 'm', 'h', 'd', 'w', 'y']);
    static ref ALL_DURATION_BUT_YEAR_UNITS: HashSet<char> =
        HashSet::from(['s', 'm', 'h', 'd', 'w']);
    static ref SPACE_SET: HashSet<char> = HashSet::from([' ', '\t', '\n', '\r']);
    static ref HEX_CHAR_SET: HashSet<char> = HashSet::from(['x', 'X']);
    static ref SCI_CHAR_SET: HashSet<char> = HashSet::from(['e', 'E']);
    static ref SIGN_CHAR_SET: HashSet<char> = HashSet::from(['+', '-']);
    static ref NORMAL_ESCAPE_SYMBOL_SET: HashSet<char> = "abfnrtv\\".chars().into_iter().collect();
}

pub type LexemeType = DefaultLexeme<TokenType>;

pub fn lexer<'a>(s: &'a str) -> Result<LRNonStreamingLexer<'a, 'a, LexemeType, TokenType>, String> {
    let lexemes: Vec<Result<LexemeType, String>> = Lexer::new(s).into_iter().collect();
    match lexemes.last() {
        Some(Err(info)) => Err(info.into()),
        None => Err(format!("generated empty lexemes for {}", s)),
        _ => {
            let lexemes = lexemes
                .into_iter()
                .filter_map(|l| l.ok())
                .map(|l| Ok(l))
                .collect();
            Ok(LRNonStreamingLexer::new(s, lexemes, Vec::new()))
        }
    }
}

#[derive(Debug)]
pub enum State {
    Start,
    End,
    Lexeme(TokenType),
    Identifier,
    KeywordOrIdentifier,
    Number,
    InsideBrackets,
    InsideBraces,
    LineComment,
    Space,
    String(char), // char is the symbol, ' or " or `
    Escape(char), // Escape happens inside String. char is the symbol, ' or " or `
    Err(String),
}

#[derive(Debug)]
pub struct Context {
    chars: Vec<char>,
    idx: usize,   // Current position in the Vec, increment by 1.
    start: usize, // Start position of one Token, increment by char.len_utf8.
    pos: usize,   // Current position in the input, increment by char.len_utf8.

    paren_depth: u8,    // Nesting depth of ( ) exprs, 0 means no parens.
    brace_open: bool,   // Whether a { is opened.
    bracket_open: bool, // Whether a [ is opened.
    got_colon: bool,    // Whether we got a ':' after [ was opened.
}

impl Context {
    pub fn new(input: &str) -> Context {
        Self {
            chars: input.chars().into_iter().collect(),
            idx: 0,
            start: 0,
            pos: 0,

            paren_depth: 0,
            brace_open: false,
            bracket_open: false,
            got_colon: false,
        }
    }

    /// pop the first char.
    pub fn pop(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += ch.len_utf8();
            self.idx += 1;
        };
        c
    }

    /// if the nothing , then this will do nothing.
    pub fn backup(&mut self) {
        if let Some(ch) = self.chars.get(self.idx - 1) {
            self.pos -= ch.len_utf8();
            self.idx -= 1;
        };
    }

    /// get the char at the pos to check, this won't consume it.
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    pub fn lexeme(&mut self, token_id: TokenType) -> LexemeType {
        DefaultLexeme::new(token_id, self.start, self.pos - self.start)
    }

    /// ignore the text between start and pos
    pub fn ignore(&mut self) {
        self.start = self.pos;
    }

    pub fn lexeme_string(&self) -> String {
        let mut s = String::from("");
        if self.idx == 0 {
            return s;
        }

        let mut pos = self.pos;
        let mut idx = self.idx;
        while pos > self.start {
            if let Some(&ch) = self.chars.get(idx - 1) {
                pos -= ch.len_utf8();
                idx -= 1;
                s.push(ch);
            };
        }
        s.chars().rev().collect()
    }
}

#[derive(Debug)]
pub struct Lexer {
    state: State,
    ctx: Context,
}

/// block for context operations.
impl Lexer {
    pub fn new(input: &str) -> Self {
        let ctx = Context::new(input);
        let state = State::Start;
        Self { state, ctx }
    }

    pub fn is_inside_braces(&self) -> bool {
        self.ctx.brace_open
    }

    pub fn jump_outof_braces(&mut self) {
        self.ctx.brace_open = false;
    }

    pub fn dive_into_braces(&mut self) {
        self.ctx.brace_open = true;
    }

    pub fn is_inside_brackets(&self) -> bool {
        self.ctx.bracket_open
    }

    pub fn jump_outof_brackets(&mut self) {
        self.ctx.bracket_open = false;
    }

    pub fn dive_into_brackets(&mut self) {
        self.ctx.bracket_open = true;
    }

    pub fn is_colon_scanned(&self) -> bool {
        self.ctx.got_colon
    }

    pub fn set_colon_scanned(&mut self) {
        self.ctx.got_colon = true;
    }

    pub fn reset_colon_scanned(&mut self) {
        self.ctx.got_colon = false;
    }

    pub fn inc_paren_depth(&mut self) {
        self.ctx.paren_depth += 1;
    }

    pub fn dec_paren_depth(&mut self) {
        self.ctx.paren_depth -= 1;
    }

    pub fn is_paren_balanced(&self) -> bool {
        self.ctx.paren_depth == 0
    }

    pub fn pop(&mut self) -> Option<char> {
        self.ctx.pop()
    }

    pub fn backup(&mut self) {
        self.ctx.backup();
    }

    pub fn peek(&self) -> Option<char> {
        self.ctx.peek()
    }

    /// lexeme() consumes the Span, which means consecutive lexeme() call
    /// will get wrong Span unless Lexer shifts its State.
    pub fn lexeme(&mut self, token_id: TokenType) -> LexemeType {
        let lexeme = self.ctx.lexeme(token_id);
        self.ctx.ignore();
        lexeme
    }

    pub fn lexeme_string(&self) -> String {
        self.ctx.lexeme_string()
    }

    pub fn ignore(&mut self) {
        self.ctx.ignore();
    }
}

/// block for state operations.
impl Lexer {
    pub fn shift(&mut self) {
        // NOTE: the design of the match arms's order is of no importance.
        // If different orders result in different states, then it has to be fixed.
        self.state = match self.state {
            State::Start => self.start(),
            State::End => panic!("End state can not shift forward."),
            State::Lexeme(_) => State::Start,
            State::String(ch) => self.accept_string(ch),
            State::KeywordOrIdentifier => self.accept_keyword_or_identifier(),
            State::Identifier => self.accept_identifier(),
            State::Number => self.accept_number(),
            State::InsideBrackets => self.inside_brackets(),
            State::InsideBraces => self.inside_braces(),
            State::LineComment => self.ignore_comment_line(),
            State::Escape(ch) => self.accept_escape(ch),
            State::Space => self.ignore_space(),
            State::Err(_) => State::End,
        };
    }

    fn start(&mut self) -> State {
        if self.is_inside_braces() {
            return State::InsideBraces;
        }

        if self.is_inside_brackets() {
            return State::InsideBrackets;
        }

        // NOTE: the design of the match arms's order is of no importance.
        // If different orders result in different states, then it has to be fixed.
        match self.pop() {
            Some('#') => State::LineComment,
            Some(',') => State::Lexeme(T_COMMA),
            Some(ch) if SPACE_SET.contains(&ch) => self.ignore_space(),
            Some('*') => State::Lexeme(T_MUL),
            Some('/') => State::Lexeme(T_DIV),
            Some('%') => State::Lexeme(T_MOD),
            Some('+') => State::Lexeme(T_ADD),
            Some('-') => State::Lexeme(T_SUB),
            Some('^') => State::Lexeme(T_POW),
            Some('=') => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_EQLC)
                }
                // =~ (label matcher) MUST be in brace
                Some('~') => State::Err("unexpected character after '=': ~".into()),
                _ => State::Lexeme(T_EQL),
            },
            Some('!') => match self.pop() {
                Some('=') => State::Lexeme(T_NEQ),
                Some(ch) => State::Err(format!("unexpected character after '!': {}", ch)),
                None => State::Err(format!("'!' can not be at the end")),
            },
            Some('<') => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_LTE)
                }
                _ => State::Lexeme(T_LSS),
            },
            Some('>') => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_GTE)
                }
                _ => State::Lexeme(T_GTR),
            },
            Some(ch) if is_digit(ch) => State::Number,
            Some('.') => match self.peek() {
                Some(ch) if is_digit(ch) => State::Number,
                Some(ch) => State::Err(format!("unexpected character after '.' {}", ch)),
                None => State::Err(format!("'.' can not be at the end")),
            },
            Some(ch) if is_alpha(ch) || ch == ':' => {
                self.backup();
                return State::KeywordOrIdentifier;
            }
            Some(ch) if is_string_symbol(ch) => State::String(ch),
            Some('(') => {
                self.inc_paren_depth();
                State::Lexeme(T_LEFT_PAREN)
            }
            Some(')') => {
                if self.is_paren_balanced() {
                    State::Err(format!("unexpected right parenthesis ')'"))
                } else {
                    self.dec_paren_depth();
                    State::Lexeme(T_RIGHT_PAREN)
                }
            }
            Some('{') => {
                self.dive_into_braces();
                State::Lexeme(T_LEFT_BRACE)
            }
            // the matched } has been consumed inside braces
            Some('}') => State::Err("unexpected right bracket '}'".into()),
            Some('[') => {
                self.reset_colon_scanned();
                self.dive_into_brackets();
                State::Lexeme(T_LEFT_BRACKET)
            }
            // the matched ] has been consumed inside brackets
            Some(']') => State::Err("unexpected right bracket ']'".into()),
            Some('@') => State::Lexeme(T_AT),
            Some(ch) => State::Err(format!("unexpected character: {}", ch)),
            None if !self.is_paren_balanced() => State::Err(format!("unclosed left parenthesis")),
            None => State::End,
        }
    }

    fn inside_brackets(&mut self) -> State {
        match self.pop() {
            Some(':') => {
                if self.is_colon_scanned() {
                    return State::Err("unexpected colon".into());
                }
                self.set_colon_scanned();
                State::Lexeme(T_COLON)
            }
            Some(ch) if is_digit(ch) => self.accept_duration(),
            Some(']') => {
                self.jump_outof_brackets();
                self.reset_colon_scanned();
                State::Lexeme(T_RIGHT_BRACKET)
            }
            Some('[') => State::Err("unexpected left brace '[' inside brackets".into()),
            Some(ch) => State::Err(format!("unexpected character inside brackets: {}", ch)),
            None => State::Err("unexpected end of input inside brackets".into()),
        }
    }

    /// the first number has been seen, so first backup.
    fn accept_duration(&mut self) -> State {
        self.backup();
        self.scan_number();
        if !self.accept_remaining_duration() {
            return State::Err(format!(
                "bad duration syntax around {}",
                self.lexeme_string()
            ));
        }
        State::Lexeme(T_DURATION)
    }

    /// the first number has been seen, so first backup.
    fn accept_number(&mut self) -> State {
        self.backup();
        self.scan_number();

        match self.peek() {
            Some(ch) if !SPACE_SET.contains(&ch) => State::Err(format!(
                "unexpected {} after number {}",
                ch,
                self.lexeme_string()
            )),
            _ => State::Lexeme(T_NUMBER),
        }
    }

    fn accept_keyword_or_identifier(&mut self) -> State {
        while let Some(ch) = self.pop() {
            if !is_alpha_numeric(ch) && ch != ':' {
                break;
            }
        }

        if self.peek().is_some() {
            self.backup();
        }

        let s = self.lexeme_string();
        match get_keyword_token(&s.to_lowercase()) {
            Some(token_id) => State::Lexeme(token_id),
            None if s.contains(':') => State::Lexeme(T_METRIC_IDENTIFIER),
            _ => State::Lexeme(T_IDENTIFIER),
        }
    }

    /// # has already not been consumed.
    fn ignore_comment_line(&mut self) -> State {
        while let Some(ch) = self.pop() {
            if is_end_of_line(ch) {
                break;
            }
        }
        self.ignore();
        State::Start
    }

    /// accept consumes the next char if f(ch) returns true.
    fn accept<F>(&mut self, f: F) -> bool
    where
        F: Fn(char) -> bool,
    {
        if let Some(ch) = self.peek() {
            if f(ch) {
                self.pop();
                return true;
            }
        }
        false
    }

    /// accept_run consumes a run of char from the valid set.
    fn accept_run<F>(&mut self, f: F)
    where
        F: Fn(char) -> bool,
    {
        while let Some(ch) = self.peek() {
            if f(ch) {
                self.pop();
            } else {
                break;
            }
        }
    }

    /// consumes a run of space, and ignore them.
    fn ignore_space(&mut self) -> State {
        // self.backup(); // backup to include the already spanned space
        self.accept_run(|ch| SPACE_SET.contains(&ch));
        self.ignore();
        State::Start
    }

    /// scan_number scans numbers of different formats. The scanned Item is
    /// not necessarily a valid number. This case is caught by the parser.
    fn scan_number(&mut self) {
        let mut digits: &HashSet<char> = &DEC_DIGITS_SET;

        if self.accept(|ch| ch == '0') && self.accept(|ch| HEX_CHAR_SET.contains(&ch)) {
            digits = &HEX_DIGITS_SET;
        }
        self.accept_run(|ch| digits.contains(&ch));
        if self.accept(|ch| ch == '.') {
            self.accept_run(|ch| digits.contains(&ch));
        }
        if self.accept(|ch| SCI_CHAR_SET.contains(&ch)) {
            self.accept(|ch| SIGN_CHAR_SET.contains(&ch));
            self.accept_run(|ch| DEC_DIGITS_SET.contains(&ch));
        }
    }

    /// number part has already been scanned.
    /// true only if the char after duration is not alphanumeric.
    fn accept_remaining_duration(&mut self) -> bool {
        // Next two char must be a valid duration.
        if !self.accept(|ch| ALL_DURATION_UNITS.contains(&ch)) {
            return false;
        }
        // Support for ms. Bad units like hs, ys will be caught when we actually
        // parse the duration.
        self.accept(|ch| ch == 's');

        // Next char can be another number then a unit.
        while self.accept(|ch| DEC_DIGITS_SET.contains(&ch)) {
            self.accept_run(|ch| DEC_DIGITS_SET.contains(&ch));
            // y is no longer in the list as it should always come first in durations.
            if !self.accept(|ch| ALL_DURATION_UNITS.contains(&ch)) {
                return false;
            }
            // Support for ms. Bad units like hs, ys will be caught when we actually
            // parse the duration.
            self.accept(|ch| ch == 's');
        }

        match self.peek() {
            Some(ch) if is_alpha_numeric(ch) => false,
            _ => true,
        }
    }

    /// scans a string escape sequence. The initial escaping character (\)
    /// has already been seen.
    // FIXME: more escape logic happens here, mostly to check if number is valid.
    // https://github.com/prometheus/prometheus/blob/0372e259baf014bbade3134fd79bcdfd8cbdef2c/promql/parser/lex.go#L552
    fn accept_escape(&mut self, symbol: char) -> State {
        match self.pop() {
            Some(ch) if ch == symbol || NORMAL_ESCAPE_SYMBOL_SET.contains(&ch) => {
                State::String(symbol)
            }
            Some(_) => State::String(symbol),
            None => State::Err("escape sequence not terminated".into()),
        }
    }

    /// scans a quoted string. The initial quote has already been seen.
    fn accept_string(&mut self, symbol: char) -> State {
        while let Some(ch) = self.pop() {
            if ch == '\\' {
                return State::Escape(symbol);
            }

            if ch == symbol {
                return State::Lexeme(T_STRING);
            }
        }

        State::Err(format!("unterminated quoted string {}", symbol))
    }

    /// scans the inside of a vector selector. Keywords are ignored and
    /// scanned as identifiers.
    fn inside_braces(&mut self) -> State {
        match self.pop() {
            Some('#') => State::LineComment,
            Some(',') => State::Lexeme(T_COMMA),
            Some(ch) if SPACE_SET.contains(&ch) => State::Space,
            Some(ch) if is_alpha(ch) => State::Identifier,
            Some(ch) if is_string_symbol(ch) => State::String(ch),
            Some('=') => match self.peek() {
                Some('~') => {
                    self.pop();
                    State::Lexeme(T_EQL_REGEX)
                }
                _ => State::Lexeme(T_EQL),
            },
            Some('!') => match self.pop() {
                Some('~') => State::Lexeme(T_NEQ_REGEX),
                Some('=') => State::Lexeme(T_NEQ),
                Some(ch) => State::Err(format!(
                    "unexpected character after '!' inside braces: {}",
                    ch
                )),
                None => State::Err("'!' can not be at the end".into()),
            },
            Some('{') => State::Err("unexpected left brace '{' inside braces".into()),
            Some('}') => {
                self.jump_outof_braces();
                State::Lexeme(T_RIGHT_BRACE)
            }
            Some(ch) => State::Err(format!("unexpected character inside braces: {}", ch)),
            None => State::Err("unexpected end of input inside braces".into()),
        }
    }

    // scans an alphanumeric identifier. The next character
    // is known to be a letter.
    fn accept_identifier(&mut self) -> State {
        self.accept_run(|ch| is_alpha_numeric(ch));
        State::Lexeme(T_IDENTIFIER)
    }
}

// TODO: reference iterator
impl Iterator for Lexer {
    type Item = Result<LexemeType, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.shift();
        match &self.state {
            State::Lexeme(token_id) => Some(Ok(self.lexeme(*token_id))),
            State::Err(info) => Some(Err(info.clone())),
            State::End => None,
            _ => self.next(),
        }
    }
}

fn is_string_symbol(ch: char) -> bool {
    ch == '"' || ch == '`' || ch == '\''
}

fn is_end_of_line(ch: char) -> bool {
    ch == '\r' || ch == '\n'
}

fn is_alpha_numeric(ch: char) -> bool {
    return is_alpha(ch) || is_digit(ch);
}

fn is_digit(ch: char) -> bool {
    return '0' <= ch && ch <= '9';
}

fn is_alpha(ch: char) -> bool {
    return ch == '_' || ('a' <= ch && ch <= 'z') || ('A' <= ch && ch <= 'Z');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let input = "node_cpu_seconds_total";
        // r#"hel:lo up = == != ,+ [1y] 2m 2d 3ms 2d3ms 123 1.1 0x1f .123 1.1e2 1.1 - != * / % == @ "hello" 'prometheus' `greptimedb` " " # comment at the end"#,
        let lexer = Lexer::new(input);

        // let lexer = Lexer::new("!a=");
        for lex in lexer {
            match lex {
                Ok(lexeme) => println!("{:?}, display:{}", lexeme, token_display(lexeme.tok_id())),
                Err(e) => println!("{e}"),
            }
        }
    }

    #[test]
    fn test_name() {
        let set: HashSet<char> = "abfnrtv\\".chars().into_iter().collect();
        println!("{:?}", set);
    }
}
