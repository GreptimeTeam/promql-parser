use std::fmt::{self, Display};

type Pos = i32;

// Item represents a token or text string returned from the scanner.
#[derive(Debug)]
pub struct Item {
    typ: ItemType, // The type of this Item.
    pos: Pos,      // The starting position, in bytes, of this Item in the input string.
    val: String,   // The value of this Item.
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "item {}", self.val)
    }
}

#[derive(Debug)]
pub enum ItemType {}
