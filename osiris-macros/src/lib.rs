use std::mem::replace;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Block, Error, ItemFn, Result};

#[proc_macro_attribute]
pub fn main(_: TokenStream, input: TokenStream) -> TokenStream {
    let AsyncMain { item } = parse_macro_input!(input);
    let item = transform(item);
    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn test(_: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input);
    let item = transform(item);
    quote!(#[test] #item).into()
}

fn transform(mut item: ItemFn) -> ItemFn {
    let block = item.block.clone();
    let new_block: Block = parse_quote!({osiris::block_on(async { #block }).unwrap()});
    let _ = replace(&mut item.block, Box::new(new_block));
    item.sig.asyncness = None;
    item
}

struct AsyncMain {
    item: ItemFn,
}

impl Parse for AsyncMain {
    fn parse(input: ParseStream) -> Result<Self> {
        let item: ItemFn = input.parse()?;

        let is_async = item.sig.asyncness.is_some();
        let is_main = item.sig.ident == "main";

        if !is_async && !is_main {
            return Err({
                Error::new(item.sig.span(),
                "expected `async fn main`. help: rename this function to `main` and make it `async`.")
            });
        }

        if !is_async {
            return Err({
                Error::new(
                    item.sig.span(),
                    "expected `async fn main()`. help: make this function `async`.",
                )
            });
        }
        if !is_main {
            return Err({
                Error::new(
                    item.sig.span(),
                    "expected `async fn main()`. help: rename this function to `main`.",
                )
            });
        }

        Ok(AsyncMain { item })
    }
}
