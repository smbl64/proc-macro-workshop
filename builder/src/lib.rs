use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let Data::Struct(ds) = input.data  else {
        panic!("Only structs are supported.");
    };
    let Fields::Named(fields) = ds.fields else {
        panic!("Only named fields are supported.");
    };

    // Map each "field: Type" to "field: Option<Type>"
    let builder_fields: Vec<_> = fields
        .named
        .pairs()
        .map(|ele| {
            let f = ele.value();
            let name = f.ident.as_ref().unwrap();
            let ty = &f.ty;

            quote! {
                #name: Option<#ty>
            }
        })
        .collect();

    let builder_initial_fields: Vec<_> = fields
        .named
        .pairs()
        .map(|ele| {
            let f = ele.value();
            let name = f.ident.as_ref().unwrap();

            quote! {
                #name: None
            }
        })
        .collect();

    let builder_name = quote::format_ident!("{}Builder", name);
    let builder = quote! {
        pub struct #builder_name {
            #(#builder_fields),*
        }
    };

    let struct_ext = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_initial_fields),*
                }
            }
        }
    };

    let expanded = quote! {
        #struct_ext
        #builder
    };

    eprintln!("{}", pretty_print(&expanded));
    TokenStream::from(expanded)
}

fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}
