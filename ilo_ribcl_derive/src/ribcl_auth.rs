use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, ItemStruct};

pub fn impl_ribcl_auth_macro(item_struct: ItemStruct) -> syn::Result<TokenStream> {
    let mut item_struct = item_struct;
    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.push(syn::Field::parse_named.parse2(quote! {
            /// don't update endpoint file
            #[structopt(short, long)]
            pub no_update: bool
        })?);
        fields.named.push(syn::Field::parse_named.parse2(quote! {
            /// use cached for return values
            #[structopt(short, long)]
            pub proxy_cache: bool
        })?);
        fields.named.push(syn::Field::parse_named.parse2(quote! {
            // endpoint info file
            #[structopt(short, long, parse(from_os_str), default_value = "endpoint.json")]
            pub endpoint: std::path::PathBuf
        })?);
    }
    Ok(quote! {
        #item_struct
    }
    .into())
}
