use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::mem::replace;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Block, Error, Expr, ItemFn, Result, ReturnType, Token};

#[proc_macro_attribute]
pub fn main(args: TokenStream, input: TokenStream) -> TokenStream {
    let AsyncMain { item } = parse_macro_input!(input);
    let Args { scale, restart } = parse_macro_input!(args);
    let item = transform(item, scale, restart);
    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn test(_: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input);
    let item = transform(item, parse_quote!(1), parse_quote!(false));
    quote!(#[test] #item).into()
}

fn transform(mut item: ItemFn, scale: Expr, restart: Expr) -> ItemFn {
    let block = item.block.clone();

    let ty = match item.sig.output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => ty,
    };

    let new_block: Block = parse_quote!({
        osiris::__priv::run(#scale, #restart, || -> std::io::Result<#ty> {
            osiris::block_on(async { #block })
        })
    });
    let _ = replace(&mut item.block, Box::new(new_block));
    item.sig.output = parse_quote!(-> std::process::ExitCode);
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

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Args {
                scale: parse_quote!(1),
                restart: parse_quote!(false),
            });
        }

        let mut scale = None;
        let mut restart = None;

        while !input.is_empty() {
            if scale.is_some() || restart.is_some() {
                let _: Token![,] = input.parse()?;
            }

            let ident: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value: Expr = input.parse()?;

            if ident == "scale" && scale.is_none() {
                scale = Some(value);
            } else if ident == "restart" && restart.is_none() {
                restart = Some(value);
            } else if ident != "scale" && ident != "restart" {
                return Err(Error::new(
                    ident.span(),
                    "unsupported argument. Supported arguments are: \"scale\" and \"restart\".",
                ));
            } else {
                return Err(Error::new(
                    ident.span(),
                    format!("repeated argument \"{ident}\"."),
                ));
            }
        }
        let scale = scale.unwrap_or(parse_quote!(1));
        let restart = restart.unwrap_or(parse_quote!(false));

        Ok(Args { scale, restart })
    }
}
