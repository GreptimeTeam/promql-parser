use promql_parser::parser;

fn main() {
    let promql = "node_cpu_seconds_total{cpu=0,mode=idle}";

    let ast = parser::parse(promql).unwrap();

    println!("AST: {:?}", ast);
}
