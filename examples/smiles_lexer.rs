//! Lex SMILES strings for familiar molecules and print the results.
//!
//! Run with `cargo run --example smiles_lexer`.
use protochem::smiles_lexer::{self, LexError, Lexer};

fn main() -> Result<(), LexError> {
    // L-alanine: each lexeme has a token, its input text and its position
    for lexeme in Lexer::new("N[C@@H](C)C(=O)O") {
        let lexeme = lexeme?;
        println!(
            "{:>2}  {:<7} {:?}",
            lexeme.position, lexeme.text, lexeme.token
        );
    }

    // Aspirin: `lex` collects just the tokens
    let tokens = smiles_lexer::lex("CC(=O)Oc1ccccc1C(=O)O")?;
    println!("aspirin: {} tokens", tokens.len());

    // `Xx` isn't an element, so this one is an error
    if let Err(error) = smiles_lexer::lex("C[Xx]C") {
        println!("error: {error}");
    }

    Ok(())
}
