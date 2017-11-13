/*
Schema (IDL):

schema {
    query: Query
}

type Query {
    hero(episode: Episode): Character
    human(id : ID!): Human
}

enum Episode {
    NEWHOPE
    EMPIRE
    JEDI
}

interface Character {
    id: ID!
    name: String!
    friends: [Character]
    appearsIn: [Episode]!
}

type Human implements Character {
    id: ID!
    name: String!
    friends: [Character]
    appearsIn: [Episode]!
    homePlanet: String
}

*/

use QlResult;
use types::{Name, query, result};

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Schema {
    pub items: HashMap<Name, Item>,
}

impl Schema {
    pub fn new() -> Schema {
        Schema {
            items: HashMap::new(),
        }
    }
}

pub trait Reflect {
    // Should return a Result since coercion could fail
    fn schema(&self) -> Item;
    fn name(&self) -> Name;
}

pub trait ResolveObject: Reflect + result::Resolve {
    fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value>;
}

pub trait ResolveEnum: Reflect + result::Resolve {
}

#[derive(Clone, Debug)]
pub enum Item {
    Object(Object),
    Interface(Interface),
    Enum(Enum),
}

#[derive(Clone, Debug)]
pub struct Object {
    pub implements: Vec<Name>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub struct Interface {
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub variants: Vec<Name>,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: Name,
    pub args: Vec<(Name, Type)>,
    pub ty: Type,
}

impl Field {
    pub fn field(name: Name, ty: Type) -> Field {
        Field {
            name,
            args: vec![],
            ty,
        }
    }

    pub fn fun(name: Name, args: Vec<(Name, Type)>, ty: Type) -> Field {
        Field {
            name,
            args,
            ty,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Type {
    String,
    Id,
    Name(Name),
    NonNull(Box<Type>),
    Array(Box<Type>),
}

impl Type {
    pub fn non_null(ty: Type) -> Type {
        Type::NonNull(Box::new(ty))
    }

    pub fn array(ty: Type) -> Type {
        Type::Array(Box::new(ty))
    }
}