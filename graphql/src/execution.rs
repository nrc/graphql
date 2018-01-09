use QlResult;
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
