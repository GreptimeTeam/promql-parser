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

lrlex::lrlex_mod!("token_map");
pub use token_map::*;

use crate::parser::TokenType;
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
    state: Box<dyn State>,
    ctx: Context,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let ctx = Context::new(input);
        let state = Box::new(LexState);
        Self { state, ctx }
    }
}

impl Iterator for Lexer {
    type Item = LexemeType;

    /// shift then check, if the new state is ready for lexeme.
    /// lexeme is captured from the new state.
    /// err state will terminate the whole lex process.
    ///
    /// FIXME: if last itme befor None is Err, this MUST be shown to users.
    fn next(&mut self) -> Option<Self::Item> {
        while let Ok(state) = self.state.shift(&mut self.ctx) {
            self.state = state;

            let lexeme = self.state.lexeme(&mut self.ctx);
            if lexeme.is_some() {
                return lexeme;
            }
        }
        None

        // loop {
        //     match self.state.shift(&mut self.ctx) {
        //         Ok(state) => self.state = state,
        //         Err(e) => {
        //             eprintln!("{e}");
        //             return None;
        //         }
        //     };

        //     let lexeme = self.state.lexeme(&mut self.ctx);
        //     if lexeme.is_some() {
        //         return lexeme;
        //     }
        // }
    }
}

trait State: Debug {
    // Err will end the state.
    // FIXME: Normal End Should be different from Err.
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String>;

    fn lexeme(&mut self, _: &mut Context) -> Option<LexemeType> {
        println!("call lexeme in State default method");
        None
    }
}

#[derive(Debug)]
pub struct Context {
    chars: Vec<char>,
    start: usize,
    pos: usize, // Current position in the input.

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
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    /// get the first char.
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
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

#[derive(Debug)]
struct LexState;
impl State for LexState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        println!("shift in LexState");
        if ctx.brace_open {
            println!("ctx brace_open, next state is LexInsideBracesState");
            return Ok(Box::new(LexInsideBracesState));
        }

        if ctx.bracket_open {
            println!("ctx bracket_open, next state is LexInsideBracesState");
            return Ok(Box::new(LexDurationState));
        }

        if ctx.peek() == Some('#') {
            println!("comment line. next state is LexLineCommentState");
            return Ok(Box::new(LexLineCommentState));
        }

        match ctx.pop() {
            Some(',') => Ok(Box::new(LexemeState::new(T_COMMA))),
            Some(ch) if is_space(ch) => Ok(Box::new(LexSpaceState)),
            Some('*') => Ok(Box::new(LexemeState::new(T_MUL))),
            Some('/') => Ok(Box::new(LexemeState::new(T_DIV))),
            Some('%') => Ok(Box::new(LexemeState::new(T_MOD))),
            Some('+') => Ok(Box::new(LexemeState::new(T_ADD))),
            Some('-') => Ok(Box::new(LexemeState::new(T_SUB))),
            Some('^') => Ok(Box::new(LexemeState::new(T_POW))),
            Some('=') => match ctx.peek() {
                Some('=') => {
                    ctx.pop();
                    Ok(Box::new(LexemeState::new(T_EQLC)))
                }
                // =~ (label matcher) MUST be in brace, which will be handled in LexInsideBracesState
                Some('~') => Err("unexpected character after '=': ~".into()),
                _ => Ok(Box::new(LexemeState::new(T_EQL))),
            },
            Some('!') => match ctx.pop() {
                Some('=') => Ok(Box::new(LexemeState::new(T_NEQ))),
                Some(ch) => Err(format!("unexpected character after '!': {}", ch)),
                None => Err(format!("'!' can not be at the end")),
            },
            Some('<') => match ctx.peek() {
                Some('=') => {
                    ctx.pop();
                    Ok(Box::new(LexemeState::new(T_LTE)))
                }
                _ => Ok(Box::new(LexemeState::new(T_LSS))),
            },
            Some('>') => match ctx.peek() {
                Some('=') => {
                    ctx.pop();
                    Ok(Box::new(LexemeState::new(T_GTE)))
                }
                _ => Ok(Box::new(LexemeState::new(T_GTR))),
            },
            Some(ch) if is_digit(ch) => Ok(Box::new(LexNumberOrDurationState::new(ch))),
            Some('.') => match ctx.peek() {
                Some(ch) if is_digit(ch) => Ok(Box::new(LexNumberOrDurationState::new(ch))),
                Some(ch) => Err(format!("unexpected character after '.' {}", ch)),
                None => Err(format!("'.' can not be at the end")),
            },
            Some(ch) if is_string_open(ch) => {
                ctx.string_open = true;
                Ok(Box::new(LexStringState::new(ch)))
            }
            Some(ch) if is_alpha(ch) => Ok(Box::new(LexKeywordOrIdentifierState::new(ch))),
            Some(':') if !ctx.bracket_open => Ok(Box::new(LexKeywordOrIdentifierState::new(':'))),
            Some(':') if !ctx.got_colon => {
                ctx.got_colon = true;
                Ok(Box::new(LexemeState::new(T_COLON)))
            }
            // : is in [], and : is already found once
            Some(':') => Err(format!("unexpected colon ':'")),

            Some('(') => {
                ctx.paren_depth += 1;
                Ok(Box::new(LexemeState::new(T_LEFT_PAREN)))
            }
            Some(')') => {
                if ctx.paren_depth == 0 {
                    Err(format!("unexpected right parenthesis ')'"))
                } else {
                    ctx.paren_depth -= 1;
                    Ok(Box::new(LexemeState::new(T_RIGHT_PAREN)))
                }
            }
            // NOTE: pay attention to the space after left brace, cover it in testcases.
            Some('{') => {
                ctx.brace_open = true;
                Ok(Box::new(LexemeState::new(T_LEFT_BRACE)))
            }
            Some('}') if !ctx.brace_open => Err("unexpected right bracket '}'".into()),
            Some('}') => {
                ctx.brace_open = false;
                Ok(Box::new(LexemeState::new(T_RIGHT_BRACE)))
            }
            // NOTE: pay attention to the space after left bracket, cover it in testcases.
            Some('[') => {
                ctx.got_colon = false;
                ctx.bracket_open = true;
                Ok(Box::new(LexemeState::new(T_LEFT_BRACKET)))
            }
            Some(']') if !ctx.bracket_open => Err("unexpected right bracket ']'".into()),
            Some(']') => {
                ctx.bracket_open = false;
                Ok(Box::new(LexemeState::new(T_RIGHT_BRACKET)))
            }
            Some('@') => Ok(Box::new(LexemeState::new(T_AT))),

            Some(ch) => Err(format!("unexpected character: {}", ch)),
            None => return Ok(Box::new(LexEndState)),
        }
    }
}

/// This won't shift state, and will terminate the lexer.
#[derive(Debug)]
struct LexEndState;
impl State for LexEndState {
    fn shift(&mut self, _ctx: &mut Context) -> Result<Box<dyn State>, String> {
        Err("no need to shift after EndState".into())
    }
}

/// LexemeState is where the lexeme will be captured.
#[derive(Debug)]
struct LexemeState {
    token_id: TokenType,
}
impl LexemeState {
    pub fn new(token_id: TokenType) -> Self {
        Self { token_id }
    }
}

impl State for LexemeState {
    /// After lexeme is captured, state will back to LexState with modified fields.
    fn shift(&mut self, _ctx: &mut Context) -> Result<Box<dyn State>, String> {
        println!("shift in LexemeState, next -> LexState");
        Ok(Box::new(LexState))
    }

    fn lexeme(&mut self, ctx: &mut Context) -> Option<LexemeType> {
        println!("Call lexeme in LexemeState");
        Some(ctx.lexeme(dbg!(self.token_id)))
    }
}

// #[derive(Debug)]
// struct LexInsideBracesState;
// impl State for LexInsideBracesState {
//     fn shift(&mut self, ctx: &mut Context) -> Box<dyn State> {
//         todo!()
//     }
// }

// #[derive(Debug)]
// struct LexLineCommentState;
// impl LexLineCommentState {}

// impl State for LexLineCommentState {
//     fn shift(&mut self, ctx: &mut Context) -> Box<dyn State> {
//         todo!()
//     }
// }

#[derive(Debug)]
struct LexSpaceState;
impl State for LexSpaceState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        while let Some(ch) = ctx.peek() {
            if is_space(ch) {
                ctx.pop();
            } else {
                break;
            }
        }
        ctx.align_pos();
        Ok(Box::new(LexState))
    }
}

#[derive(Debug)]
struct LexStringState {
    ch: char, // ' or " or `
}

impl LexStringState {
    pub fn new(ch: char) -> Self {
        Self { ch }
    }
}

impl State for LexStringState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexNumberOrDurationState {
    ch: char, // the leading char which has been popped from the context
}

impl LexNumberOrDurationState {
    pub fn new(ch: char) -> Self {
        Self { ch }
    }
}

impl State for LexNumberOrDurationState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexKeywordOrIdentifierState {
    ch: char,
}
impl LexKeywordOrIdentifierState {
    pub fn new(ch: char) -> Self {
        Self { ch }
    }
}

impl State for LexKeywordOrIdentifierState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexDurationState;

impl State for LexDurationState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexInsideBracesState;

impl State for LexInsideBracesState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexLineCommentState;
impl State for LexLineCommentState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
    }
}

#[derive(Debug)]
struct LexEscapeState;
impl State for LexEscapeState {
    fn shift(&mut self, ctx: &mut Context) -> Result<Box<dyn State>, String> {
        todo!()
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
        let lexer = Lexer::new(",");
        for lex in lexer {
            println!("{:?}", lex);
        }
    }
}
