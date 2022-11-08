use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> TokenStream {
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
    let builder_name = format_ident!("{}Builder", struct_name);
    let struct_ext = make_struct_ext(&builder_name, fields, struct_name);
    let builder = make_builder(struct_name, &builder_name, fields);

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

fn make_builder(
    struct_name: &Ident,
    builder_name: &Ident,
    fields: &FieldsNamed,
) -> proc_macro2::TokenStream {
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

    let setters = make_builder_setters(&fields);
    let build_method = make_build_method(struct_name, fields);
    let builder = quote! {
        pub struct #builder_name {
            #(#builder_fields),*
        }

        impl #builder_name {
            #(#setters)*

            #build_method
        }
    };

    builder
}

fn make_builder_setters(fields: &FieldsNamed) -> Vec<proc_macro2::TokenStream> {
    fields
        .named
        .pairs()
        .map(|p| {
            let field = p.value();
            let name = &field.ident.as_ref().unwrap();
            let ty = &field.ty;
            quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            }
        })
        .collect()
}

fn make_build_method(struct_name: &Ident, fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let names: Vec<_> = fields
        .named
        .pairs()
        .map(|p| {
            let field = p.value();
            field.ident.as_ref().unwrap()
        })
        .collect();

    quote! {
        fn build (self) -> Result<#struct_name, Box<dyn std::error::Error>> {
            #(
            if self.#names.is_none() {
                let msg = format!("{} has no value.", stringify!(#names));
                return Err(msg.into());
            }
            )*

            Ok(#struct_name {
                #(
                    #names: self.#names.unwrap(),
                )*
            })

        }
    }
}

fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}
