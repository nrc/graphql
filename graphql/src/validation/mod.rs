use {QlError, QlResult};
use query::{Field, Query, Value};
use schema::{self, Schema};
use types::Name;

use std::collections::HashSet;

// QUESTION: we do a lot of lookups in lists, these are O(n) where hashing is O(1)
// however, n is usually pretty small. Is it worth using hashing?

pub fn validate_query(query: &Query, schema: &Schema) -> QlResult<()> {
    let mut ctx = Context::new(schema);

    match *query {
        Query::Query(ref fields) => {
            validate_fields(fields, &schema.items[&Name("schema".to_owned())], &mut ctx);
        }
        Query::Mutation => unimplemented!(),
    }

    if ctx.errors.is_empty() {
        Ok(())
    } else {
        Err(QlError::ValidationError(ctx.errors))
    }
}

pub type Error = &'static str;

struct Context<'a> {
    errors: Vec<Error>,
    schema: &'a Schema,
}

impl<'a> Context<'a> {
    fn new(schema: &'a Schema) -> Context<'a> {
        Context {
            errors: vec![],
            schema,
        }
    }
}

// {
//   human(id: 1002) {
//     name,
//     appearsIn,
//     id
//   }
// }

fn validate_fields(fields: &[Field], ty: &schema::Item, ctx: &mut Context) {
    let ty_fields = ty.fields();

    if ty_fields.is_empty() && !fields.is_empty() {
        ctx.errors.push("fields on scalar type");
        return;
    }
    if !ty_fields.is_empty() && fields.is_empty() {
        ctx.errors.push("object type must have fields");
    }

    let mut names = HashSet::new();
    for f in fields {
        if names.contains(&*f.name.0) {
            ctx.errors.push("duplicate field");
        }
        names.insert(&*f.name.0);

        let field_ty = get_field(ty_fields, &f.name);
        let field_ty = match field_ty {
            Some(field_ty) => field_ty,
            None => {
                println!("Looking for {} in {:?}", f.name, ty_fields);
                ctx.errors.push("field not found");
                continue;
            }
        };
        validate_field(f, field_ty, ctx);
    }
}

fn validate_field(field: &Field, ty: &schema::Field, ctx: &mut Context) {
    validate_args(&field.args, &ty.args, ctx);

    // TODO what do the fields on an array type look like?
    match ty.ty.as_name_null() {
        Some(n) => match ctx.schema.items.get(n) {
            Some(item) => {
                validate_fields(&field.fields, item, ctx);
            }
            None => ctx.errors.push("type not found"),
        },
        None if !field.fields.is_empty() => {
            ctx.errors.push("fields on scalar type");
        }
        _ => {}
    }
}

fn validate_args(args: &[(Name, Value)], ty: &[(Name, schema::Type)], ctx: &mut Context) {
    fn get_type<'a>(name: &Name, ty: &'a [(Name, schema::Type)]) -> Option<&'a schema::Type> {
        for &(ref n, ref t) in ty {
            if name == n {
                return Some(t);
            }
        }
        None
    }

    let mut names = HashSet::new();
    for a in args {
        if names.contains(&a.0) {
            ctx.errors.push("duplicate argument");
        }
        names.insert(&a.0);

        match get_type(&a.0, ty) {
            Some(ty) => {
                validate_value(&a.1, ty, ctx);
            }
            None => {
                ctx.errors.push("argument not found");
            }
        }
    }

    for t in ty {
        if !names.contains(&t.0) && !(t.1).nullable {
            ctx.errors.push("missing argument");
        }
    }
}

fn validate_value(value: &Value, ty: &schema::Type, ctx: &mut Context) {
    if let Value::Null = *value {
        if !ty.nullable {
            ctx.errors.push("null value must be non-null");
        }
    }

    match ty.kind {
        schema::TypeKind::String => match *value {
            Value::Null | Value::String(_) => {}
            _ => {
                ctx.errors.push("type mismatch");
            }
        },
        schema::TypeKind::Id => match *value {
            Value::Null | Value::Name(_) => {}
            _ => {
                ctx.errors.push("type mismatch");
            }
        },
        // TODO do we need to lookup the name and check that the value matches it?
        // yeah, we do e.g., enum values or whatever. Not exactly sure how to do
        // that though since the values of an enum can be defined by the impl.
        // What does the spec say?
        schema::TypeKind::Name(_) => match *value {
            Value::Null | Value::Name(_) => {}
            _ => {
                ctx.errors.push("type mismatch");
            }
        },
        schema::TypeKind::Array(ref el_ty) => match *value {
            Value::Null => {}
            Value::Array(ref values) => for v in values {
                validate_value(v, el_ty, ctx);
            },
            _ => {
                ctx.errors.push("type mismatch");
            }
        },
    }
}

fn get_field<'a>(fields: &'a [schema::Field], name: &Name) -> Option<&'a schema::Field> {
    for f in fields {
        if &f.name == name {
            return Some(f);
        }
    }
    None
}

#[cfg(test)]
mod test {
}
