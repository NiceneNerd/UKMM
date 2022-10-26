#![feature(let_else)]
use proc_macro::TokenStream;
use quote::quote;
use syn::*;

#[proc_macro_derive(BymlData, attributes(name))]
pub fn byml_data(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("bad derive input");
    let Data::Struct(struc) = ast.data else {
        panic!("Only structs are supported");
    };
    Default::default()
}

fn impl_from_byml(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    todo!()
}
