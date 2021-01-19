use crate::attributes::{
    parse_container_attributes, parse_field_attributes, ArrayStyle, DataMapSource, MapArgs,
    RibclContainerAttributes,
};
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Ident, TypePath};

pub fn impl_builder_parse_macro(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_type = ast.ident.clone();
    // eprintln!("STRUCT: {}", struct_type);
    let struct_label = struct_type.to_string();
    let struct_type_builder = Ident::new(
        &format!("{}Builder", ast.ident.clone().to_string().to_pascal_case()),
        Span::call_site(),
    );
    //eprintln!("STRUCT BUILDER: {}", struct_type_builder);
    let builder_fields = ilo_ribcl_builder_fields(&ast.data)?;
    //eprintln!("BUILDER FIELDS: {}", builder_fields);

    // mapping from xml
    let container_attrs = parse_container_attributes(&ast.attrs)?;

    let field_initializations = ilo_ribcl_field_initializations(&ast.data)?;

    let attribute_mappings = ilo_ribcl_builder_attribute_mappings(&ast, &container_attrs)?;
    let builder_parse_attributes_mappings = if !attribute_mappings.is_empty() {
        quote! {
            for attribute in parent_element.attributes() {
                match attribute {
                    #(#attribute_mappings)*,
                    _ => {}
                }
            }
        }
    } else {
        quote! {}
    };

    let element_mappings = ilo_ribcl_builder_element_mappings(&ast, &container_attrs)?;
    let builder_parse_element_mappings = if !element_mappings.is_empty() {
        quote! {
            // process nested elements
            crate::xml::Event::Start(elem) | crate::xml::Event::Empty(elem) => {
                tracing::event!(tracing::Level::DEBUG, element=?elem);
                use inflector::Inflector;
                let elem_name = String::from_utf8(elem.name().to_vec())?.to_snake_case();
                match elem_name.as_str() {
                    "ribcl" => {}
                    "response" => crate::xml::handle_ribcl_response_errors(elem.into_owned())?,
                    #(#element_mappings)*
                    ignored => tracing::event!(tracing::Level::DEBUG, "IGNORED: {:#x?}", ignored),
                }
            },
        }
    } else {
        quote! {}
    };

    let empty_element_mappings = ilo_ribcl_builder_empty_element_mappings(&ast, &container_attrs)?;
    /*
        let builder_parse_empty_element_mappings = if empty_element_mappings.len() > 0 {
            quote! {
                // process nested elements
                crate::xml::Event::Empty(e) => {
                    tracing::event!(tracing::Level::DEBUG, element=?e);
                    use inflector::Inflector;
                    let elem_name = String::from_utf8(elem.name().to_vec())?.to_snake_case();
                    match elem_name.as_str() {
                        #(#empty_element_mappings)*
                        e => event!(Level::DEBUG, "IGNORED: {:#x?}", e),
                    }
                },
            }
        } else {
            quote! {}
        };
    */
    let element_mappings = if !element_mappings.is_empty() || !empty_element_mappings.is_empty() {
        quote! {
            let break_on = parent_element.name().to_ascii_lowercase();
            let break_on_value = String::from_utf8(break_on.clone()).unwrap();
            tracing::event!(tracing::Level::DEBUG, break_on = ?break_on_value);
            let mut buf = Vec::new();
            // must have children
            if let crate::xml::Event::Start(_) = parent {
                loop {
                    let event = self.reader.read_event(&mut buf)?;
                    match event.clone() {
                        #builder_parse_element_mappings
                        crate::xml::Event::End(ref element) if &element.name().to_ascii_lowercase() == &break_on => {
                            tracing::event!(tracing::Level::DEBUG, "BREAKING {}", #struct_label);
                            break;
                        }
                        crate::xml::Event::Eof => {
                            tracing::event!(tracing::Level::DEBUG, "BREAKING {} Eof", #struct_label);
                            return Err(crate::builder_parse::Error::NotFound {
                                target: #struct_label,
                            });
                        }
                        _ => {}
                    }
                    buf.clear();
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
            #[derive(std::default::Default, std::fmt::Debug)]
            pub struct #struct_type_builder {
                #builder_fields
            }

            impl<'a, B: std::io::BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, #struct_type_builder> for crate::xml::XmlCursor<B> {
                #[tracing::instrument(skip(self, parent))]
                fn builder_parse(
                    &mut self,
                    parent: crate::xml::Event<'a>,
                    builder: ::std::option::Option<#struct_type_builder>
                ) -> ::std::result::Result<#struct_type_builder, crate::builder_parse::Error> {
                    let mut builder = builder.unwrap_or_else(|| Default::default());
                    let parent_element = match parent {
                        crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                            element.clone().into_owned()
                        },
                        _ => unreachable!(),
                    };
                    #builder_parse_attributes_mappings
                    #element_mappings
                    Ok(builder)
                }
            }

            impl std::convert::TryFrom<#struct_type_builder> for #struct_type {
                type Error = crate::builder_parse::Error;
                #[tracing::instrument(skip(builder))]
                fn try_from(builder: #struct_type_builder) -> Result<Self, Self::Error> {
                    tracing::event!(tracing::Level::DEBUG, entering_type=stringify!(#struct_type));
                    Ok(#struct_type {
                        #field_initializations
                    })
                }
            }
    };

    //eprintln!("{}", expanded);
    Ok(expanded)
}

fn ilo_ribcl_builder_fields(data: &Data) -> syn::Result<TokenStream> {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mappings = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    //let field_type = &f.ty;
                    let builder_type = make_builder_type(&f.ty);
                    // eprintln!("TYPE: {:#?}", field_type);
                    // eprintln!("BUILDER TYPE: {}", builder_type);
                    if is_vec(&f.ty) {
                        quote_spanned! { f.span() =>
                            // #field_type
                            #field_name: #builder_type
                        }
                    } else {
                        quote_spanned! { f.span() =>
                            // #field_type
                            #field_name: Option<#builder_type>
                        }
                    }
                });
                Ok(quote! {#(#mappings),*})
            }
            _ => {
                unimplemented!()
                /*
                return Err(syn::Error::new_spanned(
                    data,
                    "only structs with named fields are supported",
                ))
                */
            }
        },
        _ => unimplemented!(),
    }
}

fn is_vec(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => matches!(segments.first(),
            Some(syn::PathSegment { ident, .. }) if "Vec" == ident.to_string().as_str()),
        _ => false,
    }
}

fn make_builder_type(ty: &syn::Type) -> TokenStream {
    match ty {
        syn::Type::Path(TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => {
            //let segment = segments.first().unwrap();
            //let PathSegment { ident, arguments } = segments.first().unwrap();
            match segments.first() {
                // nested type
                Some(syn::PathSegment {
                    ident,
                    arguments: syn::PathArguments::AngleBracketed(gen),
                }) => {
                    let ident = ident.to_string();
                    let builder_type = match ident.as_str() {
                        "Option" => Ident::new(&ident, Span::call_site()),
                        _ => Ident::new(
                            &format!("{}Builder", ident.to_string().to_pascal_case()),
                            Span::call_site(),
                        ),
                    };

                    match gen.args.first() {
                        Some(syn::GenericArgument::Type(inner_type)) => {
                            let inner_builder_type = make_builder_type(inner_type);
                            quote! { #builder_type<#inner_builder_type> }
                        }
                        _ => unimplemented!(),
                    }
                }
                // plain type
                Some(syn::PathSegment {
                    ident,
                    arguments: syn::PathArguments::None,
                }) => {
                    let builder = Ident::new(
                        &format!("{}Builder", ident.to_string().to_pascal_case()),
                        Span::call_site(),
                    );
                    quote! { #builder }
                }
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}

fn ilo_ribcl_builder_attribute_mappings(
    ast: &DeriveInput,
    container_attrs: &RibclContainerAttributes,
) -> syn::Result<Vec<TokenStream>> {
    let mut element_mappings = vec![];
    match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in fields.named.iter() {
                    // eprintln!("TYPE: {:#?}", &field.ty);
                    let field_name = &field.ident;
                    let attrs = parse_field_attributes(&field.attrs)?;
                    // eprintln!("FIELD ATTRIBUTES: {:#?}", &attrs);
                    if let syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    }) = &field.ty
                    {
                        if let Some(syn::PathSegment { ident, .. }) = segments.first() {
                            use DataMapSource::*;
                            match (container_attrs.map_source, attrs.map_source) {
                                (Some(Attributes), None) | (_, Some(Attributes)) => {
                                    let tag_name = match attrs.mappings {
                                        Some(MapArgs {
                                            tag: Some(name), ..
                                        }) => Some(name).map(|n| n.to_string()),
                                        _ => field_name.clone().map(|n| n.to_string()),
                                    };
                                    let mappings = if "Option" == ident.to_string().as_str() {
                                        // eprintln!("OPTIONAL IDENT: {:#?}", ident);
                                        quote_spanned! { field.span() =>
                                            Ok(a) if #tag_name.as_bytes() == a.key.to_ascii_lowercase() => {
                                                builder.#field_name = Some(a.ribcl_into()?)
                                            }
                                        }
                                    } else {
                                        // eprintln!("REQUIRED IDENT: {:#?}", ident);
                                        quote_spanned! { field.span() =>
                                            Ok(a) if #tag_name.as_bytes() == a.key.to_ascii_lowercase() => {
                                                builder.#field_name = a.ribcl_into()?
                                            }
                                        }
                                    };
                                    element_mappings.push(mappings);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                //Ok(quote! {#(#mappings)* })
                Ok(element_mappings)
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

//fn ilo_ribcl_expand_mappings(field: Field) -> syn::Result<>
fn ilo_ribcl_builder_element_mappings(
    ast: &DeriveInput,
    container_attrs: &RibclContainerAttributes,
) -> syn::Result<Vec<TokenStream>> {
    let mut element_mappings = vec![];
    /*
    if let DataMapSource::Attributes = container_attrs.map_source {
        return Ok(element_mappings);
    }
    */
    match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in fields.named.iter() {
                    let field_name = &field.ident;
                    let attrs = parse_field_attributes(&field.attrs)?;
                    if let syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    }) = &field.ty
                    {
                        if let Some(syn::PathSegment { ident, .. }) = segments.first() {
                            use DataMapSource::*;
                            let ident_str = ident.to_string();
                            match (container_attrs.map_source, attrs.map_source) {
                                (None, None) | (Some(Elements), None) | (_, Some(Elements)) => {
                                    let tag_name = match attrs.mappings {
                                        Some(MapArgs {
                                            tag: Some(name), ..
                                        }) => Some(name),
                                        _ => field_name.clone(),
                                    }
                                    .map(|n| n.to_string());
                                    let mapping = match (ident_str.as_str(), attrs.array_style) {
                                        ("Vec", ArrayStyle::Inline) => {
                                            //eprintln!("OPTIONAL IDENT: {:#?}",ident);
                                            quote_spanned! { field.span() =>
                                                #tag_name => {
                                                    let mut #field_name = builder.#field_name.0;
                                                    tracing::event!(tracing::Level::DEBUG, "BEFORE INLINE Vec -> {:#x?}", #field_name);
                                                    #field_name.push(self.builder_parse(event.into_owned().clone(), None)?);
                                                    tracing::event!(tracing::Level::DEBUG, "AFTER INLINE Vec -> {:#x?}", #field_name);
                                                    builder.#field_name = VecBuilder(#field_name)
                                                }
                                            }
                                        }
                                        ("Vec", _) => {
                                            //eprintln!("OPTIONAL IDENT: {:#?}",ident);
                                            quote_spanned! { field.span() =>
                                                #tag_name => {
                                                    tracing::event!(tracing::Level::DEBUG, "BEFORE INLINE Vec -> {:#x?}", builder.#field_name);
                                                    builder.#field_name = self.builder_parse(event.into_owned().clone(), None)?;
                                                    tracing::event!(tracing::Level::DEBUG, "AFTER INLINE Vec -> {:#x?}", builder.#field_name);
                                                }
                                            }
                                        }
                                        _ => {
                                            //eprintln!("REQUIRED IDENT: {:#?}",ident);
                                            quote_spanned! { field.span() =>
                                                #tag_name =>
                                                    builder.#field_name = Some(self.builder_parse(event.into_owned().clone(), builder.#field_name)?),
                                            }
                                        }
                                    };
                                    element_mappings.push(mapping);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Ok(element_mappings)
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

//fn ilo_ribcl_expand_mappings(field: Field) -> syn::Result<>
fn ilo_ribcl_builder_empty_element_mappings(
    ast: &DeriveInput,
    container_attrs: &RibclContainerAttributes,
) -> syn::Result<Vec<TokenStream>> {
    let mut element_mappings = vec![];
    match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in fields.named.iter() {
                    let field_name = &field.ident;
                    let attrs = parse_field_attributes(&field.attrs)?;
                    if let syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    }) = &field.ty
                    {
                        match segments.first() {
                            Some(syn::PathSegment { ident, .. }) => {
                                use DataMapSource::*;
                                //let ident_str = ident.to_string().as_str();
                                match (container_attrs.map_source, attrs.map_source) {
                                    (None, None)
                                    | (Some(Elements), None)
                                    | (None, Some(Elements)) => {
                                        let tag_name = match attrs.mappings {
                                            Some(MapArgs {
                                                tag: Some(name), ..
                                            }) => Some(name),
                                            _ => field_name.clone(),
                                        }
                                        .map(|f| f.to_string());
                                        use ArrayStyle::*;
                                        let mapping = match (
                                            ident.to_string().as_str(),
                                            attrs.array_style,
                                        ) {
                                            ("Vec", Inline) => {
                                                //eprintln!("OPTIONAL IDENT: {:#?}",ident);
                                                quote_spanned! { field.span() =>
                                                    #tag_name => {
                                                        let mut #field_name = builder.#field_name.unwrap_or_else(|| Default::default());
                                                        tracing::event!(tracing::Level::DEBUG, "BEFORE INLINE Vec -> {:#x?}", #field_name);
                                                        #field_name.0.push(self.builder_parse(event.into_owned().clone(), None)?);
                                                        tracing::event!(tracing::Level::DEBUG, "AFTER INLINE Vec -> {:#x?}", #field_name);
                                                        builder.#field_name = Some(#field_name)
                                                    }
                                                }
                                            }
                                            ("Vec", _) => {
                                                //eprintln!("OPTIONAL IDENT: {:#?}",ident);
                                                quote_spanned! { field.span() =>
                                                    #tag_name => {
                                                        tracing::event!(tracing::Level::DEBUG, "BEFORE INLINE Vec -> {:#x?}", builder.#field_name);
                                                        builder.#field_name = self.builder_parse(event.into_owned().clone(), None)?;
                                                        tracing::event!(tracing::Level::DEBUG, "AFTER INLINE Vec -> {:#x?}", builder.#field_name);
                                                    }
                                                }
                                            }
                                            _ => {
                                                //eprintln!("REQUIRED IDENT: {:#?}",ident);
                                                quote_spanned! { field.span() =>
                                                    #tag_name =>
                                                        builder.#field_name = Some(self.builder_parse(event.into_owned().clone(), builder.#field_name)?),
                                                }
                                            }
                                        };
                                        element_mappings.push(mapping);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                unimplemented!()
                            }
                        }
                    }
                }
                Ok(element_mappings)
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn ilo_ribcl_field_initializations(data: &Data) -> syn::Result<TokenStream> {
    let result = match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut mappings = vec![];
                for field in fields.named.iter() {
                    mappings.push(make_field_initialization(field)?);
                }
                quote! {#(#mappings),*}
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    };
    Ok(result)
}

fn make_field_initialization(field: &syn::Field) -> syn::Result<TokenStream> {
    let field_name = &field.ident;
    let field_label = field_name.clone().unwrap().to_string();
    let result = match &field.ty {
        syn::Type::Path(TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => {
            let segment = segments
                .first()
                .ok_or_else(|| syn::Error::new(field.span(), "field is missing first element"))?;
            //let PathSegment { ident, arguments } = segments.first().unwrap();
            match segment {
                // nested type
                syn::PathSegment {
                    ident,
                    arguments: syn::PathArguments::AngleBracketed(..),
                    ..
                } => {
                    //eprintln!("OPTIONAL INITIALIZER: {}", field_name.clone().unwrap());
                    if "Vec" == ident.to_string().as_str() {
                        quote_spanned! { field.span() =>
                            #field_name: {
                                tracing::event!(tracing::Level::DEBUG, #field_name=#field_label, "vec field");
                                tracing::event!(tracing::Level::DEBUG, "builder_value={:#x?}", builder.#field_name);
                                builder.#field_name.try_into()?
                            }
                        }
                    } else {
                        quote_spanned! { field.span() =>
                            #field_name: {
                                tracing::event!(tracing::Level::DEBUG, #field_name=#field_label, "optional field");
                                tracing::event!(tracing::Level::DEBUG, builder_value=?builder.#field_name);
                                builder.#field_name.unwrap_or_default().map_or_else::<Result<
                                    _,
                                    crate::builder_parse::Error
                                >, _, _>(
                                    || Ok(None),
                                    |b| {
                                        match b.try_into() {
                                            Ok(val) => Ok(Some(val)),
                                            // as this is optional ignore incomplete children
                                            Err(crate::builder_parse::Error::NotFound{..}) => Ok(None),
                                            Err(e) => Err(e),
                                        }
                                    }
                                )?
                            }
                        }
                    }
                }
                syn::PathSegment { .. } => {
                    //eprintln!("REQUIRED INITIALIZER: {}", field_name.clone().unwrap());
                    quote_spanned! { field.span() =>
                        #field_name: {
                            tracing::event!(tracing::Level::DEBUG, #field_name=#field_label, "required field");
                            builder
                            .#field_name
                            .ok_or(crate::builder_parse::Error::NotFound {
                                target: #field_label,
                            })?
                            .try_into()?
                        }
                    }
                }
            }
        }
        _ => unimplemented!(),
    };
    Ok(result)
}
