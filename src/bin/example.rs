#![feature(decl_macro)]

extern crate graphql;

use graphql::QlResult;
use graphql::types::Id;

use example_generated::*;

fn main() {
    println!("hello!");
}

// User provides
struct DbQuery;

// TODO flesh this out to actually produce data
impl Query for DbQuery {
    fn hero(&self, episode: Option<Episode>) -> QlResult<Box<AbstractCharacter>> { unimplemented!() }
    fn human(&self, id: Id) -> QlResult<Box<AbstractHuman>> { unimplemented!() }
}

ImplQuery!(DbQuery);

// Example of overriding the default implementation:
// use types::{query, result};
// struct MyCharacter;

// ImplCharacter!(MyCharacter);

// impl AbstractCharacter for MyCharacter {
//     fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
//         // magic the field out of thin air
//         unimplemented!();
//     }
// }


mod example_generated {
    use graphql::{execution, QlResult, QlError};
    use graphql::types::{Name, Id, query, result, schema};
    use graphql::types::schema::{ResolveEnum, ResolveObject};
    use graphql::types::query::FromValue;
    use graphql::types::result::Resolve;

    // TODO this is a trait because it has functions. But the other are all fields, therefore structs
    //      but what if there is a mix of both? Have a trait and a struct
    //      What if you want to return a partial object? Or compute a field?
    //      Override resolve_field for your object, schema needs an annotation for not generating an object
    //      How do coercions play into this?
    // TODO context?
    // TODO async
    pub trait Query: query::Root + Resolve {
        // QUESTION Box should be impl eventually? (Could we use assoc types for this?)
        // select_fields could then take object by value, not reference
        fn hero(&self, episode: Option<Episode>) -> QlResult<Box<AbstractCharacter>>;
        fn human(&self, id: Id) -> QlResult<Box<AbstractHuman>>;

    }

    pub macro ImplQuery($concrete: ident) {
        impl query::Root for $concrete {}

        impl Resolve for $concrete {
            // constraint: need to be able to batch and cache queries
            // constraint: partial objects
            // constraint: custom types
            fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                let mut results = vec![];
                for field in fields {
                    match field.name {
                        "hero" => {
                            // Asserts here because this should be ensured by verification.
                            // QUESTION if args.is_empty(), then should we pass null for episode?
                            assert_eq!(field.args.len(), 1);
                            let &(ref name, ref value) = &field.args[0];
                            assert_eq!(name, &"episode");
                            let episode: Option<Episode> = FromValue::from(value)?;
                            let result = self.hero(episode)?;
                            
                            results.push(result.resolve(&field.fields)?);
                        }
                        "human" => {
                            assert_eq!(field.args.len(), 1);
                            let &(ref name, ref value) = &field.args[0];
                            assert_eq!(name, &"id");
                            let id: Id = FromValue::from(value)?;
                            let result = self.human(id)?;
                            
                            results.push(result.resolve(&field.fields)?);
                        }
                        n => return Err(QlError::ExecutionError(format!("Missing field executor: {}", n))),
                    }
                }
                Ok(result::Value::Array(results))
            }
        }
    }

    // TODO adjust naming convention?
    #[derive(Clone, Debug)]
    pub struct Human {
        pub id: Id,
        pub name: String,
        pub friends: Option<Vec<Option<Character>>>,
        pub appearsIn: Vec<Option<Episode>>,
        pub homePlanet: Option<String>,
    }

    pub trait AbstractHuman: ResolveObject {
        // QUESTION could this be impl instead of Box some day?
        fn to_Character(&self) -> QlResult<Box<AbstractCharacter>>;
    }

    pub macro ImplHuman($concrete: ident) {
        // The repr traits let you go from concrete instance to schema object, but how do you go from schema object to concrete instance?
        // default on Human? TryFrom
        impl schema::Reflect for $concrete {
            fn schema(&self) -> schema::Item {
                let char_fields = vec![
                    schema::Field::field("id", schema::Type::non_null(schema::Type::Id)),
                    schema::Field::field("name", schema::Type::non_null(schema::Type::String)),
                    schema::Field::field("friends", schema::Type::array(schema::Type::Name("Character"))),
                    schema::Field::field("appearsIn", schema::Type::non_null(schema::Type::array(schema::Type::Name("Episode")))),
                ];
                let mut fields = char_fields;
                fields.push(schema::Field::field("homePlanet", schema::Type::String));
                schema::Item::Object(schema::Object { implements: vec!["Character"], fields: fields })
            }

            // Alternative:
            // Then look this up in a schema.
            // Maybe we have both? schema -> make_schema_item
            fn name(&self) -> Name {
                "Human"
            }
        }

        impl Resolve for $concrete {
            fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                execution::select_fields(self, fields)
            }
        }
    }

    ImplHuman!(Human);

    impl schema::ResolveObject for Human {
        fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
            match field.name {
                "id" => self.id.resolve(&field.fields),
                "name" => self.name.resolve(&field.fields),
                "friends" => self.friends.resolve(&field.fields),
                "appearsIn" => self.appearsIn.resolve(&field.fields),
                "homePlanet" => self.homePlanet.resolve(&field.fields),
                _ => return Err(QlError::ResolveError("field", field.name.to_owned(), None)),
            }
        }
    }

    impl AbstractHuman for Human {
        fn to_Character(&self) -> QlResult<Box<AbstractCharacter>> {
            Ok(Box::new(Character {
                id: self.id.clone(),
                name: self.name.clone(),
                friends: self.friends.clone(),
                appearsIn: self.appearsIn.clone(),
            }))
        }
    }

    #[derive(Clone, Debug)]
    pub struct Character {
        pub id: Id,
        pub name: String,
        pub friends: Option<Vec<Option<Character>>>,
        pub appearsIn: Vec<Option<Episode>>,
    }

    pub trait AbstractCharacter: ResolveObject {}

    pub macro ImplCharacter($concrete: ident) {
        impl schema::Reflect for $concrete {
            fn schema(&self) -> schema::Item {
                let char_fields = vec![
                    schema::Field::field("id", schema::Type::non_null(schema::Type::Id)),
                    schema::Field::field("name", schema::Type::non_null(schema::Type::String)),
                    schema::Field::field("friends", schema::Type::array(schema::Type::Name("Character"))),
                    schema::Field::field("appearsIn", schema::Type::non_null(schema::Type::array(schema::Type::Name("Episode")))),
                ];
                schema::Item::Object(schema::Object { implements: vec![], fields: char_fields })
            }

            // Alternative:
            // Then look this up in a schema.
            // Maybe we have both? schema -> make_schema_item
            fn name(&self) -> Name {
                "Character"
            }
        }

        impl Resolve for $concrete {
            fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                execution::select_fields(self, fields)
            }
        }
    }

    ImplCharacter!(Character);

    impl ResolveObject for Character {
        fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
            match field.name {
                "id" => self.id.resolve(&field.fields),
                "name" => self.name.resolve(&field.fields),
                "friends" => self.friends.resolve(&field.fields),
                "appearsIn" => self.appearsIn.resolve(&field.fields),
                _ => return Err(QlError::ResolveError("field", field.name.to_owned(), None)),
            }
        }
    }

    impl AbstractCharacter for Character {}

    pub trait AbstractEpisode: ResolveEnum {}

    #[derive(Clone, Debug)]
    pub enum Episode {
        NEWHOPE,
        EMPIRE,
        JEDI,
    }

    // Does this need to be overridable? E.g., to allow int to EpisodeField conversions? Or
    // is it OK to require a custom implementation of AbstractEpisode for that?
    impl FromValue for Episode {
        fn from(value: &query::Value) -> QlResult<Episode> {
            Ok(match &*<String as FromValue>::from(value)? {
                "NEWHOPE" => Episode::NEWHOPE,
                "EMPIRE" => Episode::EMPIRE,
                "JEDI" => Episode::JEDI,
                _ => return Err(QlError::LoweringError(format!("{:?}", value), "Option<Episode>".to_owned())),
            })
        }
    }

    pub macro ImplEpisode($concrete: ident) {
        impl schema::Reflect for $concrete {
            fn schema(&self) -> schema::Item {
                schema::Item::Enum(schema::Enum { variants: vec!["NEWHOPE", "EMPIRE", "JEDI"] })
            }

            fn name(&self) -> Name {
                "Episode"
            }
        }
        impl ResolveEnum for $concrete {}
        impl AbstractEpisode for $concrete {}
    }

    ImplEpisode!(Episode);

    impl Resolve for Episode {
        fn resolve(&self, _fields: &[query::Field]) -> QlResult<result::Value> {
            Ok(match *self {
                Episode::NEWHOPE => result::Value::String("NEWHOPE".to_owned()),
                Episode::EMPIRE => result::Value::String("EMPIRE".to_owned()),
                Episode::JEDI => result::Value::String("JEDI".to_owned()),
            })
        }
    }
}
