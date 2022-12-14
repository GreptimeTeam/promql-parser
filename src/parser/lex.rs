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
use lrlex::{DefaultLexeme, LRNonStreamingLexer};
use lrpar::Lexeme;
use std::fmt::Debug;

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
    Space,
    String(char),
    KeywordOrIdentifier(char),
    NumberOrDuration(char),
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
                    Some(ch) if is_digit(ch) => LexerState::NumberOrDuration(ch),
                    Some('.') => match ctx.peek() {
                        Some(ch) if is_digit(ch) => LexerState::NumberOrDuration(ch),
                        Some(ch) => {
                            LexerState::Err(format!("unexpected character after '.' {}", ch))
                        }
                        None => LexerState::Err(format!("'.' can not be at the end")),
                    },
                    Some(ch) if is_string_open(ch) => {
                        ctx.string_open = true;
                        LexerState::String(ch)
                    }
                    Some(ch) if is_alpha(ch) => LexerState::KeywordOrIdentifier(ch),
                    Some(':') if !ctx.bracket_open => LexerState::KeywordOrIdentifier(':'),
                    Some(':') if !ctx.got_colon => {
                        ctx.got_colon = true;
                        LexerState::Lexeme(T_COLON)
                    }
                    // : is in [], and : is already found once
                    Some(':') => LexerState::Err(format!("unexpected colon ':'")),

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
                while let Some(ch) = ctx.peek() {
                    if is_space(ch) {
                        ctx.pop();
                    } else {
                        break;
                    }
                }
                ctx.align_pos();
                LexerState::Start
            }
            LexerState::String(_) => todo!(),
            LexerState::KeywordOrIdentifier(_) => todo!(),
            LexerState::NumberOrDuration(_) => todo!(),
            LexerState::Duration => todo!(),
            LexerState::InsideBraces => todo!(),
            LexerState::LineComment => todo!(),
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

    /// get the first char.
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.idx + 1).copied()
    }

    /// caller MUST hold the token_id and only need the span from the context.
    pub fn lexeme(&mut self, token_id: TokenType) -> LexemeType {
        let lexeme = DefaultLexeme::new(token_id, self.start, self.pos - self.start);
        self.align_pos();
        lexeme
    }

    /// ignore the text between start and pos
    pub fn align_pos(&mut self) {
        self.start = self.pos;
    }
}

fn is_string_open(ch: char) -> bool {
    ch == '"' || ch == '`' || ch == '\''
}

fn is_space(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r'
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
        // let lexer = Lexer::new("= == != ,+  -*  /% ! == ");
        let lexer = Lexer::new("!a=");
        for lex in lexer {
            match lex {
                Ok(lexeme) => println!("{:?}, display:{}", lexeme, token_display(lexeme.tok_id())),
                Err(e) => println!("{e}"),
            }
        }
    }
}
