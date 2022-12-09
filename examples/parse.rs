use promql_parser::parser;

fn main() {
    let promql = "12";

    let ast = parser::parse(promql).unwrap();

    println!("AST: {:?}", ast);
}
