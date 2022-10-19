use proc_macro::TokenStream;

#[proc_macro_derive(Editable)]
pub fn editable(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    panic!("My struct name is: <{}>", ast.ident);
    TokenStream::new()
}
