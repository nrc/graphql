pub mod schema;
pub mod query;
pub mod result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Name(pub String);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Id(pub String);

use std::fmt::{Display, Formatter, Result};

impl Display for Name {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0.fmt(f)
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0.fmt(f)
    }
}
