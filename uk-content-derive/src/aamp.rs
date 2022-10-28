use super::get_name;
use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

fn field_from_param(
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
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            let #field_var_name: #ty = hash
                .get(#field_src_name)
                .map(|val| val.clone().try_into())
                .transpose()
                .map_err(|param| crate::UKError::InvalidParameter(#field_src_name.into(), param))?;
        }
    } else {
        quote! {
            let #field_var_name: #ty = hash
                .get(#field_src_name)
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
    if ty_path
        .path
        .segments
        .iter()
        .any(|s| s.ident.to_string().as_str() == "Option")
    {
        quote! {
            if let Some(#field_var_name) = val.#field_var_name {
                obj.insert(#field_src_name, #field_var_name.into());
            }
        }
    } else {
        quote! {
            obj.insert(#field_src_name, val.#field_var_name.into());
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
