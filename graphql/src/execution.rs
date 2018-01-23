use QlResult;
use query::Variables;
use types::{query, result, schema};

pub fn select_fields<O: schema::ResolveObject>(
    object: &O,
    fields: &[query::Field],
) -> QlResult<result::Value> {
    Ok(result::Value::Object(result::Object {
        fields: fields
            .iter()
            .map(|f| Ok((f.name.clone(), object.resolve_field(f)?)))
            .collect::<QlResult<Vec<_>>>()?,
    }))
}

pub struct Context {
    variables: Variables,
}

impl Context {
    pub fn new(variables: Variables) -> Context {
        Context { variables }
    }
}
