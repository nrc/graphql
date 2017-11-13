/*
Example query:

query {
  human(202) {
    name,
    appearsIn
  }
}
*/
use {QlResult, QlError};
use types::{Name, Id, result, schema};

// TODO variables, directives
#[derive(Clone, Debug)]
pub enum Query {
    Query(Vec<Field>),
    // TODO
    Mutation,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: Name,
    pub alias: Option<Name>,
    pub args: Vec<(Name, Value)>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    String(&'static str),
    Id(&'static str),
    Name(Name),
    Array(Vec<Value>),
    // TODO input object types
}

pub trait Root: result::Resolve {}

// TODO
impl Query {
    pub fn parse(input: &'static str) -> QlResult<Query> {
        unimplemented!();
    }

    pub fn validate(&self, schema: &schema::Schema) -> QlResult<()> {
        unimplemented!();
    }

    pub fn execute<R: Root>(&self, schema: &schema::Schema, root: R) -> QlResult<result::Value> {
        unimplemented!();
    }
}

pub trait FromValue: Sized {
    fn from(value: &Value) -> QlResult<Self>;
}

impl FromValue for String {
    fn from(value: &Value) -> QlResult<String> {
        if let Value::String(s) = *value {
            Ok(s.to_owned())
        } else {
            Err(QlError::LoweringError(format!("{:?}", value), "String".to_owned()))
        }
    }
}
impl FromValue for Id {
    fn from(value: &Value) -> QlResult<Id> {
        if let Value::Id(s) = *value {
            Ok(Id(s.to_owned()))
        } else {
            Err(QlError::LoweringError(format!("{:?}", value), "Id".to_owned()))
        }
    }
}
impl FromValue for Name {
    fn from(value: &Value) -> QlResult<Name> {
        if let Value::Name(ref n) = *value {
            Ok(n.clone())
        } else {
            Err(QlError::LoweringError(format!("{:?}", value), "Name".to_owned()))
        }
    }
}
impl<T: FromValue> FromValue for Vec<T> {
    fn from(value: &Value) -> QlResult<Vec<T>> {
        if let Value::Array(ref a) = *value {
            a.iter().map(|x| T::from(x)).collect()
        } else {
            Err(QlError::LoweringError(format!("{:?}", value), "Array".to_owned()))
        }
    }
}
impl<T: FromValue> FromValue for Option<T> {
    fn from(value: &Value) -> QlResult<Option<T>> {
        if let Value::Null = *value {
            Ok(None)
        } else {
            Ok(Some(T::from(value)?))
        }
    }
}
