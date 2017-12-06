#![feature(decl_macro)]
#![feature(associated_type_defaults)]

extern crate graphql;
extern crate graphql_macros;

use graphql::QlResult;
use graphql::types::Id;
use graphql::types::query::Root;

use example_generated::*;

fn main() {
    let query = "{
      human(id: 1002) {
        name,
        appearsIn,
        id
      }
    }";

    match graphql::handle_query(query, &DbQuery::make_schema(), DbQuery) {
        Ok(result) => println!("{}", result),
        Err(err) => println!("{:?}", err),
    }
}

// User provides
struct DbQuery;

// TODO flesh this out to actually produce data
impl Query for DbQuery {
    // type Character = MyCharacter;
    // QUESTION default assoc types do nothing? - https://github.com/rust-lang/rust/issues/35986
    type Character = Character;
    type Human = Human;
    type Episode = Episode;

    fn hero(&self, _episode: Option<Episode>) -> QlResult<Character> {
        Ok(Character {
            id: Id("0".to_owned()),
            name: "Bob".to_owned(),
            friends: Some(vec![]),
            appearsIn: vec![],
        })
    }

    fn human(&self, _id: Id) -> QlResult<Self::Human> {
        Ok(Human {
            id: Id("0".to_owned()),
            name: "Bob".to_owned(),
            friends: Some(vec![]),
            appearsIn: vec![],
            homePlanet: None,
        })
    }
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
    use graphql::types::{self, Id, Name, query, result, schema};
    use graphql::types::schema::{Reflect, ResolveEnum, ResolveObject};
    use graphql::types::query::FromValue;
    use graphql::types::result::Resolve;

    // TODO this is a trait because it has functions. But the other are all fields, therefore structs
    //      but what if there is a mix of both? Have a trait and a struct
    //      What if you want to return a partial object? Or compute a field?
    //      Override resolve_field for your object, schema needs an annotation for not generating an object
    //      How do coercions play into this?
    // TODO context?
    // TODO async
    pub trait Query: query::Root + Resolve + Reflect {
        type Character: AbstractCharacter = Character;
        type Human: AbstractHuman = Human;
        type Episode: AbstractEpisode + FromValue = Episode;

        // QUESTION Box should be impl eventually? (Could we use assoc types for this?)
        // select_fields could then take object by value, not reference
        fn hero(&self, episode: Option<Self::Episode>) -> QlResult<Self::Character>;
        fn human(&self, id: Id) -> QlResult<Self::Human>;

    }

    pub macro ImplQuery($concrete: ident) {
        impl query::Root for $concrete {
            fn make_schema() -> schema::Schema {
                let mut schema = schema::Schema::new();
                schema.items.insert(Name("schema".to_owned()), schema::schema_type());
                schema.items.insert(Name($concrete::NAME.to_owned()), $concrete::schema());
                schema.items.insert(Name(Human::NAME.to_owned()), Human::schema());
                schema.items.insert(Name(Character::NAME.to_owned()), Character::schema());
                schema.items.insert(Name(Episode::NAME.to_owned()), Episode::schema());
                assert!(schema.validate().is_ok());
                schema
            }
        }

        impl schema::Reflect for $concrete {
            const NAME: &'static str = "Query";

            fn schema() -> schema::Item {
                schema::Item::Object(schema::Object { implements: vec![], fields: vec![
                    schema::Field::fun(Name("hero".to_owned()), vec![(Name("episode".to_owned()), schema::Type::Name(Name("Episode".to_owned())))], schema::Type::Name(Name("Character".to_owned()))),
                    schema::Field::fun(Name("human".to_owned()), vec![(Name("id".to_owned()), schema::Type::non_null(schema::Type::Id))], schema::Type::Name(Name("Human".to_owned()))),
                ] })
            }
        }

        impl Resolve for $concrete {
            // constraint: need to be able to batch and cache queries
            // constraint: partial objects
            // constraint: custom types
            fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                let mut results = vec![];
                for field in fields {
                    match &*field.name.0 {
                        "hero" => {
                            // Asserts here because this should be ensured by verification.
                            // QUESTION if args.is_empty(), then should we pass null for episode?
                            assert_eq!(field.args.len(), 1);
                            let &(ref name, ref value) = &field.args[0];
                            assert_eq!(&*name.0, "episode");
                            let episode: Option<<Self as Query>::Episode> = FromValue::from(value)?;
                            let result = self.hero(episode)?;
                            let result = result.resolve(&field.fields)?;
                            
                            results.push((types::Name("hero".to_owned()), result));
                        }
                        "human" => {
                            assert_eq!(field.args.len(), 1);
                            let &(ref name, ref value) = &field.args[0];
                            assert_eq!(&*name.0, "id");
                            let id: Id = FromValue::from(value)?;
                            let result = self.human(id)?;
                            let result = result.resolve(&field.fields)?;
                            
                            results.push((types::Name("human".to_owned()), result));
                        }
                        n => return Err(QlError::ExecutionError(format!("Missing field executor: {}", n))),
                    }
                }
                Ok(result::Value::Object(result::Object { fields: results } ))
            }
        }
    }

    // TODO adjust naming convention?
    #[allow(non_snake_case)]
    #[derive(Clone, Debug)]
    pub struct Human {
        pub id: Id,
        pub name: String,
        pub friends: Option<Vec<Option<Character>>>,
        pub appearsIn: Vec<Option<Episode>>,
        pub homePlanet: Option<String>,
    }

    pub trait AbstractHuman: ResolveObject {
        type Character: AbstractCharacter;

        #[allow(non_snake_case)]
        fn to_Character(&self) -> QlResult<Self::Character>;
    }

    pub macro ImplHuman($concrete: ident) {
        impl schema::Reflect for $concrete {
            const NAME: &'static str = "Human";

            fn schema() -> schema::Item {
                let char_fields = vec![
                    schema::Field::field(Name("id".to_owned()), schema::Type::non_null(schema::Type::Id)),
                    schema::Field::field(Name("name".to_owned()), schema::Type::non_null(schema::Type::String)),
                    schema::Field::field(Name("friends".to_owned()), schema::Type::array(schema::Type::Name(Name("Character".to_owned())))),
                    schema::Field::field(Name("appearsIn".to_owned()), schema::Type::non_null(schema::Type::array(schema::Type::Name(Name("Episode".to_owned()))))),
                ];
                let mut fields = char_fields;
                fields.push(schema::Field::field(Name("homePlanet".to_owned()), schema::Type::String));
                schema::Item::Object(schema::Object { implements: vec![Name("Character".to_owned())], fields: fields })
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
            match &*field.name.0 {
                "id" => self.id.resolve(&field.fields),
                "name" => self.name.resolve(&field.fields),
                "friends" => self.friends.resolve(&field.fields),
                "appearsIn" => self.appearsIn.resolve(&field.fields),
                "homePlanet" => self.homePlanet.resolve(&field.fields),
                _ => return Err(QlError::ResolveError("field", field.name.to_string(), None)),
            }
        }
    }

    impl AbstractHuman for Human {
        type Character = Character;

        fn to_Character(&self) -> QlResult<Character> {
            Ok(Character {
                id: self.id.clone(),
                name: self.name.clone(),
                friends: self.friends.clone(),
                appearsIn: self.appearsIn.clone(),
            })
        }
    }

    #[allow(non_snake_case)]
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
            const NAME: &'static str = "Character";

            fn schema() -> schema::Item {
                let char_fields = vec![
                    schema::Field::field(Name("id".to_owned()), schema::Type::non_null(schema::Type::Id)),
                    schema::Field::field(Name("name".to_owned()), schema::Type::non_null(schema::Type::String)),
                    schema::Field::field(Name("friends".to_owned()), schema::Type::array(schema::Type::Name(Name("Character".to_owned())))),
                    schema::Field::field(Name("appearsIn".to_owned()), schema::Type::non_null(schema::Type::array(schema::Type::Name(Name("Episode".to_owned()))))),
                ];
                schema::Item::Object(schema::Object { implements: vec![], fields: char_fields })
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
            match &*field.name.0 {
                "id" => self.id.resolve(&field.fields),
                "name" => self.name.resolve(&field.fields),
                "friends" => self.friends.resolve(&field.fields),
                "appearsIn" => self.appearsIn.resolve(&field.fields),
                _ => return Err(QlError::ResolveError("field", field.name.to_string(), None)),
            }
        }
    }

    impl AbstractCharacter for Character {}

    pub trait AbstractEpisode: ResolveEnum {}

    #[allow(non_snake_case)]
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
            const NAME: &'static str = "Episode";

            fn schema() -> schema::Item {
                schema::Item::Enum(schema::Enum { variants: vec![Name("NEWHOPE".to_owned()), Name("EMPIRE".to_owned()), Name("JEDI".to_owned())] })
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

