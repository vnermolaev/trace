#![feature(trace_macros)]

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod args;

use args::Prefix;
use quote::{quote, ToTokens};
use std::cmp::Ordering;
use std::ops::Deref;
use std::str::FromStr;
use syn::{
    parse::{Parse, Parser},
    parse_quote,
};

const MACRO_NAME: &str = "trace";

#[proc_macro_attribute]
pub fn trace(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let raw_args = syn::parse_macro_input!(args as syn::AttributeArgs);

    let attr = match args::Args::from_raw_args(raw_args) {
        Ok(args) => AttrApplication::Directly(args),
        Err(errors) => {
            return errors
                .iter()
                .map(syn::Error::to_compile_error)
                .collect::<proc_macro2::TokenStream>()
                .into()
        }
    };

    let output = if let Ok(ref mut item) = syn::Item::parse.parse(input.clone()) {
        match transform_item(&[attr], item) {
            Ok(()) => item.into_token_stream(),
            Err(errors) => errors
                .iter()
                .map(syn::Error::to_compile_error)
                .collect::<proc_macro2::TokenStream>(),
        }
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

fn transform_item(attrs: &[AttrApplication], item: &mut syn::Item) -> Result<(), Vec<syn::Error>> {
    match item {
        syn::Item::Fn(ref mut item_fn) => transform_fn(attrs, item_fn),
        syn::Item::Mod(ref mut item_mod) => transform_mod(attrs, item_mod),
        syn::Item::Impl(ref mut item_impl) => transform_impl(attrs, item_impl),
        _ => Err(vec![syn::Error::new_spanned(
            item,
            "#[trace] is not supported for this item",
        )]),
    }
}

// fn expand_impl_item(args: &args::Args, mut impl_item: syn::ImplItem) -> proc_macro2::TokenStream {
//     transform_impl_item(args, AttrApplied::Directly, &mut impl_item);
//
//     match impl_item {
//         syn::ImplItem::Method(_) => impl_item.into_token_stream(),
//         _ => syn::Error::new_spanned(impl_item, "#[trace] is not supported for this impl item")
//             .to_compile_error(),
//     }
// }

fn transform_fn(
    attrs: &[AttrApplication],
    item_fn: &mut syn::ItemFn,
) -> Result<(), Vec<syn::Error>> {
    println!("FN");

    item_fn.block = Box::new(construct_traced_block(
        &attrs,
        &item_fn.ident,
        &item_fn.decl,
        &item_fn.block,
    ));

    Ok(())
}

fn transform_mod(
    attrs: &[AttrApplication],
    item_mod: &mut syn::ItemMod,
) -> Result<(), Vec<syn::Error>> {
    println!("MOD");
    assert!(
        (item_mod.content.is_some() && item_mod.semi.is_none())
            || (item_mod.content.is_none() && item_mod.semi.is_some())
    );

    if item_mod.semi.is_some() {
        unimplemented!();
    }

    if let Some((_, items)) = item_mod.content.as_mut() {
        let processable = items.iter_mut().filter(|item| match item {
            syn::Item::Fn(_) | syn::Item::Mod(_) | syn::Item::Impl(_) => true,
            _ => false,
        });

        'item_eval: for item in processable {
            for attr in attrs {
                if let AttrApplication::Directly(attr) = attr {
                    match item {
                        // TODO How about impl-s?
                        // TODO exclude/include impl
                        syn::Item::Fn(syn::ItemFn { ref ident, .. })
                        | syn::Item::Mod(syn::ItemMod { ref ident, .. }) => match attr.filter {
                            args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                                continue 'item_eval;
                            }
                            args::Filter::Disable(ref idents) if idents.contains(ident) => {
                                continue 'item_eval;
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }

            let raw_local_attr: &mut Vec<syn::Attribute> = match item {
                syn::Item::Fn(ref mut item_fn) => Ok(item_fn.attrs.as_mut()),
                syn::Item::Mod(ref mut item_mod) => Ok(item_mod.attrs.as_mut()),
                syn::Item::Impl(ref mut item_impl) => Ok(item_impl.attrs.as_mut()),
                _ => Err(vec![syn::Error::new_spanned(
                    "UNREACHABLE".into_token_stream(),
                    "UNREACHABLE",
                )]),
            }?;

            let attrs = create_context(attrs, extract_local_attrs(raw_local_attr)?);

            //
            //     let local_attrs = extract_local_attrs(raw_local_attr)?;
            //
            //     let attrs = attrs
            //         .iter()
            //         .cloned()
            //         .map(|attr| attr.demote())
            //         .chain(local_attrs.map(|local_attr| AttrApplication::Directly(local_attr)))
            //         .collect::<Vec<_>>();
            //
            transform_item(&attrs, item)?;
        }

        // items.iter_mut().for_each(|item| {
        //     if let AttrApplied::Directly = attr_applied {
        //         match *item {
        //             syn::Item::Fn(syn::ItemFn { ref ident, .. })
        //             | syn::Item::Mod(syn::ItemMod { ref ident, .. }) => match args.filter {
        //                 args::Filter::Enable(ref idents) if !idents.contains(ident) => {
        //                     return;
        //                 }
        //                 args::Filter::Disable(ref idents) if idents.contains(ident) => {
        //                     return;
        //                 }
        //                 _ => (),
        //             },
        //             _ => (),
        //         }
        //     }
        //
        //     transform_item(attrs, item);
        // });

        // items.insert(
        //     0,
        //     parse_quote! {
        //         ::std::thread_local! {
        //             static DEPTH: ::std::cell::Cell<usize> = ::std::cell::Cell::new(0);
        //         }
        //     },
        // );
    }

    Ok(())
}

fn transform_impl(
    attrs: &[AttrApplication],
    item_impl: &mut syn::ItemImpl,
) -> Result<(), Vec<syn::Error>> {
    println!("IMPL");

    'item_eval: for impl_item in item_impl.items.iter_mut() {
        if let syn::ImplItem::Method(ref mut impl_item_method) = impl_item {
            // println!("{:?}", impl_item_method.into_token_stream().to_string());

            for attr in attrs {
                if let AttrApplication::Directly(attr) = attr {
                    let ident = &impl_item_method.sig.ident;

                    match attr.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                            continue 'item_eval;
                        }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => {
                            continue 'item_eval;
                        }
                        _ => (),
                    }
                }
            }

            let attrs =
                create_context(attrs, extract_local_attrs(impl_item_method.attrs.as_mut())?);

            impl_item_method.block = construct_traced_block(
                &attrs,
                &impl_item_method.sig.ident,
                &impl_item_method.sig.decl,
                &impl_item_method.block,
            );
        }
    }

    Ok(())
}
//
// fn transform_impl_item(
//     args: &args::Args,
//     attr_applied: AttrApplied,
//     impl_item: &mut syn::ImplItem,
// ) {
//     // Will probably add more cases in the future
//     #[cfg_attr(feature = "cargo-clippy", allow(single_match))]
//     match *impl_item {
//         syn::ImplItem::Method(ref mut impl_item_method) => {
//             transform_method(args, attr_applied, impl_item_method)
//         }
//         _ => (),
//     }
// }
//
// fn transform_method(
//     args: &args::Args,
//     attr_applied: AttrApplied,
//     impl_item_method: &mut syn::ImplItemMethod,
// ) {
//     println!("METHOD");
//     impl_item_method.block = construct_traced_block(
//         &args,
//         attr_applied,
//         &impl_item_method.sig.ident,
//         &impl_item_method.sig.decl,
//         &impl_item_method.block,
//     );
// }
//
fn construct_traced_block(
    attrs: &[AttrApplication],
    ident: &proc_macro2::Ident,
    fn_decl: &syn::FnDecl,
    original_block: &syn::Block,
) -> syn::Block {
    let arg_idents = {
        let mut arg_idents = extract_arg_idents(attrs, &fn_decl);
        arg_idents.sort_by(|a, b| match (a, b) {
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Equal,
        });
        arg_idents.dedup_by(|a, b| a.is_none() && b.is_none());
        arg_idents
    };

    println!("{:?}", arg_idents);

    let pretty = if attrs.iter().any(|attr| attr.pretty) {
        "#"
    } else {
        ""
    };

    let arg_formats = attrs
        .iter()
        .find(|attr| attr.is_direct())
        .map(|attr| &attr.args_format);

    let prefix_enter = attrs
        .iter()
        .map(|attr| &attr.prefix_enter)
        .collect::<Prefix>()
        .enter();

    let prefix_exit = attrs
        .iter()
        .map(|attr| &attr.prefix_exit)
        .collect::<Prefix>()
        .exit();

    let entering_format = {
        let arg_idents_format = arg_idents
            .iter()
            .map(|wrapped| {
                wrapped.as_ref().map_or_else(
                    || "...".to_string(),
                    |arg_ident| {
                        arg_formats
                            .map(|formats| formats.get(arg_ident))
                            .flatten()
                            .map_or_else(
                                || format!("{}: {{:{}?}}", arg_ident, pretty),
                                |fmt| format!("{}: {}", arg_ident, fmt),
                            )
                    },
                )
            })
            .collect::<Vec<_>>()
            .join("\n\t");

        let sep = if arg_idents.is_empty() { "" } else { "\n\t" };

        format!("{}{}{}{}", prefix_enter, ident, sep, arg_idents_format)
    };

    let arg_idents = arg_idents
        .into_iter()
        .filter_map(|wrapped| wrapped)
        .collect::<Vec<_>>();

    let return_var = "res".to_string();
    let exiting_format = arg_formats
        .map(|formats| {
            formats
                .iter()
                .find(|(arg_ident, _)| arg_ident.to_string() == return_var)
        })
        .flatten()
        .map_or_else(
            || {
                format!(
                    "{}{}\n\t{}: {{:{}?}}",
                    prefix_exit, ident, return_var, pretty
                )
            },
            |(_, fmt)| format!("{}{}\n\t{}: {}", prefix_exit, ident, return_var, fmt),
        );

    let pause_stmt = if attrs.iter().any(|attr| attr.pause) {
        quote! {{
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            stdin.lock().lines().next();
        }}
    } else {
        quote!()
    };

    let printer = quote! { log::trace! };

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

fn create_context(given: &[AttrApplication], local: Option<args::Args>) -> Vec<AttrApplication> {
    given
        .iter()
        .cloned()
        .map(|attr| attr.demote())
        .chain(local.map(|local_attr| AttrApplication::Directly(local_attr)))
        .collect::<Vec<_>>()
}

fn extract_local_attrs(
    attrs: &mut Vec<syn::Attribute>,
) -> Result<Option<args::Args>, Vec<syn::Error>> {
    if attrs.is_empty() {
        return Ok(None);
    }

    // Evaluate attached macros.
    let pos = attrs
        .iter()
        .position(|attr| attr.path.segments[0].ident.to_string() == MACRO_NAME);

    if let Some(pos) = pos {
        // Another MACRO_NAME is attached.

        let trace_macro = attrs.remove(pos);
        println!("{:?}", trace_macro.tts.to_string());

        // TODO: is there a better way to strip brackets?
        let str = trace_macro.tts.to_string();
        let str = &str[1..str.len() - 1];
        //
        let local_args = proc_macro::TokenStream::from_str(str).unwrap();
        let raw_local_args = syn::parse_macro_input::parse::<syn::AttributeArgs>(local_args)
            .map_err(|err| vec![err])?;
        let local_args = args::Args::from_raw_args(raw_local_args)?;
        Ok(Some(local_args))
    } else {
        Ok(None)
    }
}

fn extract_arg_idents(
    attrs: &[AttrApplication],
    fn_decl: &syn::FnDecl,
) -> Vec<Option<proc_macro2::Ident>> {
    fn process_pat(
        attrs: &[AttrApplication],
        pat: &syn::Pat,
        arg_idents: &mut Vec<Option<proc_macro2::Ident>>,
    ) {
        match *pat {
            syn::Pat::Ident(ref pat_ident) => {
                let ident = &pat_ident.ident;

                for attr in attrs {
                    if let AttrApplication::Directly(attr) = attr {
                        match attr.filter {
                            args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                                arg_idents.push(None);
                                return;
                            }
                            args::Filter::Disable(ref idents) if idents.contains(ident) => {
                                arg_idents.push(None);
                                return;
                            }
                            _ => (),
                        }
                    };
                }

                arg_idents.push(Some(ident.clone()));
            }
            syn::Pat::Tuple(ref pat_tuple) => {
                pat_tuple.front.iter().for_each(|pat| {
                    process_pat(attrs, pat, arg_idents);
                });
            }
            _ => unimplemented!(),
        }
    }

    let mut arg_idents = Vec::new();

    for input in &fn_decl.inputs {
        match *input {
            syn::FnArg::SelfRef(_) | syn::FnArg::SelfValue(_) => (), // ignore `self`
            syn::FnArg::Captured(ref arg_captured) => {
                process_pat(attrs, &arg_captured.pat, &mut arg_idents);
            }
            syn::FnArg::Inferred(_) | syn::FnArg::Ignored(_) => unimplemented!(),
        }
    }

    arg_idents
}

#[derive(Debug, Clone)]
enum AttrApplication {
    Directly(args::Args),
    Indirectly(args::Args),
}

impl AttrApplication {
    fn demote(self) -> Self {
        if let AttrApplication::Directly(args) = self {
            AttrApplication::Indirectly(args)
        } else {
            self
        }
    }

    fn is_direct(&self) -> bool {
        if let AttrApplication::Directly(_) = self {
            true
        } else {
            false
        }
    }
}

impl Deref for AttrApplication {
    type Target = args::Args;

    fn deref(&self) -> &Self::Target {
        match self {
            AttrApplication::Directly(attr) => attr,
            AttrApplication::Indirectly(attr) => attr,
        }
    }
}
