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
        Ok(Value::Array(self.iter()
            .map(|t| t.resolve(fields))
            .collect::<Result<Vec<_>, _>>()?))
    }
}

mod display {
    use super::*;
    use std::fmt::{Display, Formatter, Result};

    impl Display for Value {
        fn fmt(&self, f: &mut Formatter) -> Result {
            match *self {
                Value::Id(ref id) => write!(f, "{}", id),
                Value::Object(ref obj) => obj.fmt(f),
                Value::Array(ref vals) => {
                    write!(f, "[")?;
                    let mut first = true;
                    for v in vals {
                        if first {
                            first = false;
                        } else {
                            write!(f, ",")?;
                        }
                        v.fmt(f)?;
                    }
                    write!(f, "]")
                }
                Value::String(ref s) => write!(f, "\"{}\"", s),
                Value::Int(n) => write!(f, "{}", n),
                Value::Float(n) => write!(f, "{}", n),
                Value::Null => write!(f, "null"),
            }
        }
    }

    impl Display for Object {
        fn fmt(&self, f: &mut Formatter) -> Result {
            write!(f, "{{")?;
            let mut first = true;
            for &(ref n, ref v) in &self.fields {
                if first {
                    first = false;
                } else {
                    write!(f, ",")?;
                }
                write!(f, "{}:", n.0)?;
                v.fmt(f)?;
            }
            write!(f, "}}")
        }
    }
}
