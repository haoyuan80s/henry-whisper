use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    FnArg, ForeignItemFn, ItemFn, PatType, ReturnType, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
};

// ── tauri_commands! ──────────────────────────────────────────────────────────
// Original inline macro (kept for reference / existing callers).

struct ForeignList {
    items: Vec<ForeignItemFn>,
}
impl Parse for ForeignList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse::<ForeignItemFn>()?);
        }
        Ok(ForeignList { items })
    }
}

/// Generates async WASM frontend IPC bindings from inline function signatures.
/// `invoke` must be in scope at the call site.
#[proc_macro]
pub fn tauri_commands(input: TokenStream) -> TokenStream {
    let ForeignList { items } = parse_macro_input!(input as ForeignList);
    let bindings: Vec<TokenStream2> = items.iter().map(gen_inline_binding).collect();
    quote! { #(#bindings)* }.into()
}

// ── #[ipc_command] ───────────────────────────────────────────────────────────

/// Annotate a `#[tauri::command]` function to expose its signature for IPC
/// code generation.  Emits an additional `__ipc_meta_<name>()` companion
/// function that `collect_commands!` calls.
#[proc_macro_attribute]
pub fn ipc_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let name = &func.sig.ident;
    let meta_fn = format_ident!("__ipc_meta_{}", name);
    let name_str = name.to_string();

    let ipc_params: Vec<&PatType> = func
        .sig
        .inputs
        .iter()
        .filter_map(|a| if let FnArg::Typed(pt) = a { Some(pt) } else { None })
        .filter(|pt| !is_tauri_type(&pt.ty))
        .collect();

    let params_init: Vec<TokenStream2> = ipc_params.iter().map(|pt| {
        let pname = match &*pt.pat {
            syn::Pat::Ident(pi) => pi.ident.to_string(),
            _ => "_".to_string(),
        };
        let ptype = type_str(&pt.ty);
        quote! { (#pname, #ptype) }
    }).collect();

    let ret_expr = match &func.sig.output {
        ReturnType::Default => quote! { ::std::option::Option::None },
        ReturnType::Type(_, ty) => {
            let inner = unwrap_result_str(ty);
            if inner == "()" {
                quote! { ::std::option::Option::None }
            } else {
                quote! { ::std::option::Option::Some(#inner) }
            }
        }
    };

    quote! {
        #func

        pub fn #meta_fn() -> ::henry_whisper_ipc_gen::CommandMeta {
            ::henry_whisper_ipc_gen::CommandMeta {
                name: #name_str,
                params: &[#(#params_init),*],
                return_type: #ret_expr,
            }
        }
    }
    .into()
}

// ── collect_commands! ────────────────────────────────────────────────────────

/// Collects IPC metadata from `#[ipc_command]`-annotated functions.
///
/// ```rust
/// let cmds = collect_commands![frontend_debug, get_settings, commands::save_settings];
/// ```
///
/// Expands to `vec![__ipc_meta_frontend_debug(), __ipc_meta_get_settings(), commands::__ipc_meta_save_settings()]`.
#[proc_macro]
pub fn collect_commands(input: TokenStream) -> TokenStream {
    let paths = parse_macro_input!(input with Punctuated::<syn::Path, Comma>::parse_terminated);

    let calls: Vec<TokenStream2> = paths
        .iter()
        .map(|path| {
            let mut p = path.clone();
            if let Some(last) = p.segments.last_mut() {
                last.ident = format_ident!("__ipc_meta_{}", last.ident);
                last.arguments = syn::PathArguments::None;
            }
            quote! { #p() }
        })
        .collect();

    quote! { vec![#(#calls),*] }.into()
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn is_tauri_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        let mut segs = tp.path.segments.iter();
        if let Some(first) = segs.next() {
            if first.ident == "tauri" {
                return true;
            }
            // Re-exported bare names
            if matches!(
                first.ident.to_string().as_str(),
                "AppHandle" | "State" | "Window" | "WebviewWindow"
            ) {
                return true;
            }
        }
    }
    false
}

/// Returns the type's token string stripped of spaces.
fn type_str(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

/// If `ty` is `Result<T, _>` return the string for `T`, else return `type_str(ty)`.
fn unwrap_result_str(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                        return type_str(inner);
                    }
                }
            }
        }
    }
    type_str(ty)
}

// ── tauri_commands! inline helpers (unchanged) ───────────────────────────────

fn normalize_ty(ty: &Type) -> TokenStream2 {
    if let Type::Path(tp) = ty {
        if tp.path.is_ident("String") {
            return quote! { &str };
        }
    }
    quote! { #ty }
}

fn unwrap_result(ty: &Type) -> &Type {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    ty
}

fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}

fn gen_inline_binding(f: &ForeignItemFn) -> TokenStream2 {
    let name = &f.sig.ident;
    let cmd = name.to_string();

    let params: Vec<&PatType> = f
        .sig
        .inputs
        .iter()
        .filter_map(|a| if let FnArg::Typed(pt) = a { Some(pt) } else { None })
        .collect();

    let sig_params: Vec<TokenStream2> = params
        .iter()
        .map(|pt| { let p = &pt.pat; let t = normalize_ty(&pt.ty); quote! { #p: #t } })
        .collect();

    let args_expr = if params.is_empty() {
        quote! { ::wasm_bindgen::JsValue::NULL }
    } else {
        let fields: Vec<TokenStream2> = params.iter().map(|pt| {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                let k = pi.ident.to_string(); let v = &pi.ident;
                quote! { #k: #v }
            } else {
                quote! { compile_error!("tauri_commands: param must be a plain identifier") }
            }
        }).collect();
        quote! {
            ::serde_wasm_bindgen::to_value(&::serde_json::json!({ #(#fields),* }))
                .map_err(|e| ::wasm_bindgen::JsValue::from_str(&e.to_string()))?
        }
    };

    let ret_ty = match &f.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => {
            let inner = unwrap_result(ty);
            if is_unit(inner) { None } else { Some(inner) }
        }
    };

    if let Some(vty) = ret_ty {
        quote! {
            pub async fn #name(#(#sig_params),*) -> ::std::result::Result<#vty, ::wasm_bindgen::JsValue> {
                let __result = invoke(#cmd, #args_expr).await?;
                let __value: #vty = ::serde_wasm_bindgen::from_value(__result)
                    .map_err(|e| ::wasm_bindgen::JsValue::from_str(&e.to_string()))?;
                Ok(__value)
            }
        }
    } else {
        quote! {
            pub async fn #name(#(#sig_params),*) -> ::std::result::Result<(), ::wasm_bindgen::JsValue> {
                invoke(#cmd, #args_expr).await?;
                Ok(())
            }
        }
    }
}
