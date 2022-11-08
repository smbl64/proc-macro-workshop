use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed};

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

    let result = generate(&name, &fields);
    eprintln!("{}", pretty_print(&result));
    TokenStream::from(result)
}

fn generate(struct_name: &Ident, fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let builder_name = quote::format_ident!("{}Builder", struct_name);
    let struct_ext = make_struct_ext(&builder_name, fields, struct_name);
    let builder = make_builder(&builder_name, &fields);

    quote! {
        #struct_ext
        #builder
    }
}

fn make_struct_ext(
    builder_name: &Ident,
    fields: &FieldsNamed,
    struct_name: &Ident,
) -> proc_macro2::TokenStream {
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

    let struct_ext = quote! {
        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_initial_fields),*
                }
            }
        }
    };
    struct_ext
}

fn make_builder(builder_name: &Ident, fields: &FieldsNamed) -> proc_macro2::TokenStream {
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

    let builder = quote! {
        pub struct #builder_name {
            #(#builder_fields),*
        }
    };

    builder
}

fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}
