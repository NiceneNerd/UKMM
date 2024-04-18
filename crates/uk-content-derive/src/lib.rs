mod aamp;
mod byml;
use proc_macro::TokenStream;
use quote::quote;
use syn::*;

fn get_name(field: &Field) -> String {
    if let Some(Meta::NameValue(MetaNameValue {
        path: _,
        eq_token: _,
        lit: Lit::Str(lit),
    })) = field
        .attrs
        .iter()
        .find(|at| at.path.is_ident("name"))
        .and_then(|at| at.parse_meta().ok())
    {
        lit.value()
    } else {
        field
            .ident
            .as_ref()
            .expect("no ident for field")
            .to_string()
    }
}

#[proc_macro_derive(BymlData, attributes(name))]
pub fn byml_data(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("bad derive input");
    let Data::Struct(struc) = ast.data else {
        panic!("Only structs are supported");
    };
    let Fields::Named(fields) = struc.fields else {
        panic!("Only structs with named fields are supported");
    };
    let from = byml::impl_from_byml(&ast.ident, &fields);
    let into = byml::impl_into_byml(&ast.ident, &fields);
    quote! {
        #from

        #into
    }
    .into()
}

#[proc_macro_derive(ParamData, attributes(name))]
pub fn param_data(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("bad derive input");
    let Data::Struct(struc) = ast.data else {
        panic!("Only structs are supported");
    };
    let Fields::Named(fields) = struc.fields else {
        panic!("Only structs with named fields are supported");
    };
    let from = aamp::impl_from_params(&ast.ident, &fields);
    let into = aamp::impl_into_params(&ast.ident, &fields);
    quote! {
        #from

        #into
    }
    .into()
}
