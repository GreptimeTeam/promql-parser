use lrpar::lrpar_mod;

pub mod label;
pub mod parser;
pub mod util;

lrpar_mod!("parser/promql.y");
