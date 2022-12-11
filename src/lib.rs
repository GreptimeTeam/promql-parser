pub mod label;
pub mod parser;
pub mod util;

use lrpar::lrpar_mod;
lrpar_mod!("parser/promql.y");
