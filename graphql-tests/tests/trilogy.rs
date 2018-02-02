#![feature(decl_macro)]
#![feature(proc_macro)]
#![feature(associated_type_defaults)]

extern crate graphql;
extern crate graphql_macros;

use graphql::{QlError, QlResult};
use graphql::types::{self, query, result, schema, Id, Name};
use graphql::types::schema::{Reflect, ResolveEnum, ResolveObject};
use graphql::types::query::FromValue;
use graphql::types::result::Resolve;

use graphql_macros::schema;

use std::collections::HashMap;

schema! {
    schema {
        query: Query,
    }

    type Query {
        hero(episode: Episode): Character,
        human(id : ID!): Human,
        droid(id : ID!): Droid,
    }

    enum Episode {
        NEWHOPE,
        EMPIRE,
        JEDI,
    }

    interface Character {
        id: ID!,
        name: String!,
        friends: [Character],
        appearsIn: [Episode!]!,
    }

    type Human implements Character {
        id: ID!,
        name: String!,
        friends: [Character],
        appearsIn: [Episode!]!,
        homePlanet: String,
    }

    type Droid implements Character {
        id: ID!,
        name: String!,
        friends: [Character],
        appearsIn: [Episode!]!,
        primaryFunction: String,
    }
}

struct Service {
    query: StaticQuery,
}

impl Service {
    fn new() -> Service {
        Service { query: StaticQuery::new() }
    }
}

impl Root for Service {
    type Query = StaticQuery;
    type Character = Character;
    type Human = Human;
    type Droid = Droid;
    type Episode = Episode;

    fn query(&self) -> QlResult<StaticQuery> {
        Ok(self.query.clone())
    }
}

ImplRoot!(Service);

#[derive(Clone)]
struct StaticQuery {
    luke: Human,
    vader: Human,
    han: Human,
    leia: Human,
    tarkin: Human,
    threepio: Droid,
    artoo: Droid,
}

impl StaticQuery {
    fn new() -> StaticQuery {
        let luke = Human {
            id: Id("1000".to_owned()),
            name: "Luke Skywalker".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            homePlanet: Some("Tatooine".to_owned()),
        };
        let vader = Human {
            id: Id("1001".to_owned()),
            name: "Darth Vader".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            homePlanet: Some("Tatooine".to_owned()),
        };
        let han = Human {
            id: Id("1002".to_owned()),
            name: "Han Solo".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            homePlanet: None,
        };
        let leia = Human {
            id: Id("1003".to_owned()),
            name: "Leia Organa".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            homePlanet: Some("Alderaan".to_owned()),
        };
        let tarkin = Human {
            id: Id("1004".to_owned()),
            name: "Wilhuff Tarkin".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE],
            homePlanet: None,
        };
        let threepio = Droid {
            id: Id("2000".to_owned()),
            name: "C-3PO".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            primaryFunction: Some("Protocol".to_owned()),
        };
        let artoo = Droid {
            id: Id("2001".to_owned()),
            name: "R2-D2".to_owned(),
            friends: None,
            appearsIn: vec![Episode::NEWHOPE, Episode::EMPIRE, Episode::JEDI],
            primaryFunction: Some("Astromech".to_owned()),
        };
        StaticQuery {
            luke,
            vader,
            han,
            leia,
            tarkin,
            threepio,
            artoo,
        }
    }
}

impl AbstractQuery for StaticQuery {
    type Character = Character;
    type Human = Human;
    type Droid = Droid;
    type Episode = Episode;

    fn hero(&self, episode: Option<Episode>) -> QlResult<Option<Character>> {
        match episode {
            Some(Episode::EMPIRE) => return Ok(Some(self.luke.to_Character()?)),
            _ => Ok(Some(self.artoo.to_Character()?)),
        }
    }

    fn human(&self, id: Id) -> QlResult<Option<Human>> {
        match id.0.as_str() {
            "1000" => Ok(Some(self.luke.clone())),
            "1001" => Ok(Some(self.vader.clone())),
            "1002" => Ok(Some(self.han.clone())),
            "1003" => Ok(Some(self.leia.clone())),
            "1004" => Ok(Some(self.tarkin.clone())),
            _ => Ok(None),
        }
    }

    fn droid(&self, _id: Id) -> QlResult<Option<Droid>> {
        Ok(None)
    }
}

ImplQuery!(StaticQuery);

fn query_string(query: &str, expected: &str) {
    let result = format!("{}", graphql::handle_query(query, HashMap::new(), Service::new()).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn find_hero() {
    let q = /*query HeroNameQuery*/ r#"{
      hero {
        name
      }
    }"#;
    query_string(q, r#"{data:{hero:{name:"R2-D2"}}}"#);
}

#[test]
fn find_hero_id_and_friends() {
    let q = /*query HeroNameAndFriendsQuery*/ r#"{
      hero {
        id
        name
        #friends {
          #name
        #}
      }
    }"#;
    query_string(q, r#"{data:{hero:{id:2001,name:"R2-D2"}}}"#);
}

#[test]
fn find_luke() {
    let q = /*query FetchLukeQuery*/ r#"{
      human(id: 1000) {
        name
      }
    }"#;
    query_string(q, r#"{data:{human:{name:"Luke Skywalker"}}}"#);
}
