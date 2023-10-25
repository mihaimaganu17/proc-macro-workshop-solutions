use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::Literal as Literal2;
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item_struct = syn::parse::<syn::ItemStruct>(input).expect("Not a struct");

    let mut size_tk_stream = TokenStream2::new();

    if let syn::Fields::Named(named_fields) = item_struct.fields {
        for field in named_fields.named.iter() {
            if let syn::Type::Path(type_path) = &field.ty {
                let path = &type_path.path;
                let btype_size: syn::Path = syn::parse_quote!{#path::BITS};
                if size_tk_stream.is_empty() {
                    size_tk_stream.extend(quote! { #btype_size });
                } else {
                    size_tk_stream.extend(quote! { + });
                    size_tk_stream.extend(quote! { #btype_size });
                }
            }
        }
    }

    let struct_ident = item_struct.ident;

    let token_stream = quote! {
        #[repr(C)]
        pub struct #struct_ident {
            data: [u8; (#size_tk_stream) / u8::BITS as usize],
        }
    };

    token_stream.into()
}

#[proc_macro]
pub fn generate_btypes(_input: TokenStream) -> TokenStream {
    let mut token_stream = TokenStream2::new();

    for idx in 1..=64 {
        let btype_ident = format_ident!("B{}", idx);
        let btype = quote! { pub enum #btype_ident {} };
        let btype_const = Literal2::usize_suffixed(idx);
        let btype_trait_impl = quote! {
            impl Specifier for #btype_ident {
                const BITS: usize = #btype_const;
            }
        };
        token_stream.extend(btype);
        token_stream.extend(btype_trait_impl);
    }

    token_stream.into()
}

