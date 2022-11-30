use std::fmt::{self, Display};

use super::{Node, PositionRange};

pub type Pos = i32;
pub type ItemType = i32;

// Item represents a token or text string returned from the scanner.
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

impl Node for Item {
    fn pos_range(&self) -> PositionRange {
        PositionRange {
            start: self.pos,
            end: self.pos + self.val.len() as i32,
        }
    }
}
