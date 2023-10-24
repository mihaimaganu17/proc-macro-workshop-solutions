use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::Span as Span2;
use syn::{visit_mut::{self, VisitMut}, Item};
use quote::quote;
use syn::spanned::Spanned;
use syn::punctuated::Pair;

fn parse_enum(input: &TokenStream) -> syn::Result<TokenStream2> {
    let item: Item = syn::parse(input.clone())?;

    match item {
        Item::Enum(ref item_enum) => {
            for idx in 0..item_enum.variants.len() {
                let mut variant_iter = item_enum.variants.iter().skip(idx);
                if let Some(variant) = variant_iter.next() {
                    let prev_ident = &variant.ident;
                    while let Some(curr_variant) = variant_iter.next() {
                        if curr_variant.ident.lt(&prev_ident) {
                            return Err(syn::Error::new(
                                curr_variant.ident.span(),
                                format!("{} should sort before {}", curr_variant.ident, prev_ident),
                            ));
                        }
                    }
                }
            }
            Ok(TokenStream2::new())
        }
        _ => Err(syn::Error::new(Span2::call_site(), "expected enum or match expression")),
    }
}

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;

    let mut token_stream = parse_enum(&input).unwrap_or_else(syn::Error::into_compile_error);

    if !token_stream.is_empty() {
        token_stream.extend(TokenStream2::from(input));
        let tk: TokenStream = token_stream.into();
        tk
    } else {
        input
    }
}

struct MatchReplace;

impl VisitMut for MatchReplace {
    fn visit_expr_match_mut(&mut self, expr_match: &mut syn::ExprMatch) {
        let len = expr_match.attrs.len();
        for idx in 0..len {
            let attr = expr_match.attrs.remove(idx);
            if let syn::Meta::Path(path) = &attr.meta {
                if let Some(seg) = path.segments.last() {
                    if seg.ident.to_string() == "sorted".to_string() {
                        let token_stream = check_match_arms(&expr_match.arms)
                            .unwrap_or_else(syn::Error::into_compile_error);
                        if !token_stream.is_empty() {
                            expr_match.expr = syn::parse_quote!(#token_stream);
                        }
                    }
                }
            }
        }
        visit_mut::visit_expr_match_mut(self, expr_match);
    }
}

fn check_match_arms(arms: &[syn::Arm]) -> syn::Result<TokenStream2> {
    for idx in 0..arms.len() {
        let mut arm_iter = arms.iter().skip(idx);
        if let Some(prev_arm) = arm_iter.next() {
            if let Some((prev_ident, _span1)) = ident_from_pat(prev_arm.pat.clone()) {
                while let Some(curr_arm) = arm_iter.next() {
                    if let Some((curr_ident, span2)) = ident_from_pat(curr_arm.pat.clone()) {
                        if curr_ident.lt(&prev_ident) {
                            return Err(syn::Error::new(
                                span2,
                                format!("{} should sort before {}", curr_ident, prev_ident),
                            ));
                        }
                    } else {
                        if let syn::Pat::Wild(wild_pat) = &curr_arm.pat {
                            if arm_iter.size_hint().0 > 0 {
                                return Err(syn::Error::new(
                                    wild_pat.span(),
                                    "`_` wildcard should sort last",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(
                                curr_arm.pat.span(),
                                "unsupported by #[sorted]",
                            ));
                        }
                    }
                }
            } else {
                if let syn::Pat::Wild(wild_pat) = &prev_arm.pat {
                    if arm_iter.size_hint().0 > 0 {
                        return Err(syn::Error::new(
                            wild_pat.span(),
                            "`_` wildcard should sort last",
                        ));
                    }
                } else {
                    return Err(syn::Error::new(
                        prev_arm.pat.span(),
                        "unsupported by #[sorted]",
                    ));
                }
            }
        }
    }
    Ok(TokenStream2::new())
}

// Extracts ident from a pattern
fn ident_from_pat(pat: syn::Pat) -> Option<(String, Span2)> {
    let ident = match pat {
        syn::Pat::TupleStruct(ref tuple_struct) => {
            let ident = tuple_struct
                .path
                .segments
                .pairs()
                .fold(
                    String::new(), |acc, pair| {
                match pair {
                    Pair::Punctuated(path, _punct) => {
                        if acc == "" {
                            format!("{}::", path.ident)
                        } else {
                            format!("{}{}::", acc, path.ident)
                        }
                    }
                    Pair::End(path) => {
                        if acc == "" {
                            format!("{}", path.ident)
                        } else {
                            format!("{}{}", acc, path.ident)
                        }
                    }
                }
            });
            Some((ident, tuple_struct.path.span()))
        }
        syn::Pat::Ident(pat_ident) => {
            Some((pat_ident.ident.to_string(), pat_ident.ident.span()))
        }
        syn::Pat::Struct(pat_struct) => {
            let ident = pat_struct.path.segments.iter()
                .fold(String::new(), |acc, x| {
                    if acc == "" {
                        format!("{}", x.ident)
                    } else {
                        format!("{}::{}", acc, x.ident)
                    }
            });
            Some((ident, pat_struct.path.span()))
        }
        /*
        syn::Pat::Wild(pat_wild) => {
            Some(("_".to_string(), pat_wild.span()))
        }
        */
        _ => None
    };
    ident
}

fn parse_fn(input: TokenStream) -> syn::Result<TokenStream2> {
    let item: Item = syn::parse(input).expect("Expected a function item");

    match item {
        Item::Fn(mut item_fn) => {
            MatchReplace.visit_item_fn_mut(&mut item_fn);
            let tk_stream = quote! { #item_fn };
            Ok(tk_stream)
        }
        _ => Err(syn::Error::new(Span2::call_site(), "expected function item")),
    }
}

// The #[sorted::check] macro will expand by looking inside the function to find
// any match-expressions carrying a #[sorted] attribute, checking the order of
// the arms in that match-expression, and then stripping away the inner
// #[sorted] attribute to prevent the stable compiler from refusing to compile
// the code.
#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;

    let token_stream = parse_fn(input).unwrap_or_else(syn::Error::into_compile_error);

    token_stream.into()
}
