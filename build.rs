// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
