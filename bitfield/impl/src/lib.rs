use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::Literal as Literal2;
use proc_macro2::Span as Span2;
use proc_macro2::Punct as Punct2;
use proc_macro2::Spacing as Spacing2;
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item_struct = syn::parse::<syn::ItemStruct>(input).expect("Not a struct");

    let zero_lit = Literal2::usize_unsuffixed(0);
    let mut start_range = TokenStream2::new();
    start_range.extend(quote! { #zero_lit });
    let mut size_tk_stream = TokenStream2::new();
    size_tk_stream.extend(quote! { #zero_lit });
    let mut fns_tk_stream = TokenStream2::new();

    if let syn::Fields::Named(named_fields) = item_struct.fields {
        for field in named_fields.named.iter() {
            let ty = &field.ty;
            let btype_size: syn::Path = syn::parse_quote!{#ty::BITS};
            size_tk_stream.extend(quote! { + });
            size_tk_stream.extend(quote! { #btype_size });

            if let Some(ident) = &field.ident {
                let fn_get_ident = format_ident!("get_{}", ident);
                let fn_set_ident = format_ident!("set_{}", ident);
                //let ty_to_return: syn::Path = syn::parse_quote!{#ty::SizeType};
                let dot1 = Punct2::new('.', Spacing2::Joint);
                let dot2 = Punct2::new('.', Spacing2::Joint);
                let start_range_byte = quote!{ (#start_range) / u8::BITS as usize };
                let _range = quote! {
                    #start_range_byte #dot1 #dot2
                };
                let fn_get = quote! {
                    pub fn #fn_get_ident(&self) -> u64 {
                        let mut value = 0u64;
                        for bit in #start_range..#size_tk_stream {
                            let byte_idx = bit / u8::BITS as usize;
                            let bit_pos = u8::BITS as usize - 1 - (bit % u8::BITS as usize);
                            value =
                                value | (((self.data[byte_idx] >> bit_pos as u8) & 1) as u64) << (bit - (#start_range));
                        }
                        value
                    }
                    pub fn #fn_set_ident(&mut self, value: u64) {
                        for bit in (#start_range..#size_tk_stream) {
                            let byte_idx = bit / u8::BITS as usize;
                            let bit_pos = u8::BITS as usize - 1 - (bit % u8::BITS as usize);
                            let mask = ((self.data[byte_idx] >> bit_pos) & 1);
                            let to_replace_mask =
                                (((value >> (bit - (#start_range))) & 1) << bit_pos) as u8;
                            self.data[byte_idx] |= to_replace_mask;
                        }
                        println!("Data {:?}", self.data);
                    }
                };
                start_range = size_tk_stream.clone();

                fns_tk_stream.extend(fn_get);
            }
        }
    }

    let struct_ident = item_struct.ident;
    let size_in_bits = quote! { (#size_tk_stream) / u8::BITS as usize };

    let const_mod8 = quote! {
        const mod8 = (#size_tk_stream) % u8::BITS as usize;
    };

    let token_stream = quote! {
        #const_mod8

        #[repr(C)]
        pub struct #struct_ident {
            data: [u8; #size_in_bits],
        }

        impl #struct_ident {
            pub fn new() -> Self {
                #struct_ident {
                    data: [0; #size_in_bits]
                }
            }

            #fns_tk_stream
        }

        #[cfg(mod8 = 0)]
        fn get_ripped() {}
    };

    token_stream.into()
}

#[proc_macro]
pub fn generate_btypes(_input: TokenStream) -> TokenStream {
    let mut token_stream = TokenStream2::new();

    for idx in 1..=64 {
        let btype_ident = format_ident!("B{}", idx);
        let _size_type_token = size_to_primitive(idx);
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

// Takes a size expressed in bits and returns a type that is big enough to able to contain it
fn size_to_primitive(size: usize) -> TokenStream2 {
    let byte_size = u8::BITS as usize;
    let numerator = size / byte_size;
    let remainder = size % byte_size;

    let tk_stream2 = if remainder == 0 {
        match numerator {
            1 => quote! { u8 },
            2 => quote! { u16 },
            3 => quote! { u32 },
            4 => quote! { u32 },
            5 => quote! { u64 },
            6 => quote! { u64 },
            7 => quote! { u64 },
            8 => quote! { u64 },
            _ => syn::Error::new(
                Span2::call_site(),
                format!("Invalid size of bits {}", numerator)
            ).into_compile_error(),
        }
    } else {
        match numerator {
            0 => quote! { u8 },
            1 => quote! { u16 },
            2 => quote! { u32 },
            3 => quote! { u32 },
            4 => quote! { u64 },
            5 => quote! { u64 },
            6 => quote! { u64 },
            7 => quote! { u64 },
            _ => syn::Error::new(
                Span2::call_site(),
                format!("Invalid size of bits {}", numerator)
            ).into_compile_error(),
        }
    };

    tk_stream2.into()
}

