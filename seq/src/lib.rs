use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::TokenTree as TokenTree2;
use proc_macro2::Delimiter as Delimiter2;
use proc_macro2::Ident as Ident2;
use proc_macro2::Group as Group2;
use syn::parse::{Parse, ParseStream};
use syn::Token;
use quote::{quote, format_ident};

#[derive(Debug)]
struct SeqHeader {
    ident: syn::Ident,
    range_start: syn::LitInt,
    range_stop: syn::LitInt,
    inclusive: bool,
}

impl Parse for SeqHeader {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident2 = input.parse()?;
        input.parse::<Token![in]>()?;
        let range_start = input.parse()?;
        input.parse::<Token![..]>()?;

        // We want to look ahead and check if the following token is an equal sign `=` such that
        // we can implement an inclusive range
        let lookahead = input.lookahead1();
        let inclusive = if lookahead.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            true
        } else {
            false
        };
        let range_stop = input.parse()?;

        Ok(
            SeqHeader {
                ident: ident.clone(),
                range_start,
                range_stop,//: syn::LitInt::new("256", ident.span()),
                inclusive,
            }
        )
    }
}

fn fill_token_stream(token_stream: TokenStream2, ident: Ident2, idx: u64) -> TokenStream2 {
    let mut new_tk_stream = TokenStream2::new();

    let mut token_iter = token_stream.into_iter().peekable();

    while let Some(token) = token_iter.next() {
        let token_tree = match token {
            TokenTree2::Group(ref group) => {
                let group_stream = fill_token_stream(group.stream(), ident.clone(), idx);
                let mut new_group =
                    Group2::new(
                        group.delimiter(),
                        group_stream,
                    );
                new_group.set_span(token.span());
                TokenTree2::Group(new_group)
            }
            TokenTree2::Ident(ref inner_ident) => {
                let mut curr_ident = inner_ident.clone();
                // Whenever we find an ident, we need to peek and parse any potential `Tilde` after
                // it, such that we can return a correct identfier.
                while let Some(TokenTree2::Punct(punct)) = token_iter.peek() {
                    // Check if the punctuation is a Tilde
                    if punct.as_char() == '~' {
                        token_iter.next();
                        if let Some(TokenTree2::Ident(next_ident)) = token_iter.peek() {
                            if next_ident.to_string() == ident.to_string() {
                                curr_ident = format_ident!("{}{}", curr_ident, idx);
                            } else {
                                curr_ident = format_ident!("{}{}", curr_ident, next_ident);
                            };
                            // Since we got a result from peeking at the next 2 instances, we
                            // advance 2 positions
                            token_iter.next();
                        }
                    } else {
                        break;
                    }
                }
                let mut curr_tree = if ident.to_string() == inner_ident.to_string() {
                    let idx_lit = proc_macro2::Literal::u64_unsuffixed(idx);
                    TokenTree2::Literal(idx_lit)
                } else {
                    TokenTree2::Ident(curr_ident)
                };
                curr_tree.set_span(token.span());
                curr_tree

            }
            _ => token
        };
        new_tk_stream.extend(quote! { #token_tree });
    }

    new_tk_stream
}

// Parses the stream for the `#(...)`
fn parse_for_repeat_token(
    original_stream: TokenStream2,
    ident: Ident2,
    range_start: u64,
    range_stop: u64,
    special_repeat_section: &mut bool,
) -> TokenStream2 {
    let mut new_stream = TokenStream2::new();
    let mut token_iter = original_stream.into_iter().peekable();

    while let Some(token) = token_iter.next() {
        match token {
            TokenTree2::Punct(ref punct_start) => {
                if punct_start.as_char() == '#' {
                    if let Some(token2) = token_iter.next() {
                        if let TokenTree2::Group(ref group) = token2 {
                            if group.delimiter() == Delimiter2::Parenthesis {
                                if let Some(TokenTree2::Punct(ref punct_stop)) = token_iter.peek() {
                                    if punct_stop.as_char() == '*' {
                                        // At this point we know that the group we just parsed it the group
                                        // we need to repet as the token stream, so we do add it to the
                                        // original stream.
                                        *special_repeat_section = true;
                                        for idx in range_start..range_stop {
                                            let token_stream = fill_token_stream(group.stream(), ident.clone(), idx);
                                            new_stream.extend(token_stream);
                                        }
                                        token_iter.next();
                                    } else {
                                        let token2 = TokenTree2::Group(group.clone());
                                        let token3 = TokenTree2::Punct(punct_stop.clone());
                                        new_stream.extend(quote!{#token});
                                        new_stream.extend(quote!{#token2});
                                        new_stream.extend(quote!{#token3});
                                    }
                                } else {
                                    new_stream.extend(quote!{#token});
                                    new_stream.extend(quote!{#token2});
                                }
                            } else {
                                new_stream.extend(quote!{#token});
                                new_stream.extend(quote!{#token2});
                            }
                        } else {
                            new_stream.extend(quote!{#token});
                            new_stream.extend(quote!{#token2});
                        }
                    } else {
                        new_stream.extend(quote!{#token});
                    }
                } else {
                    new_stream.extend(quote!{#token});
                }
            }
            TokenTree2::Group(group) => {
                let new_stream_ext = parse_for_repeat_token(
                    group.stream(),
                    ident.clone(),
                    range_start,
                    range_stop,
                    special_repeat_section,
                );
                let new_stream_group = Group2::new(group.delimiter(), new_stream_ext);
                new_stream.extend(quote!{#new_stream_group});
            }
            _ => {
                if new_stream.is_empty() {
                    new_stream.extend(quote!{#token});
                } else {
                    new_stream.extend(quote!{#token});
                }
            }
        }
    };

    new_stream
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let mut token_iter = TokenStream2::from(input).into_iter();

    let mut header = TokenStream2::new();
    let mut loop_body = None;

    while let Some(token) = token_iter.next() {
        match token {
            TokenTree2::Group(ref body) => {
                if body.delimiter() == Delimiter2::Brace {
                    loop_body = Some(body.stream());
                } else {
                    header.extend(quote!{#token});
                }
            }
            _ => header.extend(quote!{#token}),
        }
    }

    let code_token_stream = loop_body.expect("Failed to get loop body");

    let SeqHeader {
        ident,
        range_start,
        range_stop,
        inclusive,
    } = syn::parse::<SeqHeader>(header.into()).expect("Failed to read header");

    // Convert the literal integers into actual integers
    let range_start = range_start.base10_parse::<u64>().expect("Failed to convert");
    let range_stop = range_stop.base10_parse::<u64>().expect("Failed to convert");
    let range_stop = if inclusive {
        range_stop + 1
    } else {
        range_stop
    };

    let mut special_repeat_section = false;
    let mut out_stream = TokenStream2::new();

    let repeated_section_stream =
        parse_for_repeat_token(code_token_stream.clone(), ident.clone(), range_start, range_stop, &mut special_repeat_section);

    if !special_repeat_section {
        for idx in range_start..range_stop {
            let token_stream = fill_token_stream(code_token_stream.clone(), ident.clone(), idx);
            out_stream.extend(token_stream);
        }
    } else {
        out_stream.extend(repeated_section_stream);
    }
    //TokenStream::new()
    out_stream.into()
    //quote! {#pre, #out_token_stream,#post,}.into()
}
