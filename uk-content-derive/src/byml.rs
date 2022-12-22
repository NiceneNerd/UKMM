use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

use super::get_name;

fn field_from_byml(
    ty: &Type,
    field_var_name: &Ident,
    field_src_name: String,
    err_msg: String,
) -> TokenStream {
    let Type::Path(ref ty_path) = ty else {
        panic!("invalid field type")
    };
    if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Byml")
    {
        quote! {
            let #field_var_name =  hash
                .get(#field_src_name)
                .ok_or(UKError::MissingBymlKey(#err_msg))?
                .clone();
        }
    } else if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            let #field_var_name: #ty = hash
                .get(#field_src_name)
                .map(|val| val.clone().try_into())
                .transpose()
                .map_err(|by| crate::UKError::InvalidByml(#field_src_name.into(), by))?;
        }
    } else {
        quote! {
            let #field_var_name: #ty = hash
                .get(#field_src_name)
                .cloned()
                .ok_or(UKError::MissingBymlKey(#err_msg))?
                .try_into()
                .map_err(|by| crate::UKError::InvalidByml(#field_src_name.into(), by))?;
        }
    }
}

pub fn impl_from_byml(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    let field_tries = fields.named.iter().map(|field| {
        let field_var_name = field.ident.as_ref().expect("no ident for field");
        let field_src_name = get_name(field);
        let err_msg = format!("{} missing {}", name, field_src_name);
        field_from_byml(&field.ty, field_var_name, field_src_name, err_msg)
    });
    let field_assigns = fields.named.iter().map(|field| {
        let name = field.ident.as_ref().expect("no ident for field");
        quote!(#name, )
    });
    quote! {
        #[automatically_derived]
        impl TryFrom<&::roead::byml::Byml> for #name {
            type Error = crate::UKError;
            fn try_from(byml: &Byml) -> ::std::result::Result<#name, Self::Error> {
                let hash = byml.as_hash()?;
                #(#field_tries)*
                Ok(Self {
                    #(#field_assigns)*
                })
            }
        }
    }
}

fn field_to_byml(ty: &Type, field_var_name: &Ident, field_src_name: String) -> TokenStream {
    let Type::Path(ref ty_path) = ty else {
        panic!("invalid field type")
    };
    if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            if let Some(#field_var_name) = val.#field_var_name {
                hash.insert(#field_src_name.into(), #field_var_name.into());
            }
        }
    } else {
        quote! {
            hash.insert(#field_src_name.into(), val.#field_var_name.into());
        }
    }
}

pub fn impl_into_byml(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    let fields = fields.named.iter().map(|field| {
        let field_var_name = field.ident.as_ref().expect("no ident for field");
        let field_src_name = get_name(field);
        field_to_byml(&field.ty, field_var_name, field_src_name)
    });
    quote! {
        #[automatically_derived]
        impl From<#name> for ::roead::byml::Byml {
            fn from(val: #name) -> Self {
                let mut hash = ::roead::byml::Hash::default();
                #(#fields)*
                Byml::Hash(hash)
            }
        }
    }
}
