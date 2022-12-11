mod ast;
mod function;
pub mod lex;
pub mod parse;
pub mod production;
mod token;
pub mod value;

pub use ast::Expr;
pub use function::{get_function, Function};
pub use lex::{lexer, LexemeType, Lexer};
pub use parse::parse;
pub use production::{lexeme_to_string, lexeme_to_token, span_to_string};
pub use token::{Token, TokenType};
pub use value::{Value, ValueType};

use crate::label::{MatchOp, Matcher};

use lrlex::lrlex_mod;
lrlex_mod!("token_map");
pub use token_map::*;
