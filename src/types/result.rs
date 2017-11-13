/* e.g.
{
  "data": {
    "hero": {
      "name": "R2-D2",
      "friends": [
        {
          "name": "Luke Skywalker"
        },
        {
          "name": "Han Solo"
        },
        {
          "name": "Leia Organa"
        }
      ]
    }
  }
}
*/

use QlResult;
use types::{query, Id, Name};

#[derive(Clone, Debug)]
pub enum Value {
    Id(Id),
    Object(Object),
    Array(Vec<Value>),
    String(String),
    Int(i64),
    Float(f64),
    Null,
}

#[derive(Clone, Debug)]
pub struct Object {
    pub fields: Vec<(Name, Value)>,
}

// QUESTION: Is this the right place for Resolve?
pub trait Resolve {
    fn resolve(&self, fields: &[query::Field]) -> QlResult<Value>;
}

impl Resolve for Id {
    fn resolve(&self, _fields: &[query::Field]) -> QlResult<Value> {
        Ok(Value::Id(self.clone()))
    }
}
impl Resolve for String {
    fn resolve(&self, _fields: &[query::Field]) -> QlResult<Value> {
        Ok(Value::String(self.clone()))
    }
}
impl<T: Resolve> Resolve for Option<T> {
    fn resolve(&self, fields: &[query::Field]) -> QlResult<Value> {
        match self.as_ref() {
            Some(x) => x.resolve(fields),
            None => Ok(Value::Null),
        }
    }
}
impl<T: Resolve> Resolve for Vec<T> {
    fn resolve(&self, fields: &[query::Field]) -> QlResult<Value> {
        // TODO collect all errors not just one
        Ok(Value::Array(self.iter().map(|t| t.resolve(fields)).collect::<Result<Vec<_>, _>>()?))
    }
}
