use QlResult;
use types::{query, result, Name};

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

    pub fn validate(&self) -> QlResult<()> {
        // TODO
        Ok(())
    }
}

// TODO should include mutation if provided by user (and maybe query should be optional too?)
pub fn schema_type() -> Item {
    Item::Object(Object {
        implements: vec![],
        fields: vec![
            Field::fun(Name("query".to_owned()), vec![], Type::name("Query")),
        ],
    })
}

pub const SCHEMA_NAME: &'static str = "schema";

// QUESTION Reflect and Resolve should probably be elsewhere
pub trait Reflect {
    const NAME: &'static str;

    fn schema() -> Item;
}

pub trait ResolveObject: Reflect + result::Resolve {
    fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value>;
}

pub trait ResolveEnum: Reflect + result::Resolve {}

#[derive(Clone, Debug)]
pub enum Item {
    Schema(Interface),
    Object(Object),
    Interface(Interface),
    Enum(Enum),
}

impl Item {
    pub fn fields(&self) -> &[Field] {
        match *self {
            Item::Object(ref obj) => &obj.fields,
            Item::Schema(ref i) | Item::Interface(ref i) => &i.fields,
            Item::Enum(_) => &[],
        }
    }

    pub fn assert_field(&self, name: Name) -> &Field {
        self.fields().iter().find(|f| f.name == name).expect("Missing field")
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
    // QUESTION: Do we need to distinguish between no arg list and an empty arg list?
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
        Field { name, args, ty }
    }
}

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub nullable: bool,
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    String,
    Id,
    Name(Name),
    Array(Box<Type>),
}

impl Type {
    pub fn non_null(kind: TypeKind) -> Type {
        Type {
            kind,
            nullable: false,
        }
    }

    pub fn array(ty: Type) -> Type {
        Type {
            kind: TypeKind::Array(Box::new(ty)),
            nullable: true,
        }
    }

    pub fn name(s: &str) -> Type {
        Type {
            kind: TypeKind::Name(Name(s.to_owned())),
            nullable: true,
        }
    }

    pub fn assert_name(&self) -> &Name {
        match self.kind {
            TypeKind::Name(ref n) => n,
            _ => panic!("Type::assert_name called on non-Name: {:?}", self),
        }
    }

    pub fn as_name_null(&self) -> Option<&Name> {
        match self.kind {
            TypeKind::Name(ref n) => Some(n),
            _ => None,
        }
    }
}
