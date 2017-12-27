#![feature(decl_macro)]
#![feature(proc_macro)]

extern crate graphql;
extern crate proc_macro;
#[cfg(feature = "rustfmt")]
extern crate rustfmt_nightly as rustfmt;

use proc_macro::{TokenStream, TokenTree, TokenNode, Term, Span, quote};
use graphql::{parse_schema, QlResult};
use graphql::types::Name;

mod ir;

#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    // TODO to_string loses whitespace from the input, so we must use commas
    // Worse, Rust seems to insert newlines randomly into the string, so we must
    // replace them.
    let schema = parse_schema(&input.to_string().replace('\n', " ")).expect("Error parsing schema");
    let lowered = ir::lower_schema(&schema);

    let mut result = vec![];
    let schema = emit_schema(&lowered).expect("Problem emitting schema");
    result.push(schema);

    let result: TokenStream = result.into_iter().collect();

    #[cfg(feature = "rustfmt")]
    {
        use rustfmt::{format_snippet, Config};
        use std::default::Default;

        let formatted = format_snippet(&result.to_string(), &Config::default()).expect("Could not format output of `schema`");
        println!("{}",formatted);
    }
    // TODO workaround hygiene bugs
    result.to_string().parse().unwrap()
}

fn emit_schema(schema: &ir::Schema) -> QlResult<TokenStream> {
    let mut result = vec![];
    for item in schema.items.values() {
        if item.name() == &Name("schema".to_owned()) {
            result.push(emit_server(schema)?);
        } else {
            result.push(emit_item(item, schema)?);
        }
    }
    Ok(result.into_iter().collect())
}

fn emit_item(item: &ir::Item, schema: &ir::Schema) -> QlResult<TokenStream> {
    match *item {
        ir::Item::Object(ref o) => emit_object(o, schema),
        ir::Item::Enum(ref e) => emit_enum(e, schema),
        _ => unreachable!(),
    }
}

struct TokenBuilder {
    result: Vec<TokenStream>,
}

impl TokenBuilder {
    fn new() -> TokenBuilder {
        TokenBuilder {
            result: vec![],
        }
    }

    fn push(&mut self, ts: TokenStream) {
        self.result.push(ts);
    }

    fn finish(self) -> TokenStream {
        self.result.into_iter().collect()
    }
}

fn emit_enum(item: &ir::Enum, _schema: &ir::Schema) -> QlResult<TokenStream> {
    let mut builder = TokenBuilder::new();
    // trait AbstractFoo
    builder.push(item.emit_abstract_trait());
    // macro ImplFoo
    //     impl Reflect for $concrete
    //     impl ResolveEnum for $concrete
    //     impl AbstractFoo for $concrete
    builder.push(item.emit_impl_macro());
    // enum Foo
    builder.push(item.emit_concrete_enum());
    // ImplFoo!(Foo)
    builder.push(item.emit_impl_macro_use());
    // impl FromValue for Foo
    builder.push(item.emit_impl_from_value());
    // impl Resolve for Foo
    builder.push(item.emit_impl_resolve());

    Ok(builder.finish())
}

fn emit_object(item: &ir::Object, schema: &ir::Schema) -> QlResult<TokenStream> {
    let mut builder = TokenBuilder::new();
    // pub trait AbstractCharacter: ResolveObject
    builder.push(item.emit_abstract_trait());
    // pub macro ImplCharacter
    //     impl schema::Reflect for $concrete
    //     impl Resolve for $concrete
    builder.push(item.emit_impl_macro());
    if item.has_fields {
        // pub struct Foo
        builder.push(item.emit_concrete_struct());
        // ImplFoo!(Foo);
        builder.push(item.emit_impl_macro_use());

        if !item.has_fns {
            // impl ResolveObject for Foo
            builder.push(item.emit_resolve_object_impl());
            // impl AbstractFoo for Foo
            builder.push(item.emit_abstract_impl(schema));
        }
    }

    Ok(builder.finish())
}

fn ident(s: &str) -> TokenTree {
    TokenTree {
        span: Span::call_site(),
        kind: TokenNode::Term(Term::intern(s)),
    }
}

// TODO can we share code between Enum and Object?
impl ir::Enum {
    fn name_t(&self) -> TokenTree {
        ident(&self.name.0)
    }

    fn name_str(&self) -> TokenTree {
        ident(&format!("\"{}\"", self.name.0))
    }

    fn abs_name_t(&self) -> TokenTree {
        ident(&format!("Abstract{}", self.name.0))
    }

    fn impl_name_t(&self) -> TokenTree {
        ident(&format!("Impl{}", self.name.0))
    }

    fn emit_abstract_trait(&self) -> TokenStream {
        let abs_name_t = self.abs_name_t();
        // TODO hygiene problem? Can't find graphql
        quote!(pub trait $abs_name_t: ::graphql::types::schema::ResolveEnum {})
    }

    fn emit_impl_macro(&self) -> TokenStream {
        let impl_name_t = self.impl_name_t();
        let abs_name_t = self.abs_name_t();
        let name_str = self.name_str();
        let variants: TokenStream = self.variants.iter().map(|v| v.emit_string()).collect();

        quote!(
            pub macro $impl_name_t($$concrete: ident) {
                impl schema::Reflect for $$concrete {
                    const NAME: &'static str = $name_str;

                    fn schema() -> schema::Item {
                        schema::Item::Enum(schema::Enum { variants: vec![$variants] })
                    }
                }
                impl ResolveEnum for $$concrete {}
                impl $abs_name_t for $$concrete {}
            }
        )
    }

    fn emit_concrete_enum(&self) -> TokenStream {
        let name_t = self.name_t();
        let variants: TokenStream = self.variants.iter().map(|v| v.emit_decl()).collect();

        quote!(
            #[allow(non_snake_case)]
            #[derive(Clone, Debug)]
            pub enum $name_t {
                $variants
            }
        )
    }

    fn emit_impl_macro_use(&self) -> TokenStream {
        let name_t = self.name_t();
        let impl_name_t = self.impl_name_t();

        quote!($impl_name_t!($name_t);)
    }

    fn emit_impl_from_value(&self) -> TokenStream {
        let name_t = self.name_t();
        let arms = self.variants.iter().map(|v| v.emit_from_str_arm(name_t.clone()));
        let expect_str = ident(&format!("\"Option<{}>\"", self.name.0));
        let arms: TokenStream = arms
            .chain(
                Some(quote!(
                    _ => return Err(QlError::LoweringError(format!("{:?}", value), $expect_str.to_owned())),
                )).into_iter()
            ).collect();

        quote!(
            impl FromValue for $name_t {
                fn from(value: &query::Value) -> QlResult<$name_t> {
                    Ok(match &*<String as FromValue>::from(value)? {
                        $arms
                    })
                }
            }
        )
    }

    fn emit_impl_resolve(&self) -> TokenStream {
        let name_t = self.name_t();
        let variants: TokenStream = self.variants.iter().map(|v| v.emit_resolve_arm(name_t.clone())).collect();

        quote!(
            impl Resolve for $name_t {
                fn resolve(&self, _fields: &[query::Field]) -> QlResult<result::Value> {
                    Ok(match *self {
                        $variants
                    })
                }
            }
        )
    }

    fn emit_assoc_ty(&self) -> TokenStream {
        let name_t = self.name_t();
        let abs_name_t = self.abs_name_t();
        quote!(type $name_t: $abs_name_t + FromValue = $name_t;)
    }

    fn emit_schema(&self) -> TokenStream {
        let name_t = self.name_t();
        quote!(Name(<Self as Root>::$name_t::NAME.to_owned()), <Self as Root>::$name_t::schema())
    }
}

impl ir::Object {
    fn name_t(&self) -> TokenTree {
        ident(&self.name.0)
    }

    fn name_str(&self) -> TokenTree {
        ident(&format!("\"{}\"", self.name.0))
    }

    fn abs_name_t(&self) -> TokenTree {
        ident(&format!("Abstract{}", self.name.0))
    }

    fn impl_name_t(&self) -> TokenTree {
        ident(&format!("Impl{}", self.name.0))
    }

    fn emit_concrete_struct(&self) -> TokenStream {
        let name_t = self.name_t();
        let fields: TokenStream = self.fields.iter().map(|f| f.emit_struct_field()).collect();

        quote!(
            #[allow(non_snake_case)]
            #[derive(Clone, Debug)]
            pub struct $name_t {
                $fields
            }
        )
    }

    fn emit_abstract_trait(&self) -> TokenStream {
        let abs_name_t = self.abs_name_t();

        // Assoc types - any type used in a function or `implements` clause must
        // have an assoc type. Since fields aren't represented abstractly, they
        // don't need assoc types.
        let impl_types: TokenStream = self.abs_names.iter().map(|n| {
            let i_name = ident(&n.0);
            let i_abs_name = ident(&format!("Abstract{}", n.0));
            quote!(type $i_name: $i_abs_name = $i_name;)
        }).collect();

        // Conversion functions for converting this object to
        // an object in its implements list.
        let conversion_fns: TokenStream = self.implements.iter().map(|n| {
            let i_name = ident(&n.0);
            let fn_name = ident(&format!("to_{}", n.0));
            quote!(
                #[allow(non_snake_case)]
                fn $fn_name(&self) -> QlResult<Self::$i_name>;
            )
        }).collect();

        let fns: TokenStream = self.fields.iter().map(|f| f.emit_fn_sig(&format!("Abstract{}", self.name.0))).collect();

        quote!(
            pub trait $abs_name_t: ::graphql::types::schema::ResolveObject {
                $impl_types
                $conversion_fns
                $fns
            }
        )
    }

    fn emit_abstract_impl(&self, schema: &ir::Schema) -> TokenStream {
        let name_t = self.name_t();
        let abs_name_t = self.abs_name_t();

        // Assoc types and conversion functions for converting this object to
        // an object in its implements list.
        let impl_types: TokenStream = self.abs_names.iter().map(|n| {
            let i_name = ident(&n.0);
            quote!(type $i_name = $i_name;)
        }).collect();
        let conversion_fns: TokenStream = self.implements.iter().map(|n| {
            let i_name = ident(&n.0);
            let fn_name = ident(&format!("to_{}", n.0));
            let super_ty = schema.items[&n].assert_object();
            let fields: TokenStream = super_ty.fields.iter().map(|f| {
                let f_name = ident(&f.name.0);
                quote!($f_name: self.$f_name.clone(),)
            }).collect();
            quote!(
                fn $fn_name(&self) -> QlResult<Self::$i_name> {
                    Ok($i_name {
                        $fields
                    })
                }
            )
        }).collect();

        quote!(
            impl $abs_name_t for $name_t {
                $impl_types
                $conversion_fns
            }
        )
    }

    fn emit_impl_macro(&self) -> TokenStream {
        let impl_name_t = self.impl_name_t();
        let name_str = self.name_str();
        let field_schemas: TokenStream = self.fields.iter().map(|f| f.emit_schema()).collect();
        let resolve_fields: TokenStream = self.fields.iter().map(|f| f.emit_resolve_arm(&format!("Abstract{}", self.name.0))).collect();

        let impl_resolve_object = if !self.has_fields {
            // TODO share code with emit_resolve_object_impl
            let fields: TokenStream = self.fields.iter().map(|f| f.emit_dispatch_resolve_arm()).collect();
            quote!(
                impl ResolveObject for $$concrete {
                    fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
                        match &*field.name.0 {
                            $fields
                            _ => return Err(QlError::ResolveError("field", field.name.to_string(), None)),
                        }
                    }
                }
            )
        } else {
            quote!()
        };

        // Note: if the object has no fields at all (i.e., no fields, function-like or not),
        // then this macro could implement `AbstractFoo` (see `emit_abstract_impl`).
        // However, this is non-trivial and I imagine it doesn't happen often in
        // practice.

        quote!(
            pub macro $impl_name_t($$concrete: ident) {
                impl schema::Reflect for $$concrete {
                    const NAME: &'static str = $name_str;

                    fn schema() -> schema::Item {
                        let fields = vec![$field_schemas];
                        schema::Item::Object(schema::Object { implements: vec![], fields })
                    }
                }

                impl Resolve for $$concrete {
                    fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                        let mut result = vec![];
                        for field in fields {
                            match &*field.name.0 {
                                $resolve_fields
                                n => return Err(QlError::ExecutionError(format!("Missing field executor in {}: {}", $name_str, n))),
                            }
                        }
                        Ok(result::Value::Object(result::Object { fields: result } ))
                    }
                }

                $impl_resolve_object
            }
        )
    }

    fn emit_impl_macro_use(&self) -> TokenStream {
        let name_t = self.name_t();
        let impl_name_t = self.impl_name_t();

        quote!($impl_name_t!($name_t);)
    }

    fn emit_resolve_object_impl(&self) -> TokenStream {
        let name_t = self.name_t();
        let fields: TokenStream = self.fields.iter().map(|f| f.emit_dispatch_resolve_arm()).collect();

        quote!(
            impl ResolveObject for $name_t {
                fn resolve_field(&self, field: &query::Field) -> QlResult<result::Value> {
                    match &*field.name.0 {
                        $fields
                        _ => return Err(QlError::ResolveError("field", field.name.to_string(), None)),
                    }
                }
            }
        )
    }

    fn emit_assoc_ty(&self) -> TokenStream {
        let name_t = self.name_t();
        let abs_name_t = self.abs_name_t();

        if self.has_fields {
            quote!(type $name_t: $abs_name_t = $name_t;)
        } else {
            quote!(type $name_t: $abs_name_t;)
        }
    }

    fn emit_schema(&self) -> TokenStream {
        let name_t = self.name_t();
        quote!(Name(<Self as Root>::$name_t::NAME.to_owned()), <Self as Root>::$name_t::schema())
    }
}


fn emit_server(schema: &ir::Schema) -> QlResult<TokenStream> {
    let mut builder = TokenBuilder::new();
    // pub trait Root: query::Root
    builder.push(schema.emit_root_trait());
    // pub macro ImplRoot($concrete: ident)
    //     impl query::Root for $concrete
    //     impl Resolve for $concrete
    builder.push(schema.emit_impl_macro());

    Ok(builder.finish())
}

impl ir::Schema {
    fn emit_root_trait(&self) -> TokenStream {
        let types: TokenStream = self.items.values().map(|i| i.emit_assoc_ty()).collect();
        quote!(
            pub trait Root: query::Root {
                $types

                fn query(&self) -> QlResult<Self::Query>;
                // FIXME Mutations
            }
        )
    }

    fn emit_impl_macro(&self) -> TokenStream {
        let schema_items: TokenStream = self.items.values().map(|i| {
            let sch = i.emit_schema();
            quote!(schema.items.insert($sch);)
        }).collect();

        quote!(
            pub macro ImplRoot($$concrete: ident) {
                impl query::Root for $$concrete {
                    fn schema() -> schema::Schema {
                        let mut schema = schema::Schema::new();
                        $schema_items
                        assert!(schema.validate().is_ok());
                        schema
                    }
                }

                impl Resolve for $$concrete {
                    fn resolve(&self, fields: &[query::Field]) -> QlResult<result::Value> {
                        let mut results = vec![];
                        for field in fields {
                            match &*field.name.0 {
                                "query" => {
                                    assert_eq!(field.args.len(), 0);
                                    let result = self.query()?;
                                    let result = result.resolve(&field.fields)?;

                                    // This is a special case where the result doesn't match the query
                                    results.push((types::Name("data".to_owned()), result));
                                }
                                // FIXME mutations
                                n => return Err(QlError::ExecutionError(format!("Missing field executor in Root: {}", n))),
                            }
                        }
                        Ok(result::Value::Object(result::Object { fields: results } ))
                    }
                }
            }
        )
    }

    fn emit_schema() -> TokenStream {
        quote!(Name(schema::SCHEMA_NAME.to_owned()), schema::schema_type())
    }
}

impl ir::Variant {
    fn emit_resolve_arm(&self, enum_name: TokenTree) -> TokenStream {
        let id = ident(&(self.0).0);
        let id_str = ident(&format!("\"{}\"", (self.0).0));
        quote!($enum_name::$id => result::Value::String($id_str.to_owned()),)
    }

    fn emit_from_str_arm(&self, enum_name: TokenTree) -> TokenStream {
        let id = ident(&(self.0).0);
        let id_str = ident(&format!("\"{}\"", (self.0).0));
        quote!($id_str => $enum_name::$id,)
    }

    fn emit_decl(&self) -> TokenStream {
        let id = ident(&(self.0).0);
        quote!($id,)
    }

    fn emit_string(&self) -> TokenStream {
        let id = ident(&format!("\"{}\"", (self.0).0));
        quote!(Name($id.to_owned()),)
    }
}

impl ir::Field {
    fn emit_dispatch_resolve_arm(&self) -> TokenStream {
        let name = ident(&self.name.0);
        let name_str = ident(&format!("\"{}\"", self.name.0));
        if self.args.is_empty() {
            quote!($name_str => self.$name.resolve(&field.fields),)
        } else {
            quote!($name_str => panic!("trying to dispatch function as field: {}", $name_str),)
        }
    }

    fn emit_resolve_arm(&self, abs_self_type: &str) -> TokenStream {
        let name_str = ident(&format!("\"{}\"", self.name.0));
        if self.args.is_empty() {
            quote!($name_str => result.push((types::Name($name_str.to_owned()), self.resolve_field(field)?)),)
        } else {
            let name = ident(&self.name.0);

            let process_args: TokenStream = self.args.iter().enumerate().map(|(i, a)| {
                let arg_n = ident(&format!("arg{}", i));
                let arg_ty = a.1.emit_abs_rust_type(abs_self_type);
                let name_str = ident(&format!("\"{}\"", a.0));
                let none_expr = if a.1.nullable {
                    quote!(None)
                } else {
                    quote!(panic!("Required non-null argument not supplied: {}", $name_str))
                };
                quote!(
                    let $arg_n: $arg_ty = match field.find_arg(&Name($name_str.to_owned())) {
                        Some(val) => FromValue::from(val)?,
                        None => $none_expr,
                    };
                )
            }).collect();

            let arg_list: TokenStream = (0..self.args.len()).map(|i| {
                let arg_n = ident(&format!("arg{}", i));
                quote!($arg_n,)
            }).collect();

            quote!(
                $name_str => {
                    $process_args

                    let sub_result = self.$name($arg_list)?;
                    let sub_result = sub_result.resolve(&field.fields)?;

                    result.push((types::Name($name_str.to_owned()), sub_result))
                }
            )
        }
    }

    fn emit_struct_field(&self) -> TokenStream {
        let name = ident(&self.name.0);
        let ty = self.ty.emit_rust_type();
        quote!(pub $name: $ty,)
    }

    fn emit_schema(&self) -> TokenStream {
        let name_str = ident(&format!("\"{}\"", self.name.0));
        let ty = self.ty.emit_type_schema();

        if self.args.is_empty() {
            quote!(schema::Field::field(Name($name_str.to_owned()), $ty),)
        } else {
            let args: TokenStream = self.args.iter().map(|a| {
                let name_str = ident(&format!("\"{}\"", (a.0).0));
                let ty = (a.1).emit_type_schema();
                quote!((Name($name_str.to_owned()), $ty))
            }).collect();
            quote!(
                schema::Field::fun(
                    Name($name_str.to_owned()),
                    vec![$args],
                    $ty,
                ),
            )
        }
    }

    fn emit_fn_sig(&self, abs_self_type: &str) -> TokenStream {
        if self.args.is_empty() {
            return quote!();
        }

        let name = ident(&self.name.0);
        let ty = self.ty.emit_abs_rust_type(abs_self_type);
        let args: TokenStream = self.args.iter().map(|a| {
            let name = ident(&(a.0).0);
            let ty = (a.1).emit_abs_rust_type(abs_self_type);
            quote!($name: $ty,)
        }).collect();

        quote!(fn $name(&self, $args) -> QlResult<$ty>;)
    }
}

impl ir::Type {
    fn emit_type_schema(&self) -> TokenStream {
        let nullable = ident(&self.nullable.to_string());
        let kind = match self.kind {
            ir::TypeKind::String => quote!(schema::TypeKind::String),
            ir::TypeKind::Id => quote!(schema::TypeKind::Id),
            ir::TypeKind::Name(ref n) => {
                let n = ident(&format!("\"{}\"", n.0));
                quote!(schema::TypeKind::Name(Name($n.to_owned())))
            }
            ir::TypeKind::Array(ref inner) => {
                let inner = inner.emit_type_schema();
                quote!(schema::TypeKind::Array(Box::new($inner)))
            }
        };
        quote!(schema::Type {
            kind: $kind,
            nullable: $nullable,
        })
    }

    fn emit_rust_type(&self) -> TokenStream {
        let result = match self.kind {
            ir::TypeKind::String => quote!(String),
            ir::TypeKind::Id => quote!(Id),
            ir::TypeKind::Name(ref n) => {
                let n = ident(&n.0);
                n.into()
            }
            ir::TypeKind::Array(ref inner) => {
                let inner = inner.emit_rust_type();
                quote!(Vec<$inner>)
            }
        };

        if self.nullable {
            quote!(Option<$result>)
        } else {
            result
        }
    }

    // This is like `emit_rust_type`, except we use `Self::Foo` rather than `Foo`
    // assuming that the type is kept abstract in the current context.
    fn emit_abs_rust_type(&self, abs_self_type: &str) -> TokenStream {
        let result = match self.kind {
            ir::TypeKind::String => quote!(String),
            ir::TypeKind::Id => quote!(Id),
            ir::TypeKind::Name(ref n) => {
                let n = ident(&format!("<Self as {}>::{}", abs_self_type, n.0));
                n.into()
            }
            ir::TypeKind::Array(ref inner) => {
                let inner = inner.emit_abs_rust_type(abs_self_type);
                quote!(Vec<$inner>)
            }
        };

        if self.nullable {
            quote!(Option<$result>)
        } else {
            result
        }
    }
}
