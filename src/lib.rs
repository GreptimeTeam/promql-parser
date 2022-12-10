use lrlex::{lrlex_mod, DefaultLexeme, LRNonStreamingLexer};
use lrpar::{lrpar_mod, Lexeme, NonStreamingLexer, Span};

pub mod label;
pub mod parser;

lrpar_mod!("parser/promql.y");
