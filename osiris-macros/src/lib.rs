use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::mem::replace;

use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    parenthesized, parse_macro_input, parse_quote, Block, Error, Expr, ItemFn, Result, Token,
};

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

struct Args {
    scale: Expr,
    restart: Expr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = Args {
            scale: parse_quote!(1),
            restart: parse_quote!(false),
        };

        if input.is_empty() {
            return Ok(args);
        }
        let content;
        parenthesized!(content in input);

        let mut scale = false;
        let mut restart = false;
        while !content.is_empty() {
            let ident: Ident = content.parse()?;
            let _: Token![=] = content.parse()?;
            let expr: Expr = content.parse()?;
            if ident == "scale" {
                args.scale = expr;
                scale = true;
            } else if ident == "restart" {
                args.restart = expr;
                restart = true;
            } else if scale {
                return Err(Error::new(
                    ident.span(),
                    "argument `scale` is defined mupltiple times",
                ));
            } else if restart {
                return Err(Error::new(
                    ident.span(),
                    "argument `restart` defined multiple times",
                ));
            } else {
                return Err(Error::new(
                    ident.span(),
                    format!("unknown argument \"{ident}\". Supported arguments are: `scale` and `restart`."),
                ));
            }
            let Ok(_): Result<Token![,]> = content.parse() else {
                break
            };
        }

        if !content.is_empty() {
            return Err(content.error("expected end of input"));
        }

        Ok(args)
    }
}

impl Parse for AsyncMain {
    fn parse(input: ParseStream) -> Result<Self> {
        let item: ItemFn = input.parse()?;
        let is_async = item.sig.asyncness.is_some();
        if !is_async {
            return Err({
                Error::new(
                    item.sig.span(),
                    "expected `async fn main()`. help: make this function `async`.",
                )
            });
        }
        Ok(AsyncMain { item })
    }
}
