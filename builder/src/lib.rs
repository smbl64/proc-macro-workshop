use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, GenericArgument, PathArguments, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let Data::Struct(ds) = input.data else {
        panic!("Only structs are supported.");
    };
    let Fields::Named(fields) = ds.fields else {
        panic!("Only named fields are supported.");
    };

    let result = generate(&struct_name, &fields);
    eprintln!("{}", pretty_print(&result));
    TokenStream::from(result)
}

fn generate(struct_name: &Ident, fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let builder_name = format_ident!("{}Builder", struct_name);
    let builder_factory = make_builder_factory(&builder_name, fields, struct_name);
    let builder = make_builder(struct_name, &builder_name, fields);

    quote! {
        #builder_factory
        #builder
    }
}

fn make_builder_factory(
    builder_name: &Ident,
    struct_fields: &FieldsNamed,
    struct_name: &Ident,
) -> proc_macro2::TokenStream {
    let builder_initial_fields: Vec<_> = struct_fields
        .named
        .pairs()
        .map(|pair| {
            let field = pair.value();
            let name = field.ident.as_ref().unwrap();
            //if field.attrs.len() > 0 {
            //    let a = field.attrs.first().unwrap();
            //    dbg!(a);
            //}

            quote! {
                #name: None
            }
        })
        .collect();

    let builder_factory = quote! {
        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_initial_fields),*
                }
            }
        }
    };
    builder_factory
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
        .map(|pair| {
            let field = pair.value();
            let name = field.ident.as_ref().unwrap();
            let mut ty = &field.ty;

            let inner_ty = find_inner_type(ty);
            if inner_ty.is_some() {
                ty = inner_ty.unwrap();
            }

            quote! {
                #name: Option<#ty>
            }
        })
        .collect();

    let setters = make_builder_setters(&fields);
    let build_method = make_build_method(struct_name, fields);
    quote! {
        pub struct #builder_name {
            #(#builder_fields),*
        }

        impl #builder_name {
            #(#setters)*

            #build_method
        }
    }
}

fn make_builder_setters(struct_fields: &FieldsNamed) -> Vec<proc_macro2::TokenStream> {
    struct_fields
        .named
        .pairs()
        .map(|p| {
            let field = p.value();
            let name = &field.ident.as_ref().unwrap();
            let mut ty = &field.ty;
            let inner_ty = find_inner_type(ty);

            if inner_ty.is_some() {
                ty = inner_ty.unwrap();
            }

            quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            }
        })
        .collect()
}

fn make_build_method(struct_name: &Ident, struct_fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let mandatory_field_names: Vec<_> = struct_fields
        .named
        .pairs()
        .filter(|p| find_inner_type(&p.value().ty).is_none())
        .map(|p| {
            let field = p.value();
            field.ident.as_ref().unwrap()
        })
        .collect();

    let optional_field_names: Vec<_> = struct_fields
        .named
        .pairs()
        .filter(|p| find_inner_type(&p.value().ty).is_some())
        .map(|p| {
            let field = p.value();
            field.ident.as_ref().unwrap()
        })
        .collect();

    quote! {
        fn build (&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
            #(
            if self.#mandatory_field_names.is_none() {
                let msg = format!("{} has no value.", stringify!(#mandatory_field_names));
                return Err(msg.into());
            }
            )*

            Ok(#struct_name {
                #(#mandatory_field_names: std::mem::take(&mut self.#mandatory_field_names).unwrap(),)*
                #(#optional_field_names: std::mem::take(&mut self.#optional_field_names),)*
            })

        }
    }
}

fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}

/// Find T in an `Option<T>` declaration.
/// See "tests/06-optional-field.rs" for the pattern.
fn find_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let Some(path_segment) = type_path.path.segments.first() else {
        return None;
    };

    if path_segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(ref args) = path_segment.arguments else {
        return None;
    };

    let Some(GenericArgument::Type(ref t)) = args.args.first() else {
        return None;
    };

    Some(t)
}
