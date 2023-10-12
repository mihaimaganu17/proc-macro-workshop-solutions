use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    spanned::Spanned,
    LitStr,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let mut syn_tree: syn::DeriveInput = syn::parse(input).unwrap();

    let struct_ident = syn_tree.ident;

    let named_fields = if let syn::Data::Struct(struct_tree) = syn_tree.data {
        if let syn::Fields::Named(named_fields) = struct_tree.fields {
            named_fields.named
        } else {
            unimplemented!();
        }
    } else {
        unimplemented!();
    };

    for generic in syn_tree.generics.params.iter_mut() {
        if let syn::GenericParam::Type(ref mut generic_ty) = generic {
            // For this generic, we have to check if there are just `PhantomData` fields in the
            // structure that contains it. In this case, we avoid to add the bound generic.

            // First we assume that the generic type is present in the structure and it is not
            // wrapped around the `PhantomData` type.
            let mut generic_in_struct = false;

            for field in named_fields.iter() {
                if field.ty == syn::parse_quote!(#generic_ty) {
                    generic_in_struct = true;
                }
            }

            // If we could not find the generic inside the struct by itself, we do not add the
            // bound and move on to the next structure.
            if !generic_in_struct {
                continue;
            }
            // Add the required `Debug` bound to the structure
            generic_ty.bounds.push(syn::parse_quote!(std::fmt::Debug));
        }
    }

    // Identify if the generics from the struct definitions have the `Debug` trait bound
    let (impl_generics, ty_generics, where_clause) = syn_tree.generics.split_for_impl();

    let mut fields_write_calls = TokenStream2::new();

    for field in named_fields.iter() {
        let ident = if let Some(ident) = &field.ident {
            ident
        } else {
            unimplemented!();
        };

        let debug_ident = syn::LitStr::new(&ident.to_string(), ident.span());

        // Construct the debug formatting as standard or by passing the custom attribute
        let debug_fmt = if let Some(attr) = field.attrs.first() {
            // Parse the attribute into an expression that we can evaluate
            if let syn::Meta::NameValue(syn::MetaNameValue { path: _, eq_token: _, value} ) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        lit_str.clone()
                    } else {
                        LitStr::new("{:?}", attr.span())
                    }
                } else {
                    LitStr::new("{:?}", attr.span())
                }
            } else {
                LitStr::new("{:?}", attr.span())
            }
        } else {
            LitStr::new("{:?}", field.span())
        };

        fields_write_calls.extend(
            quote! {
                .field(#debug_ident, &std::format_args!(#debug_fmt, &self.#ident))
            }
        );
    }

    let debug_ident = LitStr::new(&struct_ident.to_string(), struct_ident.span());

    let token_stream = quote! {
        impl #impl_generics std::fmt::Debug for #struct_ident #ty_generics #where_clause {
            fn fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(#debug_ident)
                    #fields_write_calls
                    .finish()
            }
        }
    }.into();

    token_stream
}
