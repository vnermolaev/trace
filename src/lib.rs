#![feature(trace_macros)]

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod args;

use quote::{quote, ToTokens};
use std::cmp::Ordering;
use std::str::FromStr;
use syn::{
    parse::{Parse, Parser},
    parse_quote,
};

#[proc_macro_attribute]
pub fn trace(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let raw_args = syn::parse_macro_input!(args as syn::AttributeArgs);

    let args = match args::Args::from_raw_args(raw_args) {
        Ok(args) => args,
        Err(errors) => {
            return errors
                .iter()
                .map(syn::Error::to_compile_error)
                .collect::<proc_macro2::TokenStream>()
                .into()
        }
    };

    let output = if let Ok(item) = syn::Item::parse.parse(input.clone()) {
        expand_item(&args, item)
    }
    // else if let Ok(impl_item) = syn::ImplItem::parse.parse(input.clone()) {
    //     expand_impl_item(&args, impl_item)
    // }
    else {
        let input2 = proc_macro2::TokenStream::from(input);
        syn::Error::new_spanned(input2, "expected one of: `fn`, `impl`, `mod`").to_compile_error()
    };

    output.into()
}

#[derive(Clone, Copy)]
enum AttrApplied {
    Directly,
    Indirectly,
}

fn expand_item(args: &args::Args, mut item: syn::Item) -> proc_macro2::TokenStream {
    transform_item(args, AttrApplied::Directly, &mut item);

    match item {
        syn::Item::Fn(_) | syn::Item::Mod(_) | syn::Item::Impl(_) => item.into_token_stream(),
        _ => syn::Error::new_spanned(item, "#[trace] is not supported for this item")
            .to_compile_error(),
    }
}

fn expand_impl_item(args: &args::Args, mut impl_item: syn::ImplItem) -> proc_macro2::TokenStream {
    transform_impl_item(args, AttrApplied::Directly, &mut impl_item);

    match impl_item {
        syn::ImplItem::Method(_) => impl_item.into_token_stream(),
        _ => syn::Error::new_spanned(impl_item, "#[trace] is not supported for this impl item")
            .to_compile_error(),
    }
}

fn transform_item(args: &args::Args, attr_applied: AttrApplied, item: &mut syn::Item) {
    match *item {
        syn::Item::Fn(ref mut item_fn) => transform_fn(args, attr_applied, item_fn),
        syn::Item::Mod(ref mut item_mod) => transform_mod(args, attr_applied, item_mod),
        syn::Item::Impl(ref mut item_impl) => transform_impl(args, attr_applied, item_impl),
        _ => (),
    }
}

fn transform_fn(args: &args::Args, attr_applied: AttrApplied, item_fn: &mut syn::ItemFn) {
    println!("FN");
    item_fn.block = Box::new(construct_traced_block(
        &args,
        attr_applied,
        &item_fn.ident,
        &item_fn.decl,
        &item_fn.block,
    ));
}

fn transform_mod(args: &args::Args, attr_applied: AttrApplied, item_mod: &mut syn::ItemMod) {
    assert!(
        (item_mod.content.is_some() && item_mod.semi.is_none())
            || (item_mod.content.is_none() && item_mod.semi.is_some())
    );

    if item_mod.semi.is_some() {
        unimplemented!();
    }

    if let Some((_, items)) = item_mod.content.as_mut() {
        items.iter_mut().for_each(|item| {
            if let AttrApplied::Directly = attr_applied {
                match *item {
                    syn::Item::Fn(syn::ItemFn { ref ident, .. })
                    | syn::Item::Mod(syn::ItemMod { ref ident, .. }) => match args.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                            return;
                        }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => {
                            return;
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }

            transform_item(args, AttrApplied::Indirectly, item);
        });

        items.insert(
            0,
            parse_quote! {
                ::std::thread_local! {
                    static DEPTH: ::std::cell::Cell<usize> = ::std::cell::Cell::new(0);
                }
            },
        );
    }
}

fn transform_impl(args: &args::Args, attr_applied: AttrApplied, item_impl: &mut syn::ItemImpl) {
    println!("IMPL");
    item_impl.items.iter_mut().for_each(|impl_item| {
        if let syn::ImplItem::Method(ref mut impl_item_method) = *impl_item {
            println!("{:?}", impl_item_method.into_token_stream().to_string());

            if !impl_item_method.attrs.is_empty() {
                let pos = impl_item_method.attrs.iter().position(|attr| {
                    attr.path.segments[0].ident.to_string() == "trace".to_string()
                });

                if let Some(pos) = pos {
                    let trace_macro = impl_item_method.attrs.remove(pos);
                    println!("{:?}", trace_macro.tts.to_string());

                    // let args: proc_macro::TokenStream = trace_macro.tts.into();
                    let str = trace_macro.tts.to_string();
                    let str = &str[1..str.len() - 1];
                    println!("{:?}", str.to_string());

                    let args = proc_macro::TokenStream::from_str(str).unwrap();

                    let raw_args = syn::parse_macro_input::parse::<syn::AttributeArgs>(args);
                    println!("{}", raw_args.is_ok());
                }
            }

            if let AttrApplied::Directly = attr_applied {
                let ident = &impl_item_method.sig.ident;

                match args.filter {
                    args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                        return;
                    }
                    args::Filter::Disable(ref idents) if idents.contains(ident) => {
                        return;
                    }
                    _ => (),
                }
            }

            impl_item_method.block = construct_traced_block(
                &args,
                AttrApplied::Indirectly,
                &impl_item_method.sig.ident,
                &impl_item_method.sig.decl,
                &impl_item_method.block,
            );
        }
    });
}

fn transform_impl_item(
    args: &args::Args,
    attr_applied: AttrApplied,
    impl_item: &mut syn::ImplItem,
) {
    // Will probably add more cases in the future
    #[cfg_attr(feature = "cargo-clippy", allow(single_match))]
    match *impl_item {
        syn::ImplItem::Method(ref mut impl_item_method) => {
            transform_method(args, attr_applied, impl_item_method)
        }
        _ => (),
    }
}

fn transform_method(
    args: &args::Args,
    attr_applied: AttrApplied,
    impl_item_method: &mut syn::ImplItemMethod,
) {
    println!("METHOD");
    impl_item_method.block = construct_traced_block(
        &args,
        attr_applied,
        &impl_item_method.sig.ident,
        &impl_item_method.sig.decl,
        &impl_item_method.block,
    );
}

fn construct_traced_block(
    args: &args::Args,
    attr_applied: AttrApplied,
    ident: &proc_macro2::Ident,
    fn_decl: &syn::FnDecl,
    original_block: &syn::Block,
) -> syn::Block {
    let arg_idents = {
        let mut arg_idents = extract_arg_idents(args, attr_applied, &fn_decl);

        arg_idents.sort_by(|a, b| match (a, b) {
            (IdentWrapper::Ident(..), IdentWrapper::Empty) => Ordering::Less,
            (IdentWrapper::Ident(..), IdentWrapper::Ident(..)) => Ordering::Equal,
            (IdentWrapper::Empty, IdentWrapper::Ident(..)) => Ordering::Greater,
            (IdentWrapper::Empty, IdentWrapper::Empty) => Ordering::Equal,
        });

        arg_idents.dedup_by(|a, b| match (a, b) {
            (IdentWrapper::Empty, IdentWrapper::Empty) => true,
            _ => false,
        });

        arg_idents
    };

    let pretty = if args.pretty { "#" } else { "" };

    let entering_format = {
        let arg_idents_format = arg_idents
            .iter()
            .filter_map(|wrapped| match wrapped {
                IdentWrapper::Empty => Some("...".to_string()),
                IdentWrapper::Ident(arg_ident) => Some(
                    args.args_format
                        .get(arg_ident)
                        .cloned()
                        .map(|fmt| format!("{}: {}", arg_ident, fmt))
                        .unwrap_or_else(|| format!("{}: {{:{}?}}", arg_ident, pretty)),
                ),
            })
            .collect::<Vec<_>>()
            .join("\n\t");

        let sep = if arg_idents.is_empty() { "" } else { "\n\t" };

        format!("{}{}{}{}", args.prefix_enter, ident, sep, arg_idents_format)
    };

    let arg_idents = arg_idents
        .into_iter()
        .filter_map(|wrapped| {
            if let IdentWrapper::Ident(arg_ident) = wrapped {
                Some(arg_ident)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let return_var = "res".to_string();
    let exiting_format = args
        .args_format
        .iter()
        .find(|(arg_ident, _)| arg_ident.to_string() == return_var)
        .map(|(_, fmt)| format!("{}{}\n\t{}: {}", args.prefix_exit, ident, return_var, fmt))
        .unwrap_or_else(|| {
            format!(
                "{}{}\n\t{}: {{:{}?}}",
                args.prefix_exit, ident, return_var, pretty
            )
        });

    let pause_stmt = if args.pause {
        quote! {{
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            stdin.lock().lines().next();
        }}
    } else {
        quote!()
    };

    let printer = if args.logging {
        quote! { log::trace! }
    } else {
        quote! { println! }
    };

    parse_quote! {{
        #printer(#entering_format, #(#arg_idents,)*);
        #pause_stmt
        let mut fn_closure = move || #original_block;
        let fn_return_value = fn_closure();
        #printer(#exiting_format, fn_return_value);
        #pause_stmt
        fn_return_value
    }}
}

fn extract_arg_idents(
    args: &args::Args,
    attr_applied: AttrApplied,
    fn_decl: &syn::FnDecl,
) -> Vec<IdentWrapper> {
    fn process_pat(
        args: &args::Args,
        attr_applied: AttrApplied,
        pat: &syn::Pat,
        arg_idents: &mut Vec<IdentWrapper>,
    ) {
        match *pat {
            syn::Pat::Ident(ref pat_ident) => {
                let ident = &pat_ident.ident;

                if let AttrApplied::Directly = attr_applied {
                    match args.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                            arg_idents.push(IdentWrapper::Empty);
                            return;
                        }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => {
                            arg_idents.push(IdentWrapper::Empty);
                            return;
                        }
                        _ => (),
                    }
                };

                arg_idents.push(IdentWrapper::Ident(ident.clone()));
            }
            syn::Pat::Tuple(ref pat_tuple) => {
                pat_tuple.front.iter().for_each(|pat| {
                    process_pat(args, attr_applied, pat, arg_idents);
                });
            }
            _ => unimplemented!(),
        }
    }

    let mut arg_idents = vec![];

    for input in &fn_decl.inputs {
        match *input {
            syn::FnArg::SelfRef(_) | syn::FnArg::SelfValue(_) => (), // ignore `self`
            syn::FnArg::Captured(ref arg_captured) => {
                process_pat(args, attr_applied, &arg_captured.pat, &mut arg_idents);
            }
            syn::FnArg::Inferred(_) | syn::FnArg::Ignored(_) => unimplemented!(),
        }
    }

    arg_idents
}

enum IdentWrapper {
    Empty,
    Ident(proc_macro2::Ident),
}
