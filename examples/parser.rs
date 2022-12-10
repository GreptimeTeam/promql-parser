use promql_parser::parser;

fn main() {
    let promql = "1h";

    let ast = parser::parse(promql).unwrap();

    println!("AST: {:?}", ast);
}
