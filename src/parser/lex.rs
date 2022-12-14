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
    static ref dec_digits_set: HashSet<char> = "0123456789".chars().into_iter().collect();
    static ref hex_digits_set: HashSet<char> =
        "0123456789abcdefABCDEF".chars().into_iter().collect();
    static ref all_duration_units: HashSet<char> = HashSet::from(['s', 'm', 'h', 'd', 'w', 'y']);
    static ref all_duration_but_year_units: HashSet<char> =
        HashSet::from(['s', 'm', 'h', 'd', 'w']);
    static ref only_s_duration_units: HashSet<char> = HashSet::from(['s']);
    static ref space_set: HashSet<char> = HashSet::from([' ', '\t', '\n', '\r']);
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
pub struct Lexer {
    state: LexerState,
    ctx: Context,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let ctx = Context::new(input);
        let state = LexerState::Start;
        Self { state, ctx }
    }
}

impl Iterator for Lexer {
    type Item = Result<LexemeType, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.state = self.state.shift(&mut self.ctx);
        match &self.state {
            LexerState::Lexeme(token_id) => Some(Ok(self.ctx.lexeme(*token_id))),
            LexerState::Err(info) => Some(Err(info.clone())),
            LexerState::End => None,
            _ => self.next(),
        }
    }
}

#[derive(Debug)]
enum LexerState {
    Start,
    End,
    Lexeme(TokenType),
    /// consume all the remaining spaces, and ignore them
    Space,
    String,
    KeywordOrIdentifier,
    NumberOrDuration,
    Duration,
    InsideBraces,
    LineComment,
    Escape,
    Err(String),
}

impl LexerState {
    pub fn shift(&mut self, ctx: &mut Context) -> LexerState {
        match self {
            LexerState::Start => {
                if ctx.brace_open {
                    return LexerState::InsideBraces;
                }

                if ctx.bracket_open {
                    return LexerState::Duration;
                }

                if ctx.peek() == Some('#') {
                    ctx.pop();
                    return LexerState::LineComment;
                }

                match ctx.pop() {
                    Some(',') => LexerState::Lexeme(T_COMMA),
                    Some(ch) if is_space(ch) => LexerState::Space,
                    Some('*') => LexerState::Lexeme(T_MUL),
                    Some('/') => LexerState::Lexeme(T_DIV),
                    Some('%') => LexerState::Lexeme(T_MOD),
                    Some('+') => LexerState::Lexeme(T_ADD),
                    Some('-') => LexerState::Lexeme(T_SUB),
                    Some('^') => LexerState::Lexeme(T_POW),
                    Some('=') => match ctx.peek() {
                        Some('=') => {
                            ctx.pop();
                            LexerState::Lexeme(T_EQLC)
                        }
                        // =~ (label matcher) MUST be in brace, which will be handled in LexInsideBracesState
                        Some('~') => LexerState::Err("unexpected character after '=': ~".into()),
                        _ => LexerState::Lexeme(T_EQL),
                    },
                    Some('!') => match ctx.pop() {
                        Some('=') => LexerState::Lexeme(T_NEQ),
                        Some(ch) => {
                            LexerState::Err(format!("unexpected character after '!': {}", ch))
                        }
                        None => LexerState::Err(format!("'!' can not be at the end")),
                    },
                    Some('<') => match ctx.peek() {
                        Some('=') => {
                            ctx.pop();
                            LexerState::Lexeme(T_LTE)
                        }
                        _ => LexerState::Lexeme(T_LSS),
                    },
                    Some('>') => match ctx.peek() {
                        Some('=') => {
                            ctx.pop();
                            LexerState::Lexeme(T_GTE)
                        }
                        _ => LexerState::Lexeme(T_GTR),
                    },
                    Some(ch) if is_digit(ch) => {
                        ctx.backup();
                        LexerState::NumberOrDuration
                    }
                    Some('.') => match ctx.peek() {
                        Some(ch) if is_digit(ch) => {
                            ctx.backup();
                            LexerState::NumberOrDuration
                        }
                        Some(ch) => {
                            LexerState::Err(format!("unexpected character after '.' {}", ch))
                        }
                        None => LexerState::Err(format!("'.' can not be at the end")),
                    },
                    Some(ch) if is_string_open(ch) => {
                        ctx.string_open = true;
                        LexerState::String
                    }
                    Some(ch) if is_alpha(ch) || ch == ':' => {
                        if !ctx.bracket_open {
                            ctx.backup();
                            return LexerState::KeywordOrIdentifier;
                        }
                        if ctx.got_colon {
                            return LexerState::Err("unexpected colon ':'".into());
                        }

                        ctx.got_colon = true;
                        return LexerState::Lexeme(T_COLON);
                    }
                    Some('(') => {
                        ctx.paren_depth += 1;
                        LexerState::Lexeme(T_LEFT_PAREN)
                    }
                    Some(')') => {
                        if ctx.paren_depth == 0 {
                            LexerState::Err(format!("unexpected right parenthesis ')'"))
                        } else {
                            ctx.paren_depth -= 1;
                            LexerState::Lexeme(T_RIGHT_PAREN)
                        }
                    }
                    // NOTE: pay attention to the space after left brace, cover it in testcases.
                    Some('{') => {
                        ctx.brace_open = true;
                        LexerState::Lexeme(T_LEFT_BRACE)
                    }
                    Some('}') if !ctx.brace_open => {
                        LexerState::Err("unexpected right bracket '}'".into())
                    }
                    Some('}') => {
                        ctx.brace_open = false;
                        LexerState::Lexeme(T_RIGHT_BRACE)
                    }
                    // NOTE: pay attention to the space after left bracket, cover it in testcases.
                    Some('[') => {
                        ctx.got_colon = false;
                        ctx.bracket_open = true;
                        LexerState::Lexeme(T_LEFT_BRACKET)
                    }
                    Some(']') if !ctx.bracket_open => {
                        LexerState::Err("unexpected right bracket ']'".into())
                    }
                    Some(']') => {
                        ctx.bracket_open = false;
                        LexerState::Lexeme(T_RIGHT_BRACKET)
                    }
                    Some('@') => LexerState::Lexeme(T_AT),
                    Some(ch) => LexerState::Err(format!("unexpected character: {}", ch)),
                    None if ctx.paren_depth != 0 => {
                        LexerState::Err(format!("unclosed left parenthesis"))
                    }
                    None if ctx.brace_open => LexerState::Err(format!("unclosed left brace")),
                    None if ctx.bracket_open => LexerState::Err(format!("unclosed left bracket")),
                    None => LexerState::End,
                }
            }
            LexerState::End => LexerState::Err("End state can not shift forward.".into()),
            LexerState::Lexeme(_) => LexerState::Start,
            LexerState::Space => {
                accept_space(ctx);
                LexerState::Start
            }
            LexerState::String => todo!(),
            LexerState::KeywordOrIdentifier => keyword_or_identifier(ctx),
            LexerState::NumberOrDuration => {
                if scan_number(ctx) {
                    return LexerState::Lexeme(T_NUMBER);
                }
                if accept_remaining_duration(ctx) {
                    return LexerState::Lexeme(T_DURATION);
                }
                return LexerState::Err(format!(
                    "bad number or duration syntax: {}",
                    ctx.lexeme_string()
                ));
            }
            LexerState::Duration => todo!(),
            LexerState::InsideBraces => todo!(),
            LexerState::LineComment => {
                ignore_comment_line(ctx);
                LexerState::Start
            }
            LexerState::Escape => todo!(),
            LexerState::Err(_) => LexerState::End,
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
        let c = self.chars.get(self.idx).copied();
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

    /// get the first char.
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.idx + 1).copied()
    }

    /// caller MUST hold the token_id and only need the span from the context.
    pub fn lexeme(&mut self, token_id: TokenType) -> LexemeType {
        let lexeme = DefaultLexeme::new(token_id, self.start, self.pos - self.start);
        self.ignore();
        lexeme
    }

    /// ignore the text between start and pos
    pub fn ignore(&mut self) {
        self.start = self.pos;
    }

    pub fn lexeme_string(&self) -> String {
        let mut s = String::from("");
        let mut pos = self.pos;
        let mut idx = self.idx;
        while pos >= self.start {
            if let Some(&ch) = self.chars.get(idx - 1) {
                pos -= ch.len_utf8();
                idx -= 1;
                s.push(ch);
            };
        }
        s.chars().rev().collect()
    }
}

fn keyword_or_identifier(ctx: &mut Context) -> LexerState {
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
        Some(token_id) => LexerState::Lexeme(token_id),
        None if s.contains(':') => LexerState::Lexeme(T_METRIC_IDENTIFIER),
        _ => LexerState::Lexeme(T_IDENTIFIER),
    }
}

/// # has already not been consumed.
fn ignore_comment_line(ctx: &mut Context) {
    while let Some(ch) = ctx.pop() {
        if is_end_of_line(ch) {
            break;
        }
    }
    ctx.ignore();
}

/// accept consumes the next char if it's from the valid set.
fn accept(ctx: &mut Context, set: &HashSet<char>) -> bool {
    if let Some(ch) = ctx.peek() {
        if set.contains(&ch) {
            ctx.pop();
            return true;
        }
    }
    false
}

/// accept_run consumes a run of char from the valid set.
fn accept_run(ctx: &mut Context, set: &HashSet<char>) {
    while let Some(ch) = ctx.peek() {
        if set.contains(&ch) {
            ctx.pop();
        } else {
            break;
        }
    }
}

/// accept_space consumes a run of space, and ignore them
fn accept_space(ctx: &mut Context) {
    accept_run(ctx, &space_set);
    ctx.ignore();
}

/// scan_number scans numbers of different formats. The scanned Item is
/// not necessarily a valid number. This case is caught by the parser.
fn scan_number(ctx: &mut Context) -> bool {
    let mut digits: &HashSet<char> = &dec_digits_set;

    if accept(ctx, &HashSet::from(['0'])) && accept(ctx, &HashSet::from(['x', 'X'])) {
        digits = &hex_digits_set;
    }
    accept_run(ctx, digits);
    if accept(ctx, &HashSet::from(['.'])) {
        accept_run(ctx, digits);
    }
    if accept(ctx, &HashSet::from(['e', 'E'])) {
        accept(ctx, &HashSet::from(['+', '-']));
        accept_run(ctx, &dec_digits_set);
    }
    // Next thing must not be alphanumeric unless it's the times token
    match ctx.peek() {
        Some(ch) if is_alpha_numeric(ch) => false,
        _ => true,
    }
}

fn accept_remaining_duration(ctx: &mut Context) -> bool {
    // Next two char must be a valid duration.
    if !accept(ctx, &all_duration_units) {
        return false;
    }
    // Support for ms. Bad units like hs, ys will be caught when we actually
    // parse the duration.
    accept(ctx, &only_s_duration_units);

    // Next char can be another number then a unit.
    while accept(ctx, &dec_digits_set) {
        accept_run(ctx, &dec_digits_set);
        // y is no longer in the list as it should always come first in durations.
        if !accept(ctx, &all_duration_units) {
            return false;
        }
        // Support for ms. Bad units like hs, ys will be caught when we actually
        // parse the duration.
        accept(ctx, &only_s_duration_units);
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
    space_set.contains(&ch)
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
        let lexer = Lexer::new("arstrast 123.123 0x111 1y2m 5m = == != ,+  -*  /% ! == ");
        // let lexer = Lexer::new("!a=");
        for lex in lexer {
            match lex {
                Ok(lexeme) => println!("{:?}, display:{}", lexeme, token_display(lexeme.tok_id())),
                Err(e) => println!("{e}"),
            }
        }
    }
}
