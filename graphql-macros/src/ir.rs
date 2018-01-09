use graphql::types::{schema, Name};
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use proc_macro::{quote, TokenStream};

#[derive(Clone, Debug)]
pub struct Schema {
    pub items: HashMap<Name, Item>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Object(Object),
    // TODO Do we need this?
    Schema(Object),
    Enum(Enum),
}

impl Item {
    pub fn name(&self) -> &Name {
        match *self {
            Item::Object(ref o) => &o.name,
            Item::Schema(ref o) => &o.name,
            Item::Enum(ref e) => &e.name,
        }
    }

    pub fn assert_object(&self) -> &Object {
        match *self {
            Item::Object(ref obj) => obj,
            _ => panic!("expected object, found {:?}", self),
        }
    }

    pub fn emit_schema(&self) -> TokenStream {
        match *self {
            Item::Object(ref o) => o.emit_schema(),
            Item::Schema(_) => Schema::emit_schema(),
            Item::Enum(ref e) => e.emit_schema(),
        }
    }

    pub fn emit_assoc_ty(&self) -> TokenStream {
        match *self {
            Item::Object(ref o) => o.emit_assoc_ty(),
            Item::Schema(_) => quote!(),
            Item::Enum(ref e) => e.emit_assoc_ty(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Object {
    pub name: Name,
    pub implements: Vec<Name>,
    pub fields: Vec<Field>,
    // Names in the object which must be kept abstract.
    pub abs_names: HashSet<Name>,
    // True if there are any non-function fields.
    pub has_fields: bool,
    // True if there are any function fields.
    pub has_fns: bool,
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: Name,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct Variant(pub Name);

#[derive(Clone, Debug)]
pub struct Field {
    pub name: Name,
    pub args: Vec<(Name, Type)>,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub nullable: bool,
}

impl Type {
    fn name(&self) -> Option<Name> {
        match self.kind {
            TypeKind::Name(ref n) => Some(n.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    String,
    Id,
    Name(Name),
    Array(Box<Type>),
}

pub fn lower_schema(schema: &schema::Schema) -> Schema {
    Schema {
        items: schema
            .items
            .iter()
            .map(|(n, i)| (n.clone(), lower_item(n, i)))
            .collect(),
    }
}

fn lower_item(name: &Name, item: &schema::Item) -> Item {
    match *item {
        schema::Item::Schema(ref i) => Item::Schema(lower_interface(name, i)),
        schema::Item::Object(ref o) => Item::Object(lower_object(name, o)),
        schema::Item::Interface(ref i) => Item::Object(lower_interface(name, i)),
        schema::Item::Enum(ref e) => Item::Enum(lower_enum(name, e)),
    }
}

fn lower_interface(name: &Name, interface: &schema::Interface) -> Object {
    let fields: Vec<Field> = interface.fields.iter().map(|f| lower_field(f)).collect();
    let has_fields = fields.iter().any(|f| f.args.is_empty());
    let has_fns = fields.iter().any(|f| !f.args.is_empty());
    let mut abs_names = HashSet::new();
    add_types_from_fields(&fields, &mut abs_names);
    Object {
        name: name.clone(),
        implements: vec![],
        fields,
        abs_names,
        has_fields,
        has_fns,
    }
}

fn lower_object(name: &Name, object: &schema::Object) -> Object {
    let fields: Vec<Field> = object.fields.iter().map(|f| lower_field(f)).collect();
    let has_fields = fields.iter().any(|f| f.args.is_empty());
    let has_fns = fields.iter().any(|f| !f.args.is_empty());
    let mut abs_names = HashSet::from_iter(object.implements.clone().into_iter());
    add_types_from_fields(&fields, &mut abs_names);
    Object {
        name: name.clone(),
        implements: object.implements.clone(),
        fields,
        abs_names,
        has_fields,
        has_fns,
    }
}

fn add_types_from_fields(fields: &[Field], abs_names: &mut HashSet<Name>) {
    for f in fields {
        if !f.args.is_empty() {
            if let Some(n) = f.ty.name() {
                abs_names.insert(n);
            }
        }
        for a in &f.args {
            if let Some(n) = a.1.name() {
                abs_names.insert(n);
            }
        }
    }
}

fn lower_enum(name: &Name, e: &schema::Enum) -> Enum {
    Enum {
        name: name.clone(),
        variants: e.variants.iter().map(|n| Variant(n.clone())).collect(),
    }
}

fn lower_field(field: &schema::Field) -> Field {
    Field {
        name: field.name.clone(),
        args: field
            .args
            .iter()
            .map(|&(ref n, ref t)| (n.clone(), lower_type(t)))
            .collect(),
        ty: lower_type(&field.ty),
    }
}

fn lower_type(ty: &schema::Type) -> Type {
    Type {
        kind: match ty.kind {
            schema::TypeKind::String => TypeKind::String,
            schema::TypeKind::Id => TypeKind::Id,
            schema::TypeKind::Name(ref n) => TypeKind::Name(n.clone()),
            schema::TypeKind::Array(ref ty) => TypeKind::Array(Box::new(lower_type(ty))),
        },
        nullable: ty.nullable,
    }
}
