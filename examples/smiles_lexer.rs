//! Lex a SMILES string and print the tokens.
//!
//! Run with `pixi run -e dev cargo run --example smiles_lexer`.
use protochem::smiles_lexer::{self};

fn main() {
    let tokens = smiles_lexer::lex("[12C@@H2+:1][c@OH28++:2][Fe+4:99]C1=(C#O1)c%33-c/n%33.C");
    println!("tokens: {tokens:?}");
}
