#![feature(let_else)]
use proc_macro::TokenStream;
use quote::quote;
use syn::*;

#[proc_macro_derive(BymlData, attributes(data))]
pub fn byml_data(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("bad derive input");
    let Data::Struct(struc) = ast.data else {
        panic!("Only structs are supported");
    };
    Default::default()
}

fn get_name(field: &Field) -> String {
    field
        .attrs
        .iter()
        .find(|at| at.path.is_ident("name"))
        .and_then(|at| at.parse_args::<LitStr>().ok())
        .map(|st| st.value())
        .unwrap_or_else(|| field.ident.expect("no ident for field").to_string())
}

fn impl_from_byml(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    let field_tries = fields.named.iter().map(|field| {
        let field_var_name = field.ident.expect("no ident for field");
        let field_src_name = get_name(field);
        let err_msg = format!("{} missing {}", name, field_src_name);
        let Type::Path(ty) = field.ty else {
            panic!("invalid field type")
        };
        quote! {
            let #field_var_name: #ty = byml.get(#field_src_name).ok_or(UKError::MissingBymlKey(#err_msg))?.try_into()?;
        }
    });
    let field_assigns = fields.named.iter().map(|field| {
        let name = field.ident.expect("no ident for field");
        quote!(#name, )
    });
    quote! {
        impl TryFrom<&::roead::byml::Byml> for #name {
            type Error = ::uk_content::UKError;
            fn try_from(byml: &Byml) -> ::std::result::Result<#name, Self::Error> {
                #(#field_tries)*
                Ok(Self {
                    #(#field_assigns)*
                })
            }
        }
    }
    .into()
}
