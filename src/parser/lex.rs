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
    static ref ONLY_S_DURATION_UNITS: HashSet<char> = HashSet::from(['s']);
    static ref SPACE_SET: HashSet<char> = HashSet::from([' ', '\t', '\n', '\r']);
    static ref HEX_CHAR_SET: HashSet<char> = HashSet::from(['x', 'X']);
    static ref SCI_CHAR_SET: HashSet<char> = HashSet::from(['e', 'E']);
    static ref SIGN_CHAR_SET: HashSet<char> = HashSet::from(['+', '-']);
}

pub type LexemeType = DefaultLexeme<TokenType>;

// FIXME: this is just a demo Lexer, constructed for example
pub fn lexer<'a>(s: &'a str) -> LRNonStreamingLexer<'a, 'a, LexemeType, TokenType> {
    let mut start = 0;
    let mut len = "node_cpu_seconds_total".len();
    let metric_identifier_lexeme = DefaultLexeme::new(T_METRIC_IDENTIFIER, start, len);

    start += len;
    len = "{".len();
    let left_brace_lexeme = DefaultLexeme::new(T_LEFT_BRACE, start, len);

    start += len;
    len = "cpu".len();
    let identifier1_lexeme = DefaultLexeme::new(T_IDENTIFIER, start, len);

    start += len;
    len = "=".len();
    let eql1_lexeme = DefaultLexeme::new(T_EQL, start, len);

    start += len;
    len = "0".len();
    let val1_lexeme = DefaultLexeme::new(T_STRING, start, len);

    start += len;
    len = ",".len();
    let comma_lexeme = DefaultLexeme::new(T_COMMA, start, len);

    start += len;
    len = "mode".len();
    let identifier2_lexeme = DefaultLexeme::new(T_IDENTIFIER, start, len);

    start += len;
    len = "=".len();
    let eql2_lexeme = DefaultLexeme::new(T_EQL, start, len);

    start += len;
    len = "idel".len();
    let val2_lexeme = DefaultLexeme::new(T_STRING, start, len);

    start += len;
    len = "}".len();
    let right_brace_lexeme = DefaultLexeme::new(T_RIGHT_BRACE, start, len);

    let lexemes = vec![
        metric_identifier_lexeme,
        left_brace_lexeme,
        identifier1_lexeme,
        eql1_lexeme,
        val1_lexeme,
        comma_lexeme,
        identifier2_lexeme,
        eql2_lexeme,
        val2_lexeme,
        right_brace_lexeme,
    ]
    .into_iter()
    .map(Ok)
    .collect();

    LRNonStreamingLexer::new(s, lexemes, Vec::new())
}

#[derive(Debug)]
pub enum State {
    Start,
    End,
    Lexeme(TokenType),
    String,
    KeywordOrIdentifier,
    NumberOrDuration,
    Duration,
    InsideBraces,
    LineComment,
    Escape,
    Err(String),
}

impl State {
    pub fn shift(&mut self, ctx: &mut Context) -> State {
        match self {
            State::Start => start(ctx),
            State::End => panic!("End state can not shift forward."),
            State::Lexeme(_) => State::Start,
            State::String => todo!(),
            State::KeywordOrIdentifier => keyword_or_identifier(ctx),
            State::NumberOrDuration => number_or_duration(ctx),
            State::Duration => todo!(),
            State::InsideBraces => todo!(),
            State::LineComment => ignore_comment_line(ctx),
            State::Escape => todo!(),
            State::Err(_) => State::End,
        }
    }
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
    string_open: bool,
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
            string_open: false,
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

impl Lexer {
    pub fn new(input: &str) -> Self {
        let ctx = Context::new(input);
        let state = State::Start;
        Self { state, ctx }
    }

    /// lexeme() consumes the Span, which means lexeme() will get wrong
    /// Span in consecutive calls unless Lexer shifts its State.
    fn lexeme(&mut self, token_id: TokenType) -> LexemeType {
        let lexeme = self.ctx.lexeme(token_id);
        self.ctx.ignore();
        lexeme
    }

    fn shift(&mut self) {
        self.state = self.state.shift(&mut self.ctx);
    }
}

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

fn start(ctx: &mut Context) -> State {
    if ctx.brace_open {
        return State::InsideBraces;
    }

    if ctx.bracket_open {
        return State::Duration;
    }

    match ctx.pop() {
        Some('#') => State::LineComment,
        Some(',') => State::Lexeme(T_COMMA),
        Some(ch) if is_space(ch) => {
            ctx.backup();
            accept_space(ctx)
        }
        Some('*') => State::Lexeme(T_MUL),
        Some('/') => State::Lexeme(T_DIV),
        Some('%') => State::Lexeme(T_MOD),
        Some('+') => State::Lexeme(T_ADD),
        Some('-') => State::Lexeme(T_SUB),
        Some('^') => State::Lexeme(T_POW),
        Some('=') => match ctx.peek() {
            Some('=') => {
                ctx.pop();
                State::Lexeme(T_EQLC)
            }
            // =~ (label matcher) MUST be in brace, which will be handled in LexInsideBracesState
            Some('~') => State::Err("unexpected character after '=': ~".into()),
            _ => State::Lexeme(T_EQL),
        },
        Some('!') => match ctx.pop() {
            Some('=') => State::Lexeme(T_NEQ),
            Some(ch) => State::Err(format!("unexpected character after '!': {}", ch)),
            None => State::Err(format!("'!' can not be at the end")),
        },
        Some('<') => match ctx.peek() {
            Some('=') => {
                ctx.pop();
                State::Lexeme(T_LTE)
            }
            _ => State::Lexeme(T_LSS),
        },
        Some('>') => match ctx.peek() {
            Some('=') => {
                ctx.pop();
                State::Lexeme(T_GTE)
            }
            _ => State::Lexeme(T_GTR),
        },
        Some(ch) if is_digit(ch) => {
            ctx.backup();
            State::NumberOrDuration
        }
        Some('.') => match ctx.peek() {
            Some(ch) if is_digit(ch) => {
                ctx.backup();
                State::NumberOrDuration
            }
            Some(ch) => State::Err(format!("unexpected character after '.' {}", ch)),
            None => State::Err(format!("'.' can not be at the end")),
        },
        Some(ch) if is_alpha(ch) || ch == ':' => {
            if !ctx.bracket_open {
                ctx.backup();
                return State::KeywordOrIdentifier;
            }

            // the following logic is in []
            if ctx.got_colon {
                return State::Err("unexpected colon ':'".into());
            }
            // FIXME: how to get here?
            ctx.got_colon = true;
            return State::Lexeme(T_COLON);
        }
        Some(ch) if is_string_open(ch) => {
            ctx.string_open = true;
            State::String
        }
        Some('(') => {
            ctx.paren_depth += 1;
            State::Lexeme(T_LEFT_PAREN)
        }
        Some(')') => {
            if ctx.paren_depth == 0 {
                State::Err(format!("unexpected right parenthesis ')'"))
            } else {
                ctx.paren_depth -= 1;
                State::Lexeme(T_RIGHT_PAREN)
            }
        }
        // FIXME: pay attention to the space after left brace, cover it in testcases.
        Some('{') => {
            ctx.brace_open = true;
            State::Lexeme(T_LEFT_BRACE)
        }
        Some('}') if !ctx.brace_open => State::Err("unexpected right bracket '}'".into()),
        Some('}') => {
            ctx.brace_open = false;
            State::Lexeme(T_RIGHT_BRACE)
        }
        // FIXME: pay attention to the space after left bracket, cover it in testcases.
        Some('[') => {
            ctx.got_colon = false;
            ctx.bracket_open = true;
            State::Lexeme(T_LEFT_BRACKET)
        }
        Some(']') if !ctx.bracket_open => State::Err("unexpected right bracket ']'".into()),
        Some(']') => {
            ctx.bracket_open = false;
            State::Lexeme(T_RIGHT_BRACKET)
        }
        Some('@') => State::Lexeme(T_AT),
        Some(ch) => State::Err(format!("unexpected character: {}", ch)),
        None if ctx.paren_depth != 0 => State::Err(format!("unclosed left parenthesis")),
        None if ctx.brace_open => State::Err(format!("unclosed left brace")),
        None if ctx.bracket_open => State::Err(format!("unclosed left bracket")),
        None => State::End,
    }
}

fn number_or_duration(ctx: &mut Context) -> State {
    if scan_number(ctx) {
        return State::Lexeme(T_NUMBER);
    }
    if accept_remaining_duration(ctx) {
        return State::Lexeme(T_DURATION);
    }
    return State::Err(format!(
        "bad number or duration syntax: {}",
        ctx.lexeme_string()
    ));
}

fn keyword_or_identifier(ctx: &mut Context) -> State {
    while let Some(ch) = ctx.pop() {
        if !is_alpha_numeric(ch) && ch != ':' {
            break;
        }
    }

    if ctx.peek().is_some() {
        ctx.backup();
    }

    let s = ctx.lexeme_string();
    match get_keyword_token(&s.to_lowercase()) {
        Some(token_id) => State::Lexeme(token_id),
        None if s.contains(':') => State::Lexeme(T_METRIC_IDENTIFIER),
        _ => State::Lexeme(T_IDENTIFIER),
    }
}

/// # has already not been consumed.
fn ignore_comment_line(ctx: &mut Context) -> State {
    while let Some(ch) = ctx.pop() {
        if is_end_of_line(ch) {
            break;
        }
    }
    ctx.ignore();
    State::Start
}

/// accept consumes the next char if f(ch) returns true.
fn accept<F>(ctx: &mut Context, f: F) -> bool
where
    F: Fn(char) -> bool,
{
    if let Some(ch) = ctx.peek() {
        if f(ch) {
            ctx.pop();
            return true;
        }
    }
    false
}

/// accept_run consumes a run of char from the valid set.
fn accept_run<F>(ctx: &mut Context, f: F)
where
    F: Fn(char) -> bool,
{
    while let Some(ch) = ctx.peek() {
        if f(ch) {
            ctx.pop();
        } else {
            break;
        }
    }
}

/// accept_space consumes a run of space, and ignore them
fn accept_space(ctx: &mut Context) -> State {
    accept_run(ctx, |ch| SPACE_SET.contains(&ch));
    ctx.ignore();
    State::Start
}

/// scan_number scans numbers of different formats. The scanned Item is
/// not necessarily a valid number. This case is caught by the parser.
fn scan_number(ctx: &mut Context) -> bool {
    let mut digits: &HashSet<char> = &DEC_DIGITS_SET;

    if accept(ctx, |ch| ch == '0') && accept(ctx, |ch| HEX_CHAR_SET.contains(&ch)) {
        digits = &HEX_DIGITS_SET;
    }
    accept_run(ctx, |ch| digits.contains(&ch));
    if accept(ctx, |ch| ch == '.') {
        accept_run(ctx, |ch| digits.contains(&ch));
    }
    if accept(ctx, |ch| SCI_CHAR_SET.contains(&ch)) {
        accept(ctx, |ch| SIGN_CHAR_SET.contains(&ch));
        accept_run(ctx, |ch| DEC_DIGITS_SET.contains(&ch));
    }
    // Next thing must not be alphanumeric unless it's the times token.
    // If false, it maybe a duration lexeme.
    match ctx.peek() {
        Some(ch) if is_alpha_numeric(ch) => false,
        _ => true,
    }
}

fn accept_remaining_duration(ctx: &mut Context) -> bool {
    // Next two char must be a valid duration.
    if !accept(ctx, |ch| ALL_DURATION_UNITS.contains(&ch)) {
        return false;
    }
    // Support for ms. Bad units like hs, ys will be caught when we actually
    // parse the duration.
    accept(ctx, |ch| ONLY_S_DURATION_UNITS.contains(&ch));

    // Next char can be another number then a unit.
    while accept(ctx, |ch| DEC_DIGITS_SET.contains(&ch)) {
        accept_run(ctx, |ch| DEC_DIGITS_SET.contains(&ch));
        // y is no longer in the list as it should always come first in durations.
        if !accept(ctx, |ch| ALL_DURATION_UNITS.contains(&ch)) {
            return false;
        }
        // Support for ms. Bad units like hs, ys will be caught when we actually
        // parse the duration.
        accept(ctx, |ch| ONLY_S_DURATION_UNITS.contains(&ch));
    }

    match ctx.peek() {
        Some(ch) if is_alpha_numeric(ch) => false,
        _ => true,
    }
}

fn is_string_open(ch: char) -> bool {
    ch == '"' || ch == '`' || ch == '\''
}

fn is_space(ch: char) -> bool {
    SPACE_SET.contains(&ch)
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

fn is_label(chs: Vec<char>) -> bool {
    if chs.len() == 0 || !is_alpha(chs[0]) {
        return false;
    }
    chs.iter().all(|ch| is_alpha_numeric(*ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let lexer = Lexer::new(
            "hel:lo up = == != ,+ 1y 2m 2d 3ms 2d3ms 123 1.1 0x1f .123 1.1e2 1.1 - != * / % == @ # comment at the end",
        );

        // let lexer = Lexer::new("!a=");
        for lex in lexer {
            match lex {
                Ok(lexeme) => println!("{:?}, display:{}", lexeme, token_display(lexeme.tok_id())),
                Err(e) => println!("{e}"),
            }
        }
    }
}
