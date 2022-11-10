use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

use super::get_name;

const fn hash_name(name: &str) -> u32 {
    let mut crc = 0xFFFFFFFF;
    let mut i = 0;
    while i < name.len() {
        crc ^= name.as_bytes()[i] as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        i += 1;
    }
    !crc
}

fn field_from_param(
    ty: &Type,
    field_var_name: &Ident,
    field_src_name: String,
    err_msg: String,
) -> TokenStream {
    let Type::Path(ref ty_path) = ty else {
        panic!("invalid field type")
    };
    let hash = hash_name(&field_src_name);
    if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            /// #hash = #field_src_name
            let #field_var_name: #ty = obj
                .get(#hash)
                .map(|val| val.clone().try_into())
                .transpose()
                .map_err(|param| crate::UKError::InvalidParameter(#field_src_name.into(), param))?;
        }
    } else {
        quote! {
            /// #hash = #field_src_name
            let #field_var_name: #ty = obj
                .get(#hash)
                .cloned()
                .ok_or(UKError::MissingAampKey(#err_msg))?
                .try_into()
                .map_err(|param| crate::UKError::InvalidParameter(#field_src_name.into(), param))?;
        }
    }
}

pub fn impl_from_params(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    let field_tries = fields.named.iter().map(|field| {
        let field_var_name = field.ident.as_ref().expect("no ident for field");
        let field_src_name = get_name(field);
        let err_msg = format!("{} missing {}", name, field_src_name);
        field_from_param(&field.ty, field_var_name, field_src_name, err_msg)
    });
    let field_assigns = fields.named.iter().map(|field| {
        let name = field.ident.as_ref().expect("no ident for field");
        quote!(#name, )
    });
    quote! {
        #[automatically_derived]
        impl TryFrom<&::roead::aamp::ParameterObject> for #name {
            type Error = crate::UKError;
            fn try_from(obj: &::roead::aamp::ParameterObject) -> ::std::result::Result<#name, Self::Error> {
                #(#field_tries)*
                Ok(Self {
                    #(#field_assigns)*
                })
            }
        }
    }
}

fn field_to_param(ty: &Type, field_var_name: &Ident, field_src_name: String) -> TokenStream {
    let Type::Path(ref ty_path) = ty else {
        panic!("invalid field type")
    };
    let hash = hash_name(&field_src_name);
    if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            if let Some(#field_var_name) = val.#field_var_name {
                /// #hash = #field_src_name
                obj.insert(#hash, #field_var_name.into());
            }
        }
    } else {
        quote! {
            /// #hash = #field_src_name
            obj.insert(#hash, val.#field_var_name.into());
        }
    }
}

pub fn impl_into_params(name: &Ident, fields: &FieldsNamed) -> TokenStream {
    let fields = fields.named.iter().map(|field| {
        let field_var_name = field.ident.as_ref().expect("no ident for field");
        let field_src_name = get_name(field);
        field_to_param(&field.ty, field_var_name, field_src_name)
    });
    quote! {
        #[automatically_derived]
        impl From<#name> for ::roead::aamp::ParameterObject {
            fn from(val: #name) -> Self {
                let mut obj = ::roead::aamp::ParameterObject::default();
                #(#fields)*
                obj
            }
        }
    }
}
