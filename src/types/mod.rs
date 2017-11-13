pub mod schema;
pub mod query;
pub mod result;

pub type Name = &'static str;

#[derive(Debug, Clone)]
pub struct Id(String);
