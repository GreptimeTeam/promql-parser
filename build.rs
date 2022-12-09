use cfgrammar::yacc::YaccKind;
use lrlex::{ct_token_map, DefaultLexeme};
use lrpar::CTParserBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctp = CTParserBuilder::<DefaultLexeme<u8>, u8>::new()
        .yacckind(YaccKind::Grmtools)
        .grammar_in_src_dir("parser/promql.y")?
        .build()?;
    ct_token_map::<u8>("token_map", ctp.token_map(), None)
}
