// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::parser::token::*;
use lrlex::{DefaultLexeme, LRNonStreamingLexer};
use lrpar::Lexeme;
use std::fmt::Debug;

const ESCAPE_SYMBOLS: &str = r#"abfnrtv\01234567xuU"#;
const STRING_SYMBOLS: &str = r#"'"`"#;

pub type LexemeType = DefaultLexeme<TokenId>;

pub fn lexer(s: &str) -> Result<LRNonStreamingLexer<LexemeType, TokenId>, String> {
    let lexemes: Vec<Result<LexemeType, String>> = Lexer::new(s).into_iter().collect();
    match lexemes.last() {
        Some(Err(info)) => Err(info.into()),
        Some(Ok(_)) => {
            // TODO: use better error mechanism, instead of filtering the err.
            let lexemes = lexemes.into_iter().filter_map(|l| l.ok()).map(Ok).collect();
            Ok(LRNonStreamingLexer::new(s, lexemes, Vec::new()))
        }
        None => Err(format!("no expression found in input: '{s}'")),
    }
}

#[derive(Debug)]
enum State {
    Start,
    End,
    Lexeme(TokenId),
    Identifier,
    KeywordOrIdentifier,
    NumberOrDuration,
    InsideBrackets,
    InsideBraces,
    LineComment,
    Space,
    String(char), // char is the symbol, ' or " or `
    Escape(char), // Escape happens inside String. char is the symbol, ' or " or `
    Err(String),
}

#[derive(Debug)]
struct Context {
    // TODO: use &str instead of Vec<char> for better performance.
    chars: Vec<char>,
    idx: usize,   // Current position in the Vec, increment by 1.
    start: usize, // Start position of one Token, increment by char.len_utf8.
    pos: usize,   // Current position in the input, increment by char.len_utf8.

    paren_depth: usize, // Nesting depth of ( ) exprs, 0 means no parens.
    brace_open: bool,   // Whether a { is opened.
    bracket_open: bool, // Whether a [ is opened.
    got_colon: bool,    // Whether we got a ':' after [ was opened.
}

impl Context {
    fn new(input: &str) -> Context {
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
    fn pop(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        self.idx += 1;
        Some(ch)
    }

    /// backup steps back one char. If cursor is at the beginning, it does nothing.
    /// caller should pay attention if the backup is successful or not.
    fn backup(&mut self) -> bool {
        if let Some(ch) = self.chars.get(self.idx - 1) {
            self.pos -= ch.len_utf8();
            self.idx -= 1;
            return true;
        };
        false
    }

    /// get the char at the pos to check, this won't consume it.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    /// string lexeme SHOULD trim the surrounding string symbols, ' or " or `
    fn lexeme(&mut self, token_id: TokenId) -> LexemeType {
        let mut start = self.start;
        let mut len = self.pos - self.start;
        if token_id == T_STRING {
            start += 1;
            len -= 2;
        }
        DefaultLexeme::new(token_id, start, len)
    }

    /// ignore the text between start and pos
    fn ignore(&mut self) {
        self.start = self.pos;
    }

    // TODO: refactor needed, details in Issues/15.
    fn lexeme_string(&self) -> String {
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
struct Lexer {
    state: State,
    ctx: Context,
}

/// block for context operations.
impl Lexer {
    fn new(input: &str) -> Self {
        let ctx = Context::new(input);
        let state = State::Start;
        Self { state, ctx }
    }

    fn is_inside_braces(&self) -> bool {
        self.ctx.brace_open
    }

    fn jump_outof_braces(&mut self) {
        self.ctx.brace_open = false;
    }

    fn dive_into_braces(&mut self) {
        self.ctx.brace_open = true;
    }

    fn is_inside_brackets(&self) -> bool {
        self.ctx.bracket_open
    }

    fn jump_outof_brackets(&mut self) {
        self.ctx.bracket_open = false;
    }

    fn dive_into_brackets(&mut self) {
        self.ctx.bracket_open = true;
    }

    fn is_colon_scanned(&self) -> bool {
        self.ctx.got_colon
    }

    fn set_colon_scanned(&mut self) {
        self.ctx.got_colon = true;
    }

    fn reset_colon_scanned(&mut self) {
        self.ctx.got_colon = false;
    }

    /// true only if paren depth less than MAX
    fn inc_paren_depth(&mut self) -> bool {
        if self.ctx.paren_depth < usize::MAX {
            self.ctx.paren_depth += 1;
            return true;
        }
        false
    }

    /// true only if paren depth larger than 1
    fn dec_paren_depth(&mut self) -> bool {
        if self.ctx.paren_depth >= 1 {
            self.ctx.paren_depth -= 1;
            return true;
        }
        false
    }

    fn is_paren_balanced(&self) -> bool {
        self.ctx.paren_depth == 0
    }

    fn pop(&mut self) -> Option<char> {
        self.ctx.pop()
    }

    fn backup(&mut self) -> bool {
        self.ctx.backup()
    }

    fn peek(&self) -> Option<char> {
        self.ctx.peek()
    }

    /// lexeme() consumes the Span, which means consecutive lexeme() call
    /// will get wrong Span unless Lexer shifts its State.
    fn lexeme(&mut self, token_id: TokenId) -> LexemeType {
        let lexeme = self.ctx.lexeme(token_id);
        self.ctx.ignore();
        lexeme
    }

    fn lexeme_string(&self) -> String {
        self.ctx.lexeme_string()
    }

    fn ignore(&mut self) {
        self.ctx.ignore();
    }
}

/// block for state operations.
impl Lexer {
    fn shift(&mut self) {
        // NOTE: the design of the match arms's order is of no importance.
        // If different orders result in different states, then it has to be fixed.
        self.state = match self.state {
            State::Start => self.start(),
            State::End => State::Err("End state can not shift forward.".into()),
            State::Lexeme(_) => State::Start,
            State::String(ch) => self.accept_string(ch),
            State::KeywordOrIdentifier => self.accept_keyword_or_identifier(),
            State::Identifier => self.accept_identifier(),
            State::NumberOrDuration => self.accept_number_or_duration(),
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

        let c = match self.pop() {
            None => {
                if !self.is_paren_balanced() {
                    return State::Err("unclosed left parenthesis".into());
                }
                return State::End;
            }
            Some(ch) => ch,
        };

        // NOTE: the design of the match arms's order is of no importance.
        // If different orders result in different states, then it has to be fixed.
        match c {
            '#' => State::LineComment,
            '@' => State::Lexeme(T_AT),
            ',' => State::Lexeme(T_COMMA),
            '*' => State::Lexeme(T_MUL),
            '/' => State::Lexeme(T_DIV),
            '%' => State::Lexeme(T_MOD),
            '+' => State::Lexeme(T_ADD),
            '-' => State::Lexeme(T_SUB),
            '^' => State::Lexeme(T_POW),
            '=' => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_EQLC)
                }
                // =~ (label matcher) MUST be in brace
                Some('~') => State::Err("unexpected character after '=': '~'".into()),
                _ => State::Lexeme(T_EQL),
            },
            '!' => match self.pop() {
                Some('=') => State::Lexeme(T_NEQ),
                Some(ch) => State::Err(format!("unexpected character after '!': '{ch}'")),
                None => State::Err("'!' can not be at the end".into()),
            },
            '<' => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_LTE)
                }
                _ => State::Lexeme(T_LSS),
            },
            '>' => match self.peek() {
                Some('=') => {
                    self.pop();
                    State::Lexeme(T_GTE)
                }
                _ => State::Lexeme(T_GTR),
            },
            ch if ch.is_ascii_whitespace() => self.ignore_space(),
            ch if ch.is_ascii_digit() => State::NumberOrDuration,
            '.' => match self.peek() {
                Some(ch) if ch.is_ascii_digit() => State::NumberOrDuration,
                Some(ch) => State::Err(format!("unexpected character after '.': '{ch}'")),
                None => State::Err("unexpected character: '.'".into()),
            },
            ch if is_alpha(ch) || ch == ':' => State::KeywordOrIdentifier,
            ch if STRING_SYMBOLS.contains(ch) => State::String(ch),
            '(' => {
                if self.inc_paren_depth() {
                    return State::Lexeme(T_LEFT_PAREN);
                }
                State::Err("too many left parentheses".into())
            }
            ')' => {
                if self.is_paren_balanced() {
                    return State::Err("unexpected right parenthesis ')'".into());
                }
                if self.dec_paren_depth() {
                    return State::Lexeme(T_RIGHT_PAREN);
                }
                State::Err("unexpected right parenthesis ')'".into())
            }
            '{' => {
                self.dive_into_braces();
                State::Lexeme(T_LEFT_BRACE)
            }
            // the matched } has been consumed inside braces
            '}' => State::Err("unexpected right brace '}'".into()),
            '[' => {
                self.reset_colon_scanned();
                self.dive_into_brackets();
                State::Lexeme(T_LEFT_BRACKET)
            }
            // the matched ] has been consumed inside brackets
            ']' => State::Err("unexpected right bracket ']'".into()),
            ch => State::Err(format!("unexpected character: {ch:?}")),
        }
    }

    /// the first number has been consumed, so first backup.
    fn accept_duration(&mut self) -> State {
        self.backup();
        self.scan_number();
        if !self.accept_remaining_duration() {
            self.pop(); // this is to include the bad syntax
            return State::Err(format!("bad duration syntax: {}", self.lexeme_string()));
        }
        State::Lexeme(T_DURATION)
    }

    /// the first number has been consumed, so first backup.
    fn accept_number_or_duration(&mut self) -> State {
        self.backup();
        if self.scan_number() {
            return State::Lexeme(T_NUMBER);
        }

        // Next two chars must be a valid unit and a non-alphanumeric.
        if self.accept_remaining_duration() {
            return State::Lexeme(T_DURATION);
        }

        // the next char is invalid, so it should be captured in the err info.
        self.pop();
        State::Err(format!(
            "bad number or duration syntax: {}",
            self.lexeme_string()
        ))
    }

    /// the first alphabetic character has been consumed, and no need to backup.
    fn accept_keyword_or_identifier(&mut self) -> State {
        while let Some(ch) = self.peek() {
            if is_alpha_numeric(ch) || ch == ':' {
                self.pop();
            } else {
                break;
            }
        }

        let s = self.lexeme_string();
        match get_keyword_token(&s.to_lowercase()) {
            Some(token_id) => State::Lexeme(token_id),
            None if s.contains(':') => State::Lexeme(T_METRIC_IDENTIFIER),
            _ => State::Lexeme(T_IDENTIFIER),
        }
    }

    /// # has already been consumed.
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
        self.accept_run(|ch| ch.is_ascii_whitespace());
        self.ignore();
        State::Start
    }

    /// scan_number scans numbers of different formats. The scanned Item is
    /// not necessarily a valid number. This case is caught by the parser.
    fn scan_number(&mut self) -> bool {
        let mut hex_digit = false;
        if self.accept(|ch| ch == '0') && self.accept(|ch| ch == 'x' || ch == 'X') {
            hex_digit = true;
        }
        let is_valid_digit = |ch: char| -> bool {
            if hex_digit {
                ch.is_ascii_hexdigit()
            } else {
                ch.is_ascii_digit()
            }
        };

        self.accept_run(is_valid_digit);
        if self.accept(|ch| ch == '.') {
            self.accept_run(is_valid_digit);
        }
        if self.accept(|ch| ch == 'e' || ch == 'E') {
            self.accept(|ch| ch == '+' || ch == '-');
            self.accept_run(|ch| ch.is_ascii_digit());
        }

        // Next thing must not be alpha or '.'
        // if alpha: it maybe a duration
        // if '.': invalid number
        !matches!(self.peek(), Some(ch) if is_alpha(ch) || ch == '.')
    }

    /// number part has already been scanned.
    /// true only if the char after duration is not alphanumeric.
    fn accept_remaining_duration(&mut self) -> bool {
        // Next two char must be a valid duration.
        if !self.accept(|ch| "smhdwy".contains(ch)) {
            return false;
        }
        // Support for ms. Bad units like hs, ys will be caught when we actually
        // parse the duration.
        self.accept(|ch| ch == 's');

        // Next char can be another number then a unit.
        while self.accept(|ch| ch.is_ascii_digit()) {
            self.accept_run(|ch| ch.is_ascii_digit());
            // y is no longer in the list as it should always come first in durations.
            if !self.accept(|ch| "smhdw".contains(ch)) {
                return false;
            }
            // Support for ms. Bad units like hs, ys will be caught when we actually
            // parse the duration.
            self.accept(|ch| ch == 's');
        }

        !matches!(self.peek(), Some(ch) if is_alpha_numeric(ch))
    }

    /// scans a string escape sequence. The initial escaping character (\)
    /// has already been consumed.
    // TODO: checking the validity of code point is NOT supported yet.
    fn accept_escape(&mut self, symbol: char) -> State {
        match self.pop() {
            Some(ch) if ch == symbol || ESCAPE_SYMBOLS.contains(ch) => State::String(symbol),
            Some(ch) => State::Err(format!("unknown escape sequence '{ch}'")),
            None => State::Err("escape sequence not terminated".into()),
        }
    }

    /// scans a quoted string. The initial quote has already been consumed.
    fn accept_string(&mut self, symbol: char) -> State {
        while let Some(ch) = self.pop() {
            if ch == '\\' {
                return State::Escape(symbol);
            }

            if ch == symbol {
                return State::Lexeme(T_STRING);
            }
        }

        State::Err(format!("unterminated quoted string {symbol}"))
    }

    /// scans the inside of a vector selector. Keywords are ignored and
    /// scanned as identifiers.
    fn inside_braces(&mut self) -> State {
        match self.pop() {
            Some('#') => State::LineComment,
            Some(',') => State::Lexeme(T_COMMA),
            Some(ch) if ch.is_ascii_whitespace() => State::Space,
            Some(ch) if is_alpha(ch) => State::Identifier,
            Some(ch) if STRING_SYMBOLS.contains(ch) => State::String(ch),
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
                    "unexpected character after '!' inside braces: '{ch}'"
                )),
                None => State::Err("'!' can not be at the end".into()),
            },
            Some('{') => State::Err("unexpected left brace '{' inside braces".into()),
            Some('}') => {
                self.jump_outof_braces();
                State::Lexeme(T_RIGHT_BRACE)
            }
            Some(ch) => State::Err(format!("unexpected character inside braces: '{ch}'")),
            None => State::Err("unexpected end of input inside braces".into()),
        }
    }

    // this won't affect the cursor.
    fn last_char_matches<F>(&mut self, f: F) -> bool
    where
        F: Fn(char) -> bool,
    {
        // if cursor is at the beginning, then do nothing.
        if !self.backup() {
            return false;
        }
        let matched = matches!(self.peek(), Some(ch) if f(ch));
        self.pop();
        matched
    }

    // this won't affect the cursor.
    fn is_colon_the_first_char_in_brackets(&mut self) -> bool {
        // note: colon has already been consumed, so first backup
        self.backup();
        let matched = self.last_char_matches(|ch| ch == '[');
        self.pop();
        matched
    }

    // left brackets has already be consumed.
    fn inside_brackets(&mut self) -> State {
        match self.pop() {
            Some(ch) if ch.is_ascii_whitespace() => State::Space,
            Some(':') => {
                if self.is_colon_scanned() {
                    return State::Err("unexpected second colon(:) in brackets".into());
                }

                if self.is_colon_the_first_char_in_brackets() {
                    return State::Err("expect duration before first colon(:) in brackets".into());
                }

                self.set_colon_scanned();
                State::Lexeme(T_COLON)
            }
            Some(ch) if ch.is_ascii_digit() => self.accept_duration(),
            Some(']') => {
                self.jump_outof_brackets();
                self.reset_colon_scanned();
                State::Lexeme(T_RIGHT_BRACKET)
            }
            Some('[') => State::Err("unexpected left brace '[' inside brackets".into()),
            Some(ch) => State::Err(format!("unexpected character inside brackets: '{ch}'")),
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
    is_alpha(ch) || ch.is_ascii_digit()
}

fn is_alpha(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

pub fn is_label(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        None => false,
        Some(ch) if !is_alpha(ch) => false,
        Some(_) => {
            for ch in chars {
                if !is_alpha_numeric(ch) {
                    return false;
                }
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type LexemeTuple = (TokenId, usize, usize);
    /// - MatchTuple.0 is input
    /// - MatchTuple.1 is the expected generated Lexemes
    /// - MatchTuple.2 is the Err info if the input is invalid PromQL query
    type MatchTuple = (&'static str, Vec<LexemeTuple>, Option<&'static str>);

    fn assert_matches(v: Vec<MatchTuple>) {
        let cases: Vec<(
            &str,
            Vec<Result<LexemeType, String>>,
            Vec<Result<LexemeType, String>>,
        )> = v
            .into_iter()
            .map(|(input, lexemes, err)| {
                let mut expected: Vec<Result<LexemeType, String>> = lexemes
                    .into_iter()
                    .map(|(token_id, start, len)| Ok(LexemeType::new(token_id, start, len)))
                    .collect();

                if err.is_some() {
                    expected.push(Err(err.unwrap().to_string()));
                }

                let actual: Vec<Result<LexemeType, String>> =
                    Lexer::new(input).into_iter().collect();
                (input, expected, actual)
            })
            .collect();

        for (input, expected, actual) in cases.iter() {
            assert_eq!(expected, actual, "\n<input>: {}", input);
        }
    }

    #[test]
    fn test_common() {
        let cases = vec![
            (",", vec![(T_COMMA, 0, 1)], None),
            (
                "()",
                vec![(T_LEFT_PAREN, 0, 1), (T_RIGHT_PAREN, 1, 1)],
                None,
            ),
            (
                "{}",
                vec![(T_LEFT_BRACE, 0, 1), (T_RIGHT_BRACE, 1, 1)],
                None,
            ),
            (
                "[5m]",
                vec![
                    (T_LEFT_BRACKET, 0, 1),
                    (T_DURATION, 1, 2),
                    (T_RIGHT_BRACKET, 3, 1),
                ],
                None,
            ),
            (
                "[ 5m]",
                vec![
                    (T_LEFT_BRACKET, 0, 1),
                    (T_DURATION, 2, 2),
                    (T_RIGHT_BRACKET, 4, 1),
                ],
                None,
            ),
            (
                "[  5m]",
                vec![
                    (T_LEFT_BRACKET, 0, 1),
                    (T_DURATION, 3, 2),
                    (T_RIGHT_BRACKET, 5, 1),
                ],
                None,
            ),
            (
                "[  5m ]",
                vec![
                    (T_LEFT_BRACKET, 0, 1),
                    (T_DURATION, 3, 2),
                    (T_RIGHT_BRACKET, 6, 1),
                ],
                None,
            ),
            ("\r\n\r", vec![], None),
        ];

        assert_matches(cases);
    }

    #[test]
    fn test_numbers() {
        let cases = vec![
            ("1", vec![(T_NUMBER, 0, 1)], None),
            ("4.23", vec![(T_NUMBER, 0, 4)], None),
            (".3", vec![(T_NUMBER, 0, 2)], None),
            ("5.", vec![(T_NUMBER, 0, 2)], None),
            ("NaN", vec![(T_NUMBER, 0, 3)], None),
            ("nAN", vec![(T_NUMBER, 0, 3)], None),
            ("NaN 123", vec![(T_NUMBER, 0, 3), (T_NUMBER, 4, 3)], None),
            ("NaN123", vec![(T_IDENTIFIER, 0, 6)], None),
            ("iNf", vec![(T_NUMBER, 0, 3)], None),
            ("Inf", vec![(T_NUMBER, 0, 3)], None),
            ("+Inf", vec![(T_ADD, 0, 1), (T_NUMBER, 1, 3)], None),
            (
                "+Inf 123",
                vec![(T_ADD, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)],
                None,
            ),
            (
                "-Inf 123",
                vec![(T_SUB, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)],
                None,
            ),
            ("Infoo", vec![(T_IDENTIFIER, 0, 5)], None),
            ("-Inf123", vec![(T_SUB, 0, 1), (T_IDENTIFIER, 1, 6)], None),
            (
                "-Inf 123",
                vec![(T_SUB, 0, 1), (T_NUMBER, 1, 3), (T_NUMBER, 5, 3)],
                None,
            ),
            ("0x123", vec![(T_NUMBER, 0, 5)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_strings() {
        let cases = vec![
            ("\"test\\tsequence\"", vec![(T_STRING, 1, 14)], None),
            ("\"test\\\\.expression\"", vec![(T_STRING, 1, 17)], None),
            (
                "\"test\\.expression\"",
                vec![],
                Some("unknown escape sequence '.'"),
            ),
            (
                "`test\\.expression`",
                vec![],
                Some("unknown escape sequence '.'"),
            ),
            (".٩", vec![], Some("unexpected character after '.': '٩'")),
            // TODO: accept_escape SHOULD support invalid escape character
            // "\xff"
            // `\xff`
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_durations() {
        let cases = vec![
            ("5s", vec![(T_DURATION, 0, 2)], None),
            ("123m", vec![(T_DURATION, 0, 4)], None),
            ("1h", vec![(T_DURATION, 0, 2)], None),
            ("3w", vec![(T_DURATION, 0, 2)], None),
            ("1y", vec![(T_DURATION, 0, 2)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_identifiers() {
        let cases = vec![
            ("abc", vec![(T_IDENTIFIER, 0, 3)], None),
            ("a:bc", vec![(T_METRIC_IDENTIFIER, 0, 4)], None),
            (
                "abc d",
                vec![(T_IDENTIFIER, 0, 3), (T_IDENTIFIER, 4, 1)],
                None,
            ),
            (":bc", vec![(T_METRIC_IDENTIFIER, 0, 3)], None),
            ("0a:bc", vec![], Some("bad number or duration syntax: 0a")),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_comments() {
        let cases = vec![
            ("# some comment", vec![], None),
            ("5 # 1+1\n5", vec![(T_NUMBER, 0, 1), (T_NUMBER, 8, 1)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_operators() {
        let cases = vec![
            ("=", vec![(T_EQL, 0, 1)], None),
            (
                "{=}",
                vec![(T_LEFT_BRACE, 0, 1), (T_EQL, 1, 1), (T_RIGHT_BRACE, 2, 1)],
                None,
            ),
            ("==", vec![(T_EQLC, 0, 2)], None),
            ("!=", vec![(T_NEQ, 0, 2)], None),
            ("<", vec![(T_LSS, 0, 1)], None),
            (">", vec![(T_GTR, 0, 1)], None),
            (">=", vec![(T_GTE, 0, 2)], None),
            ("<=", vec![(T_LTE, 0, 2)], None),
            ("+", vec![(T_ADD, 0, 1)], None),
            ("-", vec![(T_SUB, 0, 1)], None),
            ("*", vec![(T_MUL, 0, 1)], None),
            ("/", vec![(T_DIV, 0, 1)], None),
            ("^", vec![(T_POW, 0, 1)], None),
            ("%", vec![(T_MOD, 0, 1)], None),
            ("AND", vec![(T_LAND, 0, 3)], None),
            ("or", vec![(T_LOR, 0, 2)], None),
            ("unless", vec![(T_LUNLESS, 0, 6)], None),
            ("@", vec![(T_AT, 0, 1)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_aggregators() {
        let cases = vec![
            ("sum", vec![(T_SUM, 0, 3)], None),
            ("AVG", vec![(T_AVG, 0, 3)], None),
            ("Max", vec![(T_MAX, 0, 3)], None),
            ("min", vec![(T_MIN, 0, 3)], None),
            ("count", vec![(T_COUNT, 0, 5)], None),
            ("stdvar", vec![(T_STDVAR, 0, 6)], None),
            ("stddev", vec![(T_STDDEV, 0, 6)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_keywords() {
        let cases = vec![
            ("offset", vec![(T_OFFSET, 0, 6)], None),
            ("by", vec![(T_BY, 0, 2)], None),
            ("without", vec![(T_WITHOUT, 0, 7)], None),
            ("on", vec![(T_ON, 0, 2)], None),
            ("ignoring", vec![(T_IGNORING, 0, 8)], None),
            ("group_left", vec![(T_GROUP_LEFT, 0, 10)], None),
            ("group_right", vec![(T_GROUP_RIGHT, 0, 11)], None),
            ("bool", vec![(T_BOOL, 0, 4)], None),
            ("atan2", vec![(T_ATAN2, 0, 5)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_preprocessors() {
        let cases = vec![
            ("start", vec![(T_START, 0, 5)], None),
            ("end", vec![(T_END, 0, 3)], None),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_selectors() {
        let cases = vec![
            ("北京", vec![], Some("unexpected character: '北'")),
            ("北京='a'", vec![], Some("unexpected character: '北'")),
            ("0a='a'", vec![], Some("bad number or duration syntax: 0a")),
            (
                "{foo='bar'}",
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 3),
                    (T_EQL, 4, 1),
                    (T_STRING, 6, 3),
                    (T_RIGHT_BRACE, 10, 1),
                ],
                None,
            ),
            (
                r#"{foo="bar"}"#,
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 3),
                    (T_EQL, 4, 1),
                    (T_STRING, 6, 3),
                    (T_RIGHT_BRACE, 10, 1),
                ],
                None,
            ),
            (
                r#"{foo="bar\"bar"}"#,
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 3),
                    (T_EQL, 4, 1),
                    (T_STRING, 6, 8),
                    (T_RIGHT_BRACE, 15, 1),
                ],
                None,
            ),
            (
                r#"{NaN	!= "bar" }"#,
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 3),
                    (T_NEQ, 5, 2),
                    (T_STRING, 9, 3),
                    (T_RIGHT_BRACE, 14, 1),
                ],
                None,
            ),
            (
                r#"{alert=~"bar" }"#,
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 5),
                    (T_EQL_REGEX, 6, 2),
                    (T_STRING, 9, 3),
                    (T_RIGHT_BRACE, 14, 1),
                ],
                None,
            ),
            (
                r#"{on!~"bar"}"#,
                vec![
                    (T_LEFT_BRACE, 0, 1),
                    (T_IDENTIFIER, 1, 2),
                    (T_NEQ_REGEX, 3, 2),
                    (T_STRING, 6, 3),
                    (T_RIGHT_BRACE, 10, 1),
                ],
                None,
            ),
            (
                r#"{alert!#"bar"}"#,
                vec![(T_LEFT_BRACE, 0, 1), (T_IDENTIFIER, 1, 5)],
                Some("unexpected character after '!' inside braces: '#'"),
            ),
            (
                r#"{foo:a="bar"}"#,
                vec![(T_LEFT_BRACE, 0, 1), (T_IDENTIFIER, 1, 3)],
                Some("unexpected character inside braces: ':'"),
            ),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_common_errors() {
        let cases = vec![
            ("=~", vec![], Some("unexpected character after '=': '~'")),
            ("!~", vec![], Some("unexpected character after '!': '~'")),
            ("!(", vec![], Some("unexpected character after '!': '('")),
            ("1a", vec![], Some("bad number or duration syntax: 1a")),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_mismatched_parentheses() {
        let cases = vec![
            (
                "(",
                vec![(T_LEFT_PAREN, 0, 1)],
                Some("unclosed left parenthesis"),
            ),
            (")", vec![], Some("unexpected right parenthesis ')'")),
            (
                "())",
                vec![(T_LEFT_PAREN, 0, 1), (T_RIGHT_PAREN, 1, 1)],
                Some("unexpected right parenthesis ')'"),
            ),
            (
                "(()",
                vec![
                    (T_LEFT_PAREN, 0, 1),
                    (T_LEFT_PAREN, 1, 1),
                    (T_RIGHT_PAREN, 2, 1),
                ],
                Some("unclosed left parenthesis"),
            ),
            (
                "{",
                vec![(T_LEFT_BRACE, 0, 1)],
                Some("unexpected end of input inside braces"),
            ),
            ("}", vec![], Some("unexpected right brace '}'")),
            (
                "{{",
                vec![(T_LEFT_BRACE, 0, 1)],
                Some("unexpected left brace '{' inside braces"),
            ),
            (
                "{{}}",
                vec![(T_LEFT_BRACE, 0, 1)],
                Some("unexpected left brace '{' inside braces"),
            ),
            (
                "[",
                vec![(T_LEFT_BRACKET, 0, 1)],
                Some("unexpected end of input inside brackets"),
            ),
            (
                "[[",
                vec![(T_LEFT_BRACKET, 0, 1)],
                Some("unexpected left brace '[' inside brackets"),
            ),
            (
                "[]]",
                vec![(T_LEFT_BRACKET, 0, 1), (T_RIGHT_BRACKET, 1, 1)],
                Some("unexpected right bracket ']'"),
            ),
            (
                "[[]]",
                vec![(T_LEFT_BRACKET, 0, 1)],
                Some("unexpected left brace '[' inside brackets"),
            ),
            ("]", vec![], Some("unexpected right bracket ']'")),
        ];
        assert_matches(cases);
    }

    #[test]
    fn test_subqueries() {
        let cases = vec![
            (
                r#"test_name{on!~"bar"}[4m:4s]"#,
                vec![
                    (T_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                    (T_DURATION, 21, 2),
                    (T_COLON, 23, 1),
                    (T_DURATION, 24, 2),
                    (T_RIGHT_BRACKET, 26, 1),
                ],
                None,
            ),
            (
                r#"test:name{on!~"bar"}[4m:4s]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                    (T_DURATION, 21, 2),
                    (T_COLON, 23, 1),
                    (T_DURATION, 24, 2),
                    (T_RIGHT_BRACKET, 26, 1),
                ],
                None,
            ),
            (
                r#"test:name{on!~"b:ar"}[4m:4s]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 4),
                    (T_RIGHT_BRACE, 20, 1),
                    (T_LEFT_BRACKET, 21, 1),
                    (T_DURATION, 22, 2),
                    (T_COLON, 24, 1),
                    (T_DURATION, 25, 2),
                    (T_RIGHT_BRACKET, 27, 1),
                ],
                None,
            ),
            (
                r#"test:name{on!~"b:ar"}[4m:]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 4),
                    (T_RIGHT_BRACE, 20, 1),
                    (T_LEFT_BRACKET, 21, 1),
                    (T_DURATION, 22, 2),
                    (T_COLON, 24, 1),
                    (T_RIGHT_BRACKET, 25, 1),
                ],
                None,
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s]"#,
                vec![
                    (T_IDENTIFIER, 0, 13),
                    (T_LEFT_PAREN, 13, 1),
                    (T_IDENTIFIER, 14, 4),
                    (T_LEFT_PAREN, 18, 1),
                    (T_IDENTIFIER, 19, 3),
                    (T_LEFT_BRACE, 22, 1),
                    (T_IDENTIFIER, 23, 3),
                    (T_EQL, 26, 1),
                    (T_STRING, 28, 3),
                    (T_RIGHT_BRACE, 32, 1),
                    (T_LEFT_BRACKET, 33, 1),
                    (T_DURATION, 34, 2),
                    (T_RIGHT_BRACKET, 36, 1),
                    (T_RIGHT_PAREN, 37, 1),
                    (T_LEFT_BRACKET, 38, 1),
                    (T_DURATION, 39, 2),
                    (T_COLON, 41, 1),
                    (T_RIGHT_BRACKET, 42, 1),
                    (T_RIGHT_PAREN, 43, 1),
                    (T_LEFT_BRACKET, 44, 1),
                    (T_DURATION, 45, 2),
                    (T_COLON, 47, 1),
                    (T_DURATION, 48, 2),
                    (T_RIGHT_BRACKET, 50, 1),
                ],
                None,
            ),
            (
                r#"test:name{on!~"b:ar"}[4m:4s] offset 10m"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 4),
                    (T_RIGHT_BRACE, 20, 1),
                    (T_LEFT_BRACKET, 21, 1),
                    (T_DURATION, 22, 2),
                    (T_COLON, 24, 1),
                    (T_DURATION, 25, 2),
                    (T_RIGHT_BRACKET, 27, 1),
                    (T_OFFSET, 29, 6),
                    (T_DURATION, 36, 3),
                ],
                None,
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] offset 6m)[4m:3s]"#,
                vec![
                    (T_IDENTIFIER, 0, 13),
                    (T_LEFT_PAREN, 13, 1),
                    (T_IDENTIFIER, 14, 4),
                    (T_LEFT_PAREN, 18, 1),
                    (T_IDENTIFIER, 19, 3),
                    (T_LEFT_BRACE, 22, 1),
                    (T_IDENTIFIER, 23, 3),
                    (T_EQL, 26, 1),
                    (T_STRING, 28, 3),
                    (T_RIGHT_BRACE, 32, 1),
                    (T_LEFT_BRACKET, 33, 1),
                    (T_DURATION, 34, 2),
                    (T_RIGHT_BRACKET, 36, 1),
                    (T_RIGHT_PAREN, 37, 1),
                    (T_LEFT_BRACKET, 38, 1),
                    (T_DURATION, 39, 2),
                    (T_COLON, 41, 1),
                    (T_RIGHT_BRACKET, 42, 1),
                    (T_OFFSET, 44, 6),
                    (T_DURATION, 51, 2),
                    (T_RIGHT_PAREN, 53, 1),
                    (T_LEFT_BRACKET, 54, 1),
                    (T_DURATION, 55, 2),
                    (T_COLON, 57, 1),
                    (T_DURATION, 58, 2),
                    (T_RIGHT_BRACKET, 60, 1),
                ],
                None,
            ),
            (
                r#"test:name[ 5m]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACKET, 9, 1),
                    (T_DURATION, 11, 2),
                    (T_RIGHT_BRACKET, 13, 1),
                ],
                None,
            ),
            (
                r#"test:name{o:n!~"bar"}[4m:4s]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 1),
                ],
                Some("unexpected character inside braces: ':'"),
            ),
            (
                r#"test:name{on!~"bar"}[4m:4s:4h]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                    (T_DURATION, 21, 2),
                    (T_COLON, 23, 1),
                    (T_DURATION, 24, 2),
                ],
                Some("unexpected second colon(:) in brackets"),
            ),
            (
                r#"test:name{on!~"bar"}[4m:4s:]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                    (T_DURATION, 21, 2),
                    (T_COLON, 23, 1),
                    (T_DURATION, 24, 2),
                ],
                Some("unexpected second colon(:) in brackets"),
            ),
            (
                r#"test:name{on!~"bar"}[4m::]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                    (T_DURATION, 21, 2),
                    (T_COLON, 23, 1),
                ],
                Some("unexpected second colon(:) in brackets"),
            ),
            (
                r#"test:name{on!~"bar"}[:4s]"#,
                vec![
                    (T_METRIC_IDENTIFIER, 0, 9),
                    (T_LEFT_BRACE, 9, 1),
                    (T_IDENTIFIER, 10, 2),
                    (T_NEQ_REGEX, 12, 2),
                    (T_STRING, 15, 3),
                    (T_RIGHT_BRACE, 19, 1),
                    (T_LEFT_BRACKET, 20, 1),
                ],
                Some("expect duration before first colon(:) in brackets"),
            ),
        ];
        assert_matches(cases);
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

    #[test]
    fn test_is_label() {
        assert!(is_label("_"));
        assert!(is_label("_up"));
        assert!(is_label("up"));
        assert!(is_label("up_"));
        assert!(is_label("up_system_1"));

        assert!(!is_label(""));
        assert!(!is_label("0"));
        assert!(!is_label("0up"));
        assert!(!is_label("0_up"));
    }
}
