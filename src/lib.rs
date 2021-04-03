use itertools::Itertools;
use proc_macro::TokenStream;
use quote::quote;
use std::iter::Iterator;
use syn::parse_macro_input;

#[proc_macro_derive(Creator, attributes(create, string, path_str))]
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
    //TODO: fix with_functions

    let expanded = quote! {
        impl #item_ident {
            #function_create
            
        }
    };

    expanded.into()
}

fn function_create(
    item_ident: &proc_macro2::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let (create_fields, non_create_fields): (Vec<syn::Field>, Vec<syn::Field>) = fields
        .iter()
        .cloned()
        .partition(|field| field.attrs.iter().any(|attr| attr.path.is_ident("create")));

    let create_field_arguments = create_fields.iter().map(to_argument);

    let create_field_setters = create_fields.iter().map(|field| {
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

    let non_create_field_setters = non_create_fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! {
            #field_name : None
        }
    });

    let generics: Vec<proc_macro2::TokenStream> = create_fields
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
        pub fn create#generic_clause(#(#create_field_arguments),*) -> Self {
            #item_ident {
                #(#create_field_setters,)*
                #(#non_create_field_setters,)*
            }
        }
    }
}

fn with_functions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let bool_fields = fields.iter().filter(|field| 
        match &field.ty {
            syn::Type::Path(type_path) if type_path.path.is_ident("bool") => true,
            _ => false,
        }
    );
    let field_setters = fields.iter().map(|field| {
        let field_name = &field.ident;
        let function_name = format!("with_{}", field_name.as_ref().unwrap());
        let field_ref = format!("self.{}", field_name.as_ref().unwrap());
        let field_argument = to_argument(field);
        eprintln!("{:?}", function_name);

        quote!{
            pub fn #function_name(mut self, #field_argument) -> Self {
                #field_ref = #field_name;
                self
            }
        }
    });
    let bool_setters = bool_fields.map(|field| {
        let field_name = &field.ident;
        let function_name = format!("with_{}", field_name.as_ref().unwrap());
        quote! {
            pub fn #field_name(self) {
                #function_name(true)
            }
        }
    });
    let combined = quote! {
        #(#bool_setters)*
        #(#field_setters)*
    };
    combined
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