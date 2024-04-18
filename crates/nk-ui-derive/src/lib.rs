use proc_macro::TokenStream;
use quote::{__private::ext::RepToTokensExt, quote, ToTokens};
use syn::{
    token::Colon2, DataEnum, DataStruct, Expr, Fields, FieldsNamed, FieldsUnnamed, Ident, Type,
    Visibility,
};

#[proc_macro_derive(Editable)]
pub fn editable(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    match ast.data {
        syn::Data::Struct(struc) => impl_editable_struct(&ast.ident, struc),
        syn::Data::Enum(enu) => impl_editable_enum(&ast.ident, enu),
        syn::Data::Union(_) => unimplemented!(),
    }
}

fn get_display_type(ty: &Type) -> Expr {
    let mut ty = ty.clone();
    if let Type::Path(ref mut path) = ty {
        if let syn::PathArguments::AngleBracketed(ref mut args) =
            path.path.segments.first_mut().unwrap().arguments
        {
            args.colon2_token = Some(Colon2::default());
        }
        return syn::parse_str(&format!("{}::DISPLAY", ty.into_token_stream())).unwrap();
    }
    todo!()
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
                match <#ty as ::nk_ui::editor::EditableValue>::DISPLAY {
                    ::nk_ui::editor::EditableDisplay::Block => {
                        egui::CollapsingHeader::new(#str_name).id_source(id.with(#str_name)).show(ui, |ui| {
                            changed |= self.#name.edit_ui_with_id(ui, child_id).changed();
                        });
                    }
                    ::nk_ui::editor::EditableDisplay::Inline => {
                        ui.columns(2, |uis| {
                            uis[0].label(#str_name);
                            let res = self.#name.edit_ui_with_id(&mut uis[1], child_id);
                            changed |= res.changed();
                        });
                    }
                }
            }
        })
}

fn impl_struct_unnamed_fields(name: &Ident, fields: &FieldsUnnamed) -> TokenStream {
    let field_count = fields.unnamed.len();
    let str_name = name.to_string();
    if field_count == 1 {
        let field = fields
            .unnamed
            .next()
            .and_then(|n| n.iter().next())
            .expect("newtype struct should have one field");
        let display = get_display_type(&field.ty);
        quote! {
            #[automatically_derived]
            impl ::nk_ui::editor::EditableValue for #name {
                const DISPLAY: ::nk_ui::editor::EditableDisplay = #display;
                fn edit_ui(&mut self, ui: &mut ::nk_ui::egui::Ui) -> ::nk_ui::egui::Response {
                    self.edit_ui_with_id(ui, #str_name)
                }
                fn edit_ui_with_id(&mut self, ui: &mut ::nk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::nk_ui::egui::Response {
                    use ::nk_ui::egui;
                    let id = egui::Id::new(id).with(#str_name);
                    let mut changed = false;
                    let mut res = egui::CollapsingHeader::new(#str_name).id_source(id).show(ui, |ui| {
                        changed |= self.0.edit_ui_with_id(ui, id.with("inner")).changed();
                    }).header_response;
                    if changed {
                        res.mark_changed();
                    }
                    res
                }
            }
        }.into()
    } else {
        let field_impls = fields.unnamed.iter().enumerate().map(|(i, _)| {
            quote! {
                changed |= self.#i.edit_ui_with_id(id.with(#i)).changed();
            }
        });
        quote! {
            #[automatically_derived]
            impl ::nk_ui::editor::EditableValue for #name {
                const DISPLAY: ::nk_ui::editor::EditableDisplay = ::nk_ui::editor::EditableDisplay::Block;
                fn edit_ui(&mut self, ui: &mut ::nk_ui::egui::Ui) -> ::nk_ui::egui::Response {
                    use ::nk_ui::editor::EditableValue;
                    self.edit_ui_with_id(ui, #str_name)
                }
                fn edit_ui_with_id(&mut self, ui: &mut ::nk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::nk_ui::egui::Response {
                    use ::nk_ui::editor::EditableValue;
                    use ::nk_ui::egui;
                    let id = egui::Id::new(id).with(#str_name);
                    let mut changed = false;
                    let res = ui.group(|ui| {
                        ui.columns(#field_count, |uis| {
                            #(#field_impls)*
                        });
                    }).response;
                    if changed {
                        res.mark_changed();
                    }
                    res
                }
            }
        }.into()
    }
}

fn impl_editable_struct(name: &Ident, struc: DataStruct) -> TokenStream {
    let str_name = name.to_string();
    let (field_impls, display) = match struc.fields {
        Fields::Named(ref fields) => {
            (
                impl_struct_named_fields(fields),
                syn::parse_str::<Expr>("::nk_ui::editor::EditableDisplay::Block")
                    .expect("display variant should parse"),
            )
        }
        Fields::Unnamed(ref fields) => return impl_struct_unnamed_fields(name, fields),
        Fields::Unit => unimplemented!(),
    };
    quote! {
        #[automatically_derived]
        impl ::nk_ui::editor::EditableValue for #name {
            const DISPLAY: ::nk_ui::editor::EditableDisplay = #display;
            fn edit_ui(&mut self, ui: &mut ::nk_ui::egui::Ui) -> ::nk_ui::egui::Response {
                use ::nk_ui::editor::EditableValue;
                self.edit_ui_with_id(ui, #str_name)
            }

            fn edit_ui_with_id(&mut self, ui: &mut ::nk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::nk_ui::egui::Response {
                use ::nk_ui::egui;
                use ::nk_ui::editor::EditableValue;
                let id = egui::Id::new(id);
                let mut changed = false;
                let mut res = egui::CollapsingHeader::new(#str_name).id_source(id).show(ui, |ui| {
                    #(#field_impls)*
                });
                let mut res = res.body_response.unwrap_or(res.header_response);
                if changed {
                    res.mark_changed();
                }
                res
            }
        }
    }
    .into()
}

fn impl_editable_enum(name: &Ident, enu: DataEnum) -> TokenStream {
    let str_name = name.to_string();
    let inline = enu.variants.iter().all(|v| v.fields.is_empty());
    let variants = enu.variants.iter().map(|var| {
        let var_name = var.ident.to_string();
        let path: Expr = if var.fields.is_empty() {
            syn::parse_str(&format!("{}::{}", &str_name, var.ident)).unwrap()
        } else {
            syn::parse_str(&format!("{}::{}(_)", &str_name, var.ident)).unwrap()
        };
        #[allow(clippy::redundant_clone)]
        let def_path: Expr = if var.fields.is_empty() {
            path.clone()
        } else {
            syn::parse_str(&format!("{}::{}(Default::default())", &str_name, var.ident)).unwrap()
        };
        quote! {
            let res = ui.add(egui::SelectableLabel::new(matches!(self, #path), #var_name));
            changed |= res.changed();
            if res.clicked() {
                *self = #def_path;
            }
        }
    });
    let uis = enu.variants.iter().map(|var| {
        let is_unit = var.fields.is_empty();
        let path: Expr = if is_unit {
            syn::parse_str(&format!("{}::{}", &str_name, var.ident)).unwrap()
        } else {
            syn::parse_str(&format!("{}::{}(v)", &str_name, var.ident)).unwrap()
        };
        if is_unit {
            quote!(#path => (),)
        } else {
            let field_uis = var
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| quote!(v.edit_ui_with_id(ui, id.with(#i));));
            quote! {
                #path => {
                    #(#field_uis)*
                },
            }
        }
    });
    let var_names = enu.variants.iter().map(|var| {
        let var_name = var.ident.to_string();
        let path: Expr = if var.fields.is_empty() {
            syn::parse_str(&format!("{}::{}", &str_name, var.ident)).unwrap()
        } else {
            syn::parse_str(&format!("{}::{}(_)", &str_name, var.ident)).unwrap()
        };
        quote!(#path => #var_name,)
    });
    let display = if inline {
        syn::parse_str::<Expr>("::nk_ui::editor::EditableDisplay::Inline")
            .expect("display variant should parse")
    } else {
        syn::parse_str::<Expr>("::nk_ui::editor::EditableDisplay::Block")
            .expect("display variant should parse")
    };
    quote! {
        impl #name {
            pub fn variant_name(&self) -> &'static str {
                match self {
                    #(#var_names)*
                }
            }
        }

        #[automatically_derived]
        impl ::nk_ui::editor::EditableValue for #name {
            const DISPLAY: ::nk_ui::editor::EditableDisplay = #display;
            fn edit_ui(&mut self, ui: &mut ::nk_ui::egui::Ui) -> ::nk_ui::egui::Response {
                use ::nk_ui::editor::EditableValue;
                self.edit_ui_with_id(ui, #str_name)
            }

            fn edit_ui_with_id(&mut self, ui: &mut ::nk_ui::egui::Ui, id: impl ::std::hash::Hash) -> ::nk_ui::egui::Response {
                use ::nk_ui::egui;
                use ::nk_ui::editor::EditableValue;
                let mut changed = false;
                let id = egui::Id::new(id);
                let mut res = egui::ComboBox::new(id, #str_name)
                    .selected_text(self.variant_name())
                    .show_ui(ui, |ui| {
                        #(#variants)*
                    }).response;
                match self {
                    #(#uis)*
                };
                if changed {
                    res.mark_changed();
                }
                res
            }
        }
    }.into()
}
