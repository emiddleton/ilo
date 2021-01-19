use crate::attributes::{parse_field_attributes, RibclFieldAttributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput};

pub fn impl_write_ribcl_macro(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = ast.ident.clone();
    //eprintln!("STRUCT: {}", name);
    let tags = ilo_ribcl_tags(&ast.data)?;
    //eprintln!("TAGS: {}", tags);
    //eprintln!("MAPS: {}", maps);

    let expanded = quote! {

            //use crate::write_ribcl::WriteRibcl;
            impl crate::write_ribcl::WriteRibcl for #name {
                #[tracing::instrument(skip(writer))]
                fn write_ribcl<W: std::fmt::Write>(&self, writer: &mut W) ->
                    std::result::Result<(), crate::write_ribcl::Error>
                {
                    #tags
                    Ok(())
                }
            }

    };

    //eprintln!("{}", expanded);
    Ok(expanded)
}

pub fn ilo_ribcl_tags(data: &syn::Data) -> syn::Result<proc_macro2::TokenStream> {
    match *data {
        syn::Data::Struct(ref data) => match data.fields {
            syn::Fields::Named(ref fields) => {
                let mut mappings = vec![];
                for field in fields.named.iter() {
                    let field_name = &field.ident;
                    let format_attrs = parse_field_attributes(&field.attrs)?;
                    let mapping = match format_attrs {
                        RibclFieldAttributes {
                            empty: Some(true), ..
                        } => {
                            // eprintln!("printing empty");
                            quote_spanned! { field.span() =>
                                ribcl_tag_empty!(writer, self, #field_name);
                            }
                        }
                        RibclFieldAttributes {
                            mappings: Some(format_str),
                            ..
                        } => {
                            // eprintln!("FORMAT: {:#?}", format_str);
                            quote_spanned! { field.span() =>
                                ribcl_tag!(writer, self, #field_name #format_str);
                            }
                        }
                        _ => {
                            quote_spanned! { field.span() =>
                                ribcl_tag!(writer, self, #field_name);
                            }
                        }
                    };
                    mappings.push(mapping);
                }
                Ok(quote! {#(#mappings)*})
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}
