use itertools::Itertools;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::iter::Iterator;
use syn::parse_macro_input;

#[proc_macro_derive(Creator, attributes(option, string, path_str))]
pub fn derive_creator(_item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(_item as syn::DeriveInput);
    let item_ident = item.ident;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = item.data
    {
        named
    } else {
        panic!("You can derive only on struct!")
    };

    let function_create = function_create(&item_ident, &fields);
    let with_functions = with_functions(&fields);

    let expanded = quote! {
        impl #item_ident {
            #function_create
            #(#with_functions)*
        }
    };

    expanded.into()
}

fn function_create(
    item_ident: &proc_macro2::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let (option_fields, non_option_fields): (Vec<syn::Field>, Vec<syn::Field>) = fields
        .iter()
        .cloned()
        .partition(|field| field.attrs.iter().any(|attr| attr.path.is_ident("option")));

    let non_option_field_arguments = non_option_fields.iter().map(to_argument);

    let non_option_field_setters = non_option_fields.iter().map(|field| {
        let field_name = &field.ident;
        if field.attrs.iter().any(|attr| attr.path.is_ident("string")) {
            quote! {
                #field_name : String::from(#field_name.as_ref())
            }
        } else if field
            .attrs
            .iter()
            .any(|attr| attr.path.is_ident("path_str"))
        {
            quote! {
                #field_name : #field_name.as_ref()
                .to_string_lossy()
                .to_string()
            }
        } else {
            quote! {
                #field_name : #field_name
            }
        }
    });

    let option_field_setters = option_fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! {
            #field_name : None
        }
    });

    let generics: Vec<proc_macro2::TokenStream> = non_option_fields
        .iter()
        .flat_map(|field| field.attrs.iter().map(|attr| &attr.path))
        .unique()
        .filter_map(|path| {
            if path.is_ident("string") {
                Some(quote! {
                    S: AsRef<str>
                })
            } else if path.is_ident("path_str") {
                Some(quote! {
                    P: AsRef<std::path::Path>
                })
            } else {
                None
            }
        })
        .collect();

    let generic_clause = if generics.len() > 0 {
        quote! {
            <#(#generics,)*>
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    quote! {
        pub fn create#generic_clause(#(#non_option_field_arguments),*) -> Self {
            #item_ident {
                #(#non_option_field_setters,)*
                #(#option_field_setters,)*
            }
        }
    }
}

fn with_functions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    fields.iter().map(|field| {
        let field_name = &field.ident.as_ref().unwrap();
        let function_name = format_ident!("with_{}", field_name);
        let field_argument = to_argument(field);

        let generics = if field.attrs.iter().any(|attr| attr.path.is_ident("string")) {
            quote! { <S: AsRef<str>> }
        } else if field
            .attrs
            .iter()
            .any(|attr| attr.path.is_ident("path_str"))
        {
            quote! { <P: AsRef<std::path::Path>> }
        } else {
            proc_macro2::TokenStream::new()
        };
        quote! {
            pub fn #function_name#generics(mut self, #field_argument) -> Self {
                self.#field_name = #field_name;
                self
            }
        }
    })
}

fn to_argument(field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    if field.attrs.iter().any(|attr| attr.path.is_ident("string")) {
        quote! {
            #field_name : S
        }
    } else if field
        .attrs
        .iter()
        .any(|attr| attr.path.is_ident("path_str"))
    {
        quote! {
            #field_name : P
        }
    } else {
        let field_type = &field.ty;
        quote! {
            #field_name : #field_type
        }
    }
}
