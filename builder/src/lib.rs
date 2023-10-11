use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Expr, Lit, Type, Data, Fields, PathArguments, GenericArgument, Ident};
use std::collections::HashMap;
use syn::spanned::Spanned;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let syn_tree: syn::DeriveInput = syn::parse_macro_input!(input);

    let ident = syn_tree.ident;

    let fields_named = match syn_tree.data {
        Data::Struct(data_struct) => {
            match data_struct.fields {
                Fields::Named(fields_named) => fields_named,
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    };

    let mut optional_fields = HashMap::<&Ident, &Type>::new();
    let mut vec_fields = HashMap::<&Ident, &Type>::new();
    let mut fields = HashMap::<&Ident, &Type>::new();

    // Goes through each field and checks for the Option field
    for field in fields_named.named.iter() {
        let field_ident = if let Some(field_ident) = &field.ident {
            field_ident
        } else {
            unimplemented!()
        };

        if let Type::Path(type_path) = &field.ty {
            // Since we are only looking for the `Option` type, we only check the last argument
            if let Some(path_seg) = type_path.path.segments.last() {
                match path_seg.ident.to_string().as_ref() {
                    "Option" => {
                        if let PathArguments::AngleBracketed(angle_bracketed_ga) = &path_seg.arguments {
                            // We know `Option` takes a single generic argument, so we only take the
                            // first one out of the sequence
                            if let Some(GenericArgument::Type(generic_arg_ty)) =
                                angle_bracketed_ga.args.first()
                            {
                                optional_fields.insert(field_ident, generic_arg_ty);
                            }
                        }
                    }
                    "Vec" => {
                        if let PathArguments::AngleBracketed(angle_bracketed_ga) = &path_seg.arguments {
                            // We know `Option` takes a single generic argument, so we only take the
                            // first one out of the sequence
                            if let Some(GenericArgument::Type(generic_arg_ty)) =
                                angle_bracketed_ga.args.first()
                            {
                                vec_fields.insert(field_ident, generic_arg_ty);
                            }
                        }
                    }
                    _ => { fields.insert(field_ident, &field.ty); }
                }
            } else {
                fields.insert(field_ident, &field.ty);
            }
        } else {
            fields.insert(field_ident, &field.ty);
        }
    }

    let mut builder_fields = TokenStream2::new();
    let mut fields_instance = TokenStream2::new();

    for (ident, ty) in optional_fields.iter().chain(fields.iter()) {
        let (builder_field, builder_field_instance) =
        (
            quote! { #ident: core::option::Option<#ty>, },
            quote! { #ident: core::option::Option::None, },
        );
        builder_fields.extend(builder_field);
        fields_instance.extend(builder_field_instance);
    }

    for (ident, ty) in vec_fields.iter() {
        let (builder_field, builder_field_instance) =
        (
            quote! { #ident: core::option::Option<std::vec::Vec<#ty>>, },
            quote! { #ident: core::option::Option::None, },
        );
        builder_fields.extend(builder_field);
        fields_instance.extend(builder_field_instance);
    }

    // Setters that have the exact same name as the field and sets the entire field
    let mut builder_setters = TokenStream2::new();

    let mut vec_fields_with_attr = HashMap::<&Ident, TokenStream2>::new();

    for field in fields_named.named.iter() {
        let field_ident = if let Some(field_ident) = &field.ident {
            field_ident
        } else {
            unimplemented!()
        };

        for attr in field.attrs.iter() {
            let attr_expr: Expr = attr.parse_args().unwrap();
            let builder_fn_stream = if let Expr::Assign(ref each_expr) = attr_expr {
                if let Expr::Path(ref expr_path) = *each_expr.left {
                    if let Some(path_seg) = expr_path.path.segments.first() {
                        if path_seg.ident == "each" {
                            if let Expr::Lit(ref expr) = each_expr.right.as_ref() {
                                if let Lit::Str(lit_str) = &expr.lit {
                                    let token: Ident = lit_str.parse().unwrap();
                                    if let Some(ty) = vec_fields.get(field_ident) {
                                        let mut token_stream = quote! {
                                            pub fn #token(&mut self, #token: #ty) -> &mut Self {
                                                if let core::option::Option::Some(inner_value) =
                                                    self.#field_ident.as_mut()
                                                {
                                                    inner_value.push(#token)
                                                } else {
                                                    self.#field_ident =
                                                        core::option::Option::Some(std::vec![#token]);
                                                }
                                                self
                                            }
                                        };

                                        if token.to_string() != field_ident.to_string() {
                                            token_stream.extend(
                                                quote! {
                                                    pub fn #field_ident(
                                                        &mut self,
                                                        #field_ident: std::vec::Vec<#ty>,
                                                    ) -> &mut Self {
                                                        self.#field_ident =
                                                            core::option::Option::Some(#field_ident);
                                                        self
                                                    }
                                                }
                                            );
                                        }
                                        token_stream
                                    } else {
                                        Err(syn::Error::new(
                                            each_expr.span(),
                                            "expected `builder(each = \"...\")`",
                                        )).unwrap_or_else(syn::Error::into_compile_error).into()
                                    }
                                } else {
                                    Err(syn::Error::new(
                                        each_expr.span(),
                                        "expected `builder(each = \"...\")`",
                                    )).unwrap_or_else(syn::Error::into_compile_error).into()
                                }
                            } else {
                                Err(syn::Error::new(
                                    each_expr.span(),
                                    "expected `builder(each = \"...\")`",
                                )).unwrap_or_else(syn::Error::into_compile_error).into()
                            }
                        } else {
                            Err(syn::Error::new(
                                each_expr.span(),
                                "expected `builder(each = \"...\")`",
                            )).unwrap_or_else(syn::Error::into_compile_error).into()
                        }
                    } else {
                        Err(syn::Error::new(
                            each_expr.span(),
                            "expected `builder(each = \"...\")`",
                        )).unwrap_or_else(syn::Error::into_compile_error).into()
                    }
                } else {
                    Err(syn::Error::new(
                        each_expr.span(),
                        "expected `builder(each = \"...\")`",
                    )).unwrap_or_else(syn::Error::into_compile_error).into()
                }
            } else {
                Err(syn::Error::new(
                    attr_expr.span(),
                    "expected `builder(each = \"...\")`",
                )).unwrap_or_else(syn::Error::into_compile_error).into()
            };
            vec_fields_with_attr.insert(field_ident, builder_fn_stream);
        }
    }

    for (ident, ty) in fields.iter().chain(optional_fields.iter()) {
        // Check if the ident of the field is the same as the literal in the attribute
        let builder_setter = quote! {
            fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = core::option::Option::Some(#ident);
                self
            }
        };

        builder_setters.extend(builder_setter);
    }

    for (ident, ty) in vec_fields.iter() {
        let vec_setter = if let Some(value) = vec_fields_with_attr.remove(ident) {
            value
        } else {
            quote! {
                fn #ident(&mut self, #ident: Vec<#ty>) -> &mut Self {
                    self.#ident = core::option::Option::Some(#ident);
                    self
                }
            }
        };

        builder_setters.extend(vec_setter);
    }

    let mut original_fields = TokenStream2::new();

    for (ident, _ty) in fields.iter() {
        let field = quote! {
            #ident: self.#ident.take().ok_or("Field is None".to_string())?,
        };

        original_fields.extend(field);
    }

    let mut original_optional_fields = TokenStream2::new();

    for (ident, _ty) in optional_fields.iter() {
        let field = quote! {
            #ident: self.#ident.take(),
        };

        original_optional_fields.extend(field);
    }

    let mut vec_original_fields = TokenStream2::new();

    for (ident, _ty) in vec_fields.iter() {
        let field = quote! {
            #ident: self.#ident.take().unwrap_or(vec![]),
        };

        vec_original_fields.extend(field);
    }

    let builder_ident = format_ident!("{}Builder", ident);

    let tokens = quote! {
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #fields_instance
                }
            }
        }

        #[derive(Debug, Default)]
        pub struct #builder_ident {
            #builder_fields
        }

        impl #builder_ident {
            #builder_setters

            pub fn build(
                &mut self
            ) -> core::result::Result<#ident, std::boxed::Box<dyn std::error::Error>> {
                core::result::Result::Ok( #ident {
                    #original_fields
                    #original_optional_fields
                    #vec_original_fields
                })
            }
        }
    }.into();

    tokens
}
