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
    //eprintln!("{}", pretty_print(&result));
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

struct InternalField<'a> {
    name: &'a Ident,
    ty: &'a Type,
    inner_ty: Option<&'a Type>,
}

impl<'a> InternalField<'a> {
    /// If `self.ty` is `Option<T>`, return `T`. Otherwise return `self.ty`.
    fn get_core_type(&self) -> &Type {
        match self.inner_ty {
            Some(t) => t,
            None => self.ty,
        }
    }
}

fn transform_fields<'a>(fields: &'a FieldsNamed) -> Vec<InternalField<'a>> {
    fields
        .named
        .pairs()
        .map(|pair| {
            let field = pair.value();
            let ty = &field.ty;
            let inner_ty = find_inner_type(ty);
            let name = field.ident.as_ref().unwrap();

            InternalField { name, ty, inner_ty }
        })
        .collect()
}

fn make_builder_factory(
    builder_name: &Ident,
    struct_fields: &FieldsNamed,
    struct_name: &Ident,
) -> proc_macro2::TokenStream {
    let builder_initial_fields: Vec<_> = transform_fields(struct_fields)
        .into_iter()
        .map(|f| {
            let name = f.name;
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
    let builder_fields: Vec<_> = transform_fields(fields)
        .into_iter()
        .map(|field| {
            let name = field.name;
            let ty = field.get_core_type();

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
    transform_fields(struct_fields)
        .into_iter()
        .map(|field| {
            let name = field.name;
            let ty = field.get_core_type();

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
    let mandatory_field_names: Vec<_> = transform_fields(struct_fields)
        .into_iter()
        .filter(|f| f.inner_ty.is_none())
        .map(|f| f.name)
        .collect();

    let optional_field_names: Vec<_> = transform_fields(struct_fields)
        .into_iter()
        .filter(|f| f.inner_ty.is_some())
        .map(|f| f.name)
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
