use std::fmt::{self, Display};

pub type TokenType = u8;

// Item represents a token or text string returned from the scanner.
#[derive(Debug)]
pub struct Token {
    id: TokenType, // The type of this Item.
    val: String,   // The value of this Item.
}

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "lexer token. id: {}, val: {}", self.id, self.val)
    }
}

impl Token {
    pub fn new(id: TokenType, val: String) -> Self {
        Self { id, val }
    }

    pub fn id(&self) -> TokenType {
        self.id
    }

    pub fn val(&self) -> String {
        self.val.clone()
    }
}
