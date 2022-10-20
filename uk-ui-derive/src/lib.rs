use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, Fields, FieldsNamed, Ident, VisPublic, Visibility};

#[proc_macro_derive(Editable)]
pub fn editable(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    match ast.data {
        syn::Data::Struct(struc) => impl_editable_struct(&ast.ident, struc),
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => unimplemented!(),
    }
}

fn impl_struct_named_fields<'f, 'p: 'f>(
    fields: &'f FieldsNamed,
    parent: &'p str,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'f {
    fields
        .named
        .iter()
        .filter(|f| matches!(f.vis, Visibility::Public(_)))
        .map(move |field| {
            let name = field.ident.as_ref().expect("field name should exist");
            let str_name = name.to_string();
            let id = format!("{parent}_{name}");
            quote! {
                ::uk_ui::egui::CollapsingHeader::new(#str_name).id_source(#id).show(ui, |ui| {
                    self.#name.edit_ui(ui);
                });
            }
        })
}

fn impl_editable_struct(name: &Ident, struc: DataStruct) -> TokenStream {
    let str_name = name.to_string();
    let field_impls = match struc.fields {
        Fields::Named(ref fields) => impl_struct_named_fields(fields, &str_name),
        Fields::Unnamed(_) => todo!(),
        Fields::Unit => todo!(),
    };
    quote! {
        #[automatically_derived]
        impl ::uk_ui::editor::EditableValue for #name {
            fn edit_ui(&mut self, ui: &mut ::uk_ui::egui::Ui) -> ::uk_ui::egui::Response {
                self.edit_ui_with_id(ui, #str_name)
            }

            fn edit_ui_with_id(&mut self, ui: &mut ::uk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::uk_ui::egui::Response {
                use ::uk_ui::egui;
                egui::CollapsingHeader::new(#str_name).id_source(id).show(ui, |ui| {
                    #(#field_impls)*
                }).header_response
            }
        }
    }
    .into()
}
