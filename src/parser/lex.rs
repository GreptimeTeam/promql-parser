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
    static ref STRING_SYMBOL_SET: HashSet<char> = HashSet::from(['"', '`', '\'']);
}

pub type LexemeType = DefaultLexeme<TokenType>;

pub fn lexer(s: &str) -> Result<LRNonStreamingLexer<LexemeType, TokenType>, String> {
    let lexemes: Vec<Result<LexemeType, String>> = Lexer::new(s).into_iter().collect();
    match lexemes.last() {
        Some(Err(info)) => Err(info.into()),
        None => Err(format!("generated empty lexemes for {}", s)),
        _ => {
            let lexemes = lexemes.into_iter().filter_map(|l| l.ok()).map(Ok).collect();
            Ok(LRNonStreamingLexer::new(s, lexemes, Vec::new()))
        }
    }
}

// pub fn lexer<'a>(s: &'a str) -> Result<LRNonStreamingLexer<'a, 'a, LexemeType, TokenType>, String> {
//     let lexemes: Vec<Result<LexemeType, String>> = Lexer::new(s).into_iter().collect();
//     match lexemes.last() {
//         Some(Err(info)) => Err(info.into()),
//         None => Err(format!("generated empty lexemes for {}", s)),
//         _ => {
//             let lexemes = lexemes.into_iter().filter_map(|l| l.ok()).map(Ok).collect();
//             Ok(LRNonStreamingLexer::new(s, lexemes, Vec::new()))
//         }
//     }
// }

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
                None => State::Err("'!' can not be at the end".into()),
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
                None => State::Err("'.' can not be at the end".into()),
            },
            Some(ch) if is_alpha(ch) || ch == ':' => {
                self.backup();
                State::KeywordOrIdentifier
            }
            Some(ch) if STRING_SYMBOL_SET.contains(&ch) => State::String(ch),
            Some('(') => {
                self.inc_paren_depth();
                State::Lexeme(T_LEFT_PAREN)
            }
            Some(')') => {
                if self.is_paren_balanced() {
                    State::Err("unexpected right parenthesis ')'".into())
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
            None if !self.is_paren_balanced() => State::Err("unbalanced parenthesis".into()),
            None => State::End,
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
            if ch == '\r' || ch == '\n' {
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
        self.backup(); // backup to include the already spanned space
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

        !matches!(self.peek(), Some(ch) if is_alpha_numeric(ch))
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
            Some(ch) if STRING_SYMBOL_SET.contains(&ch) => State::String(ch),
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

    fn inside_brackets(&mut self) -> State {
        match self.pop() {
            Some(ch) if SPACE_SET.contains(&ch) => State::Space,
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

    // scans an alphanumeric identifier. The next character
    // is known to be a letter.
    fn accept_identifier(&mut self) -> State {
        self.accept_run(is_alpha_numeric);
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

fn is_alpha_numeric(ch: char) -> bool {
    is_alpha(ch) || is_digit(ch)
}

fn is_digit(ch: char) -> bool {
    ('0'..='9').contains(&ch)
}

fn is_alpha(ch: char) -> bool {
    ch == '_' || ('a'..='z').contains(&ch) || ('A'..='Z').contains(&ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref CASES: Vec<bool> = [
            ////////////////////////////////////// common
            (",", vec![(T_COMMA, 0, 1)]),
            ("()", vec![(T_LEFT_PAREN, 0, 1), (T_RIGHT_PAREN, 1, 1)]),
            ("{}", vec![(T_LEFT_BRACE, 0, 1), (T_RIGHT_BRACE, 1, 1)]),
            ("[5m]", vec![(T_LEFT_BRACKET, 0, 1), (T_DURATION, 1, 2), (T_RIGHT_BRACKET, 3, 1)]),
            ("[ 5m]", vec![(T_LEFT_BRACKET, 0, 1), (T_DURATION, 2, 2), (T_RIGHT_BRACKET, 4, 1)]),
            ("[  5m]", vec![(T_LEFT_BRACKET, 0, 1), (T_DURATION, 3, 2), (T_RIGHT_BRACKET, 5, 1)]),
            ("[  5m ]", vec![(T_LEFT_BRACKET, 0, 1), (T_DURATION, 3, 2), (T_RIGHT_BRACKET, 6, 1)]),
            ("\r\n\r", vec![]),

            ////////////////////////////////////// numbers
            ("1", vec![(T_NUMBER, 0, 1)]),
            ("4.23", vec![(T_NUMBER, 0, 4)]),
            (".3", vec![(T_NUMBER, 0, 2)]),
            ("5.", vec![(T_NUMBER, 0, 2)]),
            ("NaN", vec![(T_NUMBER, 0, 3)]),
            ("nAN", vec![(T_NUMBER, 0, 3)]),
            ("NaN 123", vec![(T_NUMBER, 0, 3), (T_NUMBER, 4, 3)]),
            ("NaN123", vec![(T_IDENTIFIER, 0, 6)]),
            ("iNf", vec![(T_NUMBER, 0, 3)]),
            ("Inf", vec![(T_NUMBER, 0, 3)]),
            ("+Inf", vec![(T_ADD, 0, 1), (T_NUMBER, 1, 3)]),
            ("+Inf 123", vec![(T_ADD, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)]),
            ("-Inf 123", vec![(T_SUB, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)]),
            ("Infoo", vec![(T_IDENTIFIER, 0, 5)]),
            ("-Inf123", vec![(T_SUB, 0, 1), (T_IDENTIFIER, 1, 6)]),
            ("-Inf 123", vec![(T_SUB, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)]),
            ("0x123", vec![(T_NUMBER, 0, 5)]),

            ////////////////////////////////////// strings
            ("\"test\\tsequence\"", vec![(T_STRING, 0, 16)]), // "test\tsequence"
            ("\"test\\\\.expression\"", vec![(T_STRING, 0, 19)]), // "test\\.expression"
            // FIXME: "\"test\\.expression\""
            ("`test\\.expression`", vec![(T_STRING, 0, 18)]), // `test\.expression`
            // FIXME: ".٩" https://github.com/prometheus/prometheus/issues/939

            ////////////////////////////////////// durations
            // NOTE: diff with Prometheus Go Version
            // duration is only valid in []
            ("[5s]", vec![(T_LEFT_BRACKET, 0, 1),(T_DURATION, 1, 2), (T_RIGHT_BRACKET, 3, 1)]),
            ("[123m]", vec![(T_LEFT_BRACKET, 0, 1),(T_DURATION, 1, 4), (T_RIGHT_BRACKET, 5, 1)]),
            ("[1h]", vec![(T_LEFT_BRACKET, 0, 1),(T_DURATION, 1, 2), (T_RIGHT_BRACKET, 3, 1)]),
            ("[3w]", vec![(T_LEFT_BRACKET, 0, 1),(T_DURATION, 1, 2), (T_RIGHT_BRACKET, 3, 1)]),
            ("[1y]", vec![(T_LEFT_BRACKET, 0, 1),(T_DURATION, 1, 2), (T_RIGHT_BRACKET, 3, 1)]),

            ////////////////////////////////////// identifiers
            ("abc", vec![(T_IDENTIFIER, 0, 3)]),
            ("a:bc", vec![(T_METRIC_IDENTIFIER, 0, 4)]),
            ("abc d", vec![(T_IDENTIFIER, 0, 3), (T_IDENTIFIER, 4, 1)]),
            (":bc", vec![(T_METRIC_IDENTIFIER, 0, 3)]),
            ("0a:bc", vec![]),

            ////////////////////////////////////// comments
            ("# some comment", vec![]),
            ("5 # 1+1\n5", vec![(T_NUMBER, 0, 1), (T_NUMBER, 8, 1)]),

            ////////////////////////////////////// operators
            ("=", vec![(T_EQL, 0, 1)]),
            ("{=}", vec![(T_LEFT_BRACE, 0, 1), (T_EQL, 1, 1), (T_RIGHT_BRACE, 2, 1)]),
            ("==", vec![(T_EQLC, 0, 2)]),
            ("!=", vec![(T_NEQ, 0, 2)]),
            ("<", vec![(T_LSS, 0, 1)]),
            (">", vec![(T_GTR, 0, 1)]),
            (">=", vec![(T_GTE, 0, 2)]),
            ("<=", vec![(T_LTE, 0, 2)]),
            ("+", vec![(T_ADD, 0, 1)]),
            ("-", vec![(T_SUB, 0, 1)]),
            ("*", vec![(T_MUL, 0, 1)]),
            ("/", vec![(T_DIV, 0, 1)]),
            ("^", vec![(T_POW, 0, 1)]),
            ("%", vec![(T_MOD, 0, 1)]),
            ("AND", vec![(T_LAND, 0, 3)]),
            ("or", vec![(T_LOR, 0, 2)]),
            ("unless", vec![(T_LUNLESS, 0, 6)]),
            ("@", vec![(T_AT, 0, 1)]),

            ////////////////////////////////////// aggregators
            ("sum", vec![(T_SUM, 0, 3)]),
            ("AVG", vec![(T_AVG, 0, 3)]),
            ("Max", vec![(T_MAX, 0, 3)]),
            ("min", vec![(T_MIN, 0, 3)]),
            ("count", vec![(T_COUNT, 0, 5)]),
            ("stdvar", vec![(T_STDVAR, 0, 6)]),
            ("stddev", vec![(T_STDDEV, 0, 6)]),

            ////////////////////////////////////// keywords
            ("offset", vec![(T_OFFSET, 0, 6)]),
            ("by", vec![(T_BY, 0, 2)]),
            ("without", vec![(T_WITHOUT, 0, 7)]),
            ("on", vec![(T_ON, 0, 2)]),
            ("ignoring", vec![(T_IGNORING, 0, 8)]),
            ("group_left", vec![(T_GROUP_LEFT, 0, 10)]),
            ("group_right", vec![(T_GROUP_RIGHT, 0, 11)]),
            ("bool", vec![(T_BOOL, 0, 4)]),
            ("atan2", vec![(T_ATAN2, 0, 5)]),

            ////////////////////////////////////// preprocessors
            ("start", vec![(T_START, 0, 5)]),
            ("end", vec![(T_END, 0, 3)]),

            ////////////////////////////////////// selectors
            ("伦敦", vec![]),
            ("伦敦='a'", vec![]),
            ("0a='a'", vec![]),
            ("{foo='bar'}",
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 3),
                  (T_EQL, 4, 1),
                  (T_STRING, 5, 5),
                  (T_RIGHT_BRACE, 10, 1)]),
            (r#"{foo="bar"}"#,
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 3),
                  (T_EQL, 4, 1),
                  (T_STRING, 5, 5),
                  (T_RIGHT_BRACE, 10, 1)]),
            (r#"{foo="bar\"bar"}"#,
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 3),
                  (T_EQL, 4, 1),
                  (T_STRING, 5, 10),
                  (T_RIGHT_BRACE, 15, 1)]),
            (r#"{NaN	!= "bar" }"#,
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 3),
                  (T_NEQ, 5, 2),
                  (T_STRING, 8, 5),
                  (T_RIGHT_BRACE, 14, 1)]),
            (r#"{alert=~"bar" }"#,
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 5),
                  (T_EQL_REGEX, 6, 2),
                  (T_STRING, 8, 5),
                  (T_RIGHT_BRACE, 14, 1)]),
            (r#"{on!~"bar"}"#,
             vec![(T_LEFT_BRACE, 0, 1),
                  (T_IDENTIFIER, 1, 2),
                  (T_NEQ_REGEX, 3, 2),
                  (T_STRING, 5, 5),
                  (T_RIGHT_BRACE, 10, 1)]),
            // NOTE: the 3rd is Err, it is ignored in test case
            (r#"{alert!#"bar"}"#, vec![(T_LEFT_BRACE, 0, 1), (T_IDENTIFIER, 1, 5)]),
            // NOTE: comma is illegal for identifier
            (r#"{foo:a="bar"}"#, vec![(T_LEFT_BRACE, 0, 1), (T_IDENTIFIER, 1, 3)]),
        ]
        .into_iter()
        .map(|(input, expected)| {
            let expected: Vec<LexemeType> = expected
                .into_iter()
                .map(|(token_id, start, len)| LexemeType::new(token_id, start, len))
                .collect();
            let actual: Vec<LexemeType> = Lexer::new(input)
                .into_iter()
                .filter_map(|l| {
                    match &l {
                        Ok(t) => println!("token: {:?}", t),
                        Err(i) => println!("err: {}", i),
                    }
                    l.ok()
                })
                .collect();

            let b = actual == expected;
            if !b {
                dbg!(&expected, &actual);
            }
            b
        })
        .collect();

        static ref LAST_ERR_CASES: Vec<bool> = [

            ////////////////////////////////////// common errors
            ("=~", "unexpected character after '=': ~"),
            ("!~", "unexpected character after '!': ~"),
            ("!(", "unexpected character after '!': ("),
            ("1a", "unexpected a after number 1"),

            ////////////////////////////////////// mismatched parentheses
            ("(", "unbalanced parenthesis"),
            (")", "unexpected right parenthesis ')'"),
            ("())", "unexpected right parenthesis ')'"),
            ("(()", "unbalanced parenthesis"),
            ("{", "unexpected end of input inside braces"),
            ("}", "unexpected right bracket '}'"),
            ("{{", "unexpected left brace '{' inside braces"),
            ("{{}}", "unexpected left brace '{' inside braces"),
            ("[", "unexpected end of input inside brackets"),
            ("[[", "unexpected left brace '[' inside brackets"),
            ("[]]", "unexpected right bracket ']'"),
            ("[[]]", "unexpected left brace '[' inside brackets"),
            ("]", "unexpected right bracket ']'"),
        ]
            .into_iter()
            .map(|(input, expected)| {
                let lexemes: Vec<Result<LexemeType, String>> = Lexer::new(input).into_iter().collect();
                let actual = match lexemes.last() {
                    Some(Err(info)) => info.to_string(),
                    _ => "".to_string(),
                };
                let b = !actual.is_empty() && actual.starts_with(expected);
                if !b {
                    dbg!(&expected, &actual);
                }
                b
            })
            .collect();

    }

    #[test]
    fn test_lexer() {
        assert!(CASES.iter().all(|&t| t));
        assert!(LAST_ERR_CASES.iter().all(|&t| t));
    }

    #[test]
    fn test_is_alpha() {
        assert!(is_alpha('_'));
        assert!(is_alpha('a'));
        assert!(is_alpha('z'));
        assert!(is_alpha('A'));
        assert!(is_alpha('Z'));
        assert!(!is_alpha('-'));
        assert!(!is_alpha('@'));
        assert!(!is_alpha('0'));
        assert!(!is_alpha('9'));
    }

    #[test]
    fn test_is_digit() {
        assert!(is_digit('0'));
        assert!(is_digit('9'));
        assert!(!is_digit('a'));
        assert!(!is_digit('z'));
        assert!(!is_digit('A'));
        assert!(!is_digit('Z'));
        assert!(!is_digit('x'));
        assert!(!is_digit('_'));
        assert!(!is_digit('-'));
        assert!(!is_digit('@'));
    }

    #[test]
    fn test_is_alpha_numeric() {
        assert!(is_alpha_numeric('_'));
        assert!(is_alpha_numeric('a'));
        assert!(is_alpha_numeric('z'));
        assert!(is_alpha_numeric('A'));
        assert!(is_alpha_numeric('Z'));
        assert!(is_alpha_numeric('0'));
        assert!(is_alpha_numeric('9'));
        assert!(!is_alpha_numeric('-'));
        assert!(!is_alpha_numeric('@'));
    }
}
