use quote::TokenStreamExt;
use syn::{
    braced,
    ext::IdentExt,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token,
};

macro_rules! fail {
    ($t:expr, $m:expr) => {
        return Err(syn::Error::new_spanned($t, $m));
    };
}

macro_rules! try_set {
    ($i:ident, $v:expr, $t:expr) => {
        match $i {
            None => $i = Some($v),
            Some(_) => fail!($t, "duplicate attribute"),
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub enum DataMapSource {
    Attributes,
    Elements,
}

pub struct RibclContainerAttributes {
    pub map_source: Option<DataMapSource>,
}

pub fn parse_container_attributes(
    input: &[syn::Attribute],
) -> syn::Result<RibclContainerAttributes> {
    let mut map_source = None;
    for attr in input {
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;
        match meta {
            syn::Meta::List(list) if list.path.is_ident("ribcl") => {
                for value in list.nested.iter() {
                    match value {
                        syn::NestedMeta::Meta(meta) => match meta {
                            syn::Meta::Path(p) if p.is_ident("elements") => {
                                try_set!(map_source, DataMapSource::Elements, value)
                            }
                            syn::Meta::Path(p) if p.is_ident("attributes") => {
                                try_set!(map_source, DataMapSource::Attributes, value)
                            }
                            u => fail!(u, "unexpected value"),
                        },
                        u => fail!(u, "unexpected attribute"),
                    }
                }
            }
            _ => {}
        }
    }
    Ok(RibclContainerAttributes { map_source })
}

#[derive(Debug)]
pub enum AttrMap {
    Same(syn::Ident),
    Rename(syn::Ident, syn::Ident),
}

impl Parse for AttrMap {
    fn parse(input: ParseStream) -> Result<Self> {
        let first: syn::Ident = input.parse()?;
        use AttrMap::*;
        if input.peek(syn::Token![:]) {
            input.parse::<syn::Token![:]>()?;
            Ok(Rename(first, input.parse()?))
        } else {
            Ok(Same(first))
        }
    }
}

impl quote::ToTokens for AttrMap {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use AttrMap::*;
        match self {
            Same(field) => {
                tokens.append(field.clone());
            }
            Rename(field, attr) => {
                tokens.append(field.clone());
                tokens.append(proc_macro2::Punct::new(':', proc_macro2::Spacing::Alone));
                tokens.append(attr.clone());
            }
        }
    }
}

#[derive(Debug)]
pub struct MapArgs {
    pub tag: Option<syn::Ident>,
    pub attr: Option<syn::Ident>,
    pub brace_token: Option<syn::token::Brace>,
    pub attrs: Option<Punctuated<AttrMap, syn::Token![,]>>,
}

impl quote::ToTokens for MapArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if let Some(tag) = &self.tag {
            tokens.append(proc_macro2::Punct::new(':', proc_macro2::Spacing::Alone));
            tokens.append(tag.clone());
        }
        if let Some(attr) = &self.attr {
            tokens.append(proc_macro2::Punct::new('.', proc_macro2::Spacing::Alone));
            tokens.append(attr.clone());
        }
        if let (Some(brace_token), Some(attrs)) = (&self.brace_token, &self.attrs) {
            brace_token.surround(tokens, |tokens| {
                attrs.to_tokens(tokens);
            });
        }
    }
}

mod kw {
    syn::custom_keyword!(empty);
}

impl Parse for MapArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attr = None;
        let mut attrs = None;
        let mut brace_token = None;
        let tag = input.call(syn::Ident::parse_any).ok();
        if input.peek(syn::Token![.]) {
            input.parse::<syn::Token![.]>()?;
            attr = input.parse().ok();
        } else if input.peek(token::Brace) {
            let content;
            brace_token = Some(braced!(content in input));
            attrs =
                Some(Punctuated::<AttrMap, syn::Token![,]>::parse_separated_nonempty(&content)?);
        }
        Ok(MapArgs {
            tag,
            attr,
            brace_token,
            attrs,
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum ArrayStyle {
    Nested,
    Inline,
}

#[derive(Debug)]
pub struct RibclFieldAttributes {
    pub array_style: ArrayStyle,
    pub empty: Option<bool>,
    pub mappings: Option<MapArgs>,
    pub map_source: Option<DataMapSource>,
}

pub fn parse_field_attributes(input: &[syn::Attribute]) -> syn::Result<RibclFieldAttributes> {
    let mut array_style = None;
    let mut empty = None;
    let mut mappings = None;
    let mut map_source = None;
    for attr in input {
        // eprintln!("FIELD ATTRIBUTE: {:#x?}", attr);

        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;
        match meta {
            syn::Meta::List(list) if list.path.is_ident("ribcl") => {
                for value in list.nested.iter() {
                    match value {
                        syn::NestedMeta::Meta(meta) => match meta {
                            syn::Meta::Path(path) if path.is_ident("inline") => {
                                try_set!(array_style, ArrayStyle::Inline, value)
                            }
                            syn::Meta::Path(path) if path.is_ident("empty") => {
                                try_set!(empty, true, value)
                            }
                            syn::Meta::Path(p) if p.is_ident("elements") => {
                                try_set!(map_source, DataMapSource::Elements, value)
                            }
                            syn::Meta::Path(path) if path.is_ident("attribute") => {
                                try_set!(map_source, DataMapSource::Attributes, value)
                            }
                            syn::Meta::NameValue(syn::MetaNameValue {
                                path,
                                lit: syn::Lit::Str(val),
                                ..
                            }) if path.is_ident("map") => {
                                let map_str = val.value().to_string();
                                try_set!(mappings, syn::parse_str(&map_str)?, value)
                            }
                            u => fail!(u, "unexpected value"),
                        },
                        u => fail!(u, "unexpected attribute"),
                    }
                }
            }
            _ => {}
        }
    }
    Ok(RibclFieldAttributes {
        array_style: array_style.unwrap_or(ArrayStyle::Nested),
        empty,
        mappings,
        map_source,
    })
}
