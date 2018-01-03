# GraphQL server framework in Rust

This framework lets you write type-safe, efficient GraphQL servers in Rust. We
make heavy use of macros to cut down on boilerplate and the trait system to
allow maximum flexibility.

This project is at 'proof of concept' stage. It can only handle minimal examples
and some key components are missing. However, I believe the results are already
promising - Rust and GraphQL are a great match!

In the future we should use Rusts emerging async IO systems to make extremely
performant servers.


## Example

Use the `schema` macro to specify the schema for your server (using IDL):

```
schema! {
    schema {
        query: Query,
    }

    type Query {
        hero(episode: Episode): Character,
        human(id : ID!): Human,
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
        appearsIn: [Episode]!,
    }

    type Human implements Character {
        id: ID!,
        name: String!,
        friends: [Character],
        appearsIn: [Episode]!,
        homePlanet: String,
    }
}
```

You can see the output for this example use of the `schema` macro at [schema.out](examples/trilogy/schema.out).

The macro generates concrete and abstract versions of each item. The library user
must specify implementations for functions (e.g., `hero` in the above schema).
You can then use the generated types - `enum`s are Rust enums, `type`s are Rust
structs, etc.:

TODO these are equivalent to resolvers in the JS frameworks

```
struct MyServer;

impl Root for MyServer {
    type Query = DbQuery;

    fn query(&self) -> QlResult<DbQuery> {
        Ok(DbQuery)
    }
}

ImplRoot!(MyServer);


struct DbQuery;

impl AbstractQuery for DbQuery {
    fn hero(&self, episode: Option<Episode>) -> QlResult<Option<Character>> {
        match episode {
            Some(Episode::JEDI) => {
                // In real life, this would query the DB or execute business logic.
                Ok(Some(Character {
                    id: Id("0".to_owned()),
                    name: "Luke".to_owned(),
                    friends: Some(vec![]),
                    appearsIn: vec![],
                }))
            }
            _ => unimplemented!(),
        }
    }

    fn human(&self, _id: Id) -> QlResult<Option<Human>> {
        ...
    }
}
```

If you don't want to use the generated representation for a certain item, you
can provide your own (perhaps using a `HashMap` of data, rather than fields).
You then implement the `abstract` view of the item (e.g., `AbstractHuman` for
`Human`) and override the relevant associated type (e.g., `type Human = MyHuman;`
in the implementations of `Root` and `AbstractQuery`, and anywhere else the type
is used):

```
struct MyHuman {
    id: usize,
    db_table: DbTablePtr,
}

ImplHuman!(MyHuman);

impl AbstractHuman for MyHuman {
    fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
        ...
    }
}
```

TODO show `main`
