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
use types::{query, result};

use std::collections::HashMap;

pub type Name = &'static str;

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

    pub fn validate(&self) -> QlResult<()> {
        // TODO
        Ok(())
    }
}

// TODO should include mutation if provided by user (and maybe query should be optional too?)
pub fn schema_type() -> Item {
    Item::Object(Object {
        implements: vec![],
        fields: vec![Field::fun("query", vec![], Type::Name("Query"))]
    })
}

// QUESTION Reflect and Resolve should probably be elsewhere
pub trait Reflect {
    fn schema() -> Item;
    // TODO should be assoc const
    fn name() -> Name;
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

impl Item {
    pub fn fields(&self) -> &[Field] {
        match *self {
            Item::Object(ref obj) => &obj.fields,
            Item::Interface(ref i) => &i.fields,
            Item::Enum(_) => &[],
        }
    }
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

    pub fn assert_name(&self) -> &Name {
        match *self {
            Type::Name(ref n) => n,
            _ => panic!("Type::assert_name called on non-Name: {:?}", self),
        }
    }

    pub fn is_nullable(&self) -> bool {
        match *self {
            Type::NonNull(_) => false,
            _ => true,
        }
    }

    pub fn as_name_null(&self) -> Option<&Name> {
        match *self {
            Type::Name(ref n) => Some(n),
            Type::NonNull(ref t) => t.as_name_null(),
            _ => None,
        }

    }
}
