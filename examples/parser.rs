use promql_parser::parser;

fn main() {
    let promql = "this is to test string literal";

    let ast = parser::parse(promql).unwrap();

    println!("AST: {:?}", ast);
}
