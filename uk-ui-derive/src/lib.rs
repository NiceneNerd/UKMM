use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, Expr, Fields, FieldsNamed, Ident, Type, VisPublic, Visibility};
use uk_ui::editor::EditableDisplay;

#[proc_macro_derive(Editable)]
pub fn editable(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    match ast.data {
        syn::Data::Struct(struc) => impl_editable_struct(&ast.ident, struc),
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => unimplemented!(),
    }
}

fn impl_struct_named_fields(
    fields: &FieldsNamed,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    fields
        .named
        .iter()
        .filter(|f| matches!(f.vis, Visibility::Public(_)))
        .map(move |field| {
            let name = field.ident.as_ref().expect("field name should exist");
            let ty = &field.ty;
            let str_name = name.to_string();
            quote! {
                let child_id = id.with(#str_name);
                match <#ty as ::uk_ui::editor::EditableValue>::DISPLAY {
                    ::uk_ui::editor::EditableDisplay::Block => {
                        ::uk_ui::egui::CollapsingHeader::new(#str_name).id_source(id.with(#str_name)).show(ui, |ui| {
                            self.#name.edit_ui_with_id(ui, child_id);
                        });
                    }
                    ::uk_ui::editor::EditableDisplay::Inline => {
                        let mut height = 0.0;
                        ::uk_ui::egui_extras::StripBuilder::new(ui)
                            .size(::uk_ui::egui_extras::Size::relative(0.2))
                            .size(::uk_ui::egui_extras::Size::remainder())
                            .horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    ui.label(#str_name);
                                });
                                strip.cell(|ui| {
                                    height = self.#name.edit_ui_with_id(ui, child_id).rect.height();
                                });
                            });
                            ui.shrink_height_to_current();
                            ui.allocate_space([0.0, height].into());
                        }
                }
            }
        })
}

fn impl_editable_struct(name: &Ident, struc: DataStruct) -> TokenStream {
    let str_name = name.to_string();
    let (field_impls, display) = match struc.fields {
        Fields::Named(ref fields) => (
            impl_struct_named_fields(fields),
            syn::parse_str::<Expr>("::uk_ui::editor::EditableDisplay::Block")
                .expect("display variant should parse"),
        ),
        Fields::Unnamed(_) => todo!(),
        Fields::Unit => todo!(),
    };
    quote! {
        #[automatically_derived]
        impl ::uk_ui::editor::EditableValue for #name {
            const DISPLAY: ::uk_ui::editor::EditableDisplay = #display;
            fn edit_ui(&mut self, ui: &mut ::uk_ui::egui::Ui) -> ::uk_ui::egui::Response {
                self.edit_ui_with_id(ui, #str_name)
            }

            fn edit_ui_with_id(&mut self, ui: &mut ::uk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::uk_ui::egui::Response {
                use ::uk_ui::egui;
                let id = egui::Id::new(id);
                egui::CollapsingHeader::new(#str_name).id_source(id).show(ui, |ui| {
                    #(#field_impls)*
                }).header_response
            }
        }
    }
    .into()
}
