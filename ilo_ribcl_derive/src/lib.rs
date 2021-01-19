extern crate proc_macro;

mod attributes;
mod builder_parse;
mod ribcl_auth;
mod write_ribcl;

use {
    builder_parse::impl_builder_parse_macro,
    ribcl_auth::impl_ribcl_auth_macro,
    syn::{parse_macro_input, DeriveInput},
    write_ribcl::impl_write_ribcl_macro,
};

#[proc_macro_attribute]
pub fn ribcl_auth(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(input as syn::ItemStruct);
    let _ = parse_macro_input!(args as syn::parse::Nothing);
    match impl_ribcl_auth_macro(item_struct) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(BuilderParse, attributes(ribcl))]
pub fn builder_parse_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_builder_parse_macro(&ast) {
        Ok(ts) => proc_macro::TokenStream::from(ts),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(WriteRibcl, attributes(ribcl))]
pub fn write_ribcl_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_write_ribcl_macro(&ast) {
        Ok(ts) => proc_macro::TokenStream::from(ts),
        Err(e) => e.to_compile_error().into(),
    }
}
