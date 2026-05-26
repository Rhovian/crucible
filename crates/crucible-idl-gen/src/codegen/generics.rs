//! Helpers for emitting Rust generics from Anchor IDL `generics`/`generic-arg` data.
//!
//! Every helper returns an **empty** `TokenStream` when there are no generics so
//! call sites stay branch-free: `pub struct Foo #generic_params_decl(...) { ... }`
//! emits `pub struct Foo { ... }` for the no-generic case.

use anchor_lang_idl::types::{IdlArrayLen, IdlGenericArg, IdlTypeDefGeneric};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::idl_type_to_tokens;

/// `<A, B, const N: usize>` — for use on the declaration line of a struct/enum/type alias.
pub(crate) fn generic_params_decl(gs: &[IdlTypeDefGeneric]) -> TokenStream {
    if gs.is_empty() {
        return TokenStream::new();
    }
    let items = gs.iter().map(|g| match g {
        IdlTypeDefGeneric::Type { name } => {
            let ident = format_ident!("{}", name);
            quote! { #ident }
        }
        IdlTypeDefGeneric::Const { name, ty } => {
            let ident = format_ident!("{}", name);
            let ty_tokens = parse_const_generic_type(ty);
            quote! { const #ident: #ty_tokens }
        }
    });
    quote! { < #(#items),* > }
}

/// `<A, B, N>` — for use after a type name (impl target, self-reference). No bounds, no `const`.
pub(crate) fn generic_params_use(gs: &[IdlTypeDefGeneric]) -> TokenStream {
    if gs.is_empty() {
        return TokenStream::new();
    }
    let items = gs.iter().map(|g| {
        let name = match g {
            IdlTypeDefGeneric::Type { name } | IdlTypeDefGeneric::Const { name, .. } => name,
        };
        let ident = format_ident!("{}", name);
        quote! { #ident }
    });
    quote! { < #(#items),* > }
}

/// `<u64, 8>` — instantiated arguments at a reference site.
pub(crate) fn generic_args_to_tokens(args: &[IdlGenericArg]) -> TokenStream {
    if args.is_empty() {
        return TokenStream::new();
    }
    let items = args.iter().map(|a| match a {
        IdlGenericArg::Type { ty } => idl_type_to_tokens(ty),
        IdlGenericArg::Const { value } => value
            .parse::<TokenStream>()
            .unwrap_or_else(|e| panic!("invalid const generic value `{value}`: {e}")),
    });
    quote! { < #(#items),* > }
}

/// `where T: Bound, U: Bound` — for adding a trait bound to every TYPE generic
/// param of a typedef. Const generics are skipped (no bounds apply). Returns
/// empty `TokenStream` if there are no type generics.
///
/// Use this for emitting hand-written `impl` blocks that need bounds (e.g.,
/// `unsafe impl<T> Pod for Foo<T> where T: bytemuck::Pod {}`). For derive-based
/// impls, the derive macro auto-adds the bounds — no helper needed.
pub(crate) fn type_generic_bounds(gs: &[IdlTypeDefGeneric], bound: TokenStream) -> TokenStream {
    let idents: Vec<_> = gs
        .iter()
        .filter_map(|g| match g {
            IdlTypeDefGeneric::Type { name } => Some(format_ident!("{}", name)),
            IdlTypeDefGeneric::Const { .. } => None,
        })
        .collect();
    if idents.is_empty() {
        return TokenStream::new();
    }
    quote! { where #(#idents: #bound),* }
}

/// Array length to tokens. Replaces `array_len_to_usize` for callsites that may
/// see `IdlArrayLen::Generic(N)` — emits `N` as an identifier in that case so
/// `[T; N]` resolves against the surrounding generic scope.
pub(crate) fn array_len_to_tokens(len: &IdlArrayLen) -> TokenStream {
    match len {
        IdlArrayLen::Value(n) => {
            let lit = proc_macro2::Literal::usize_suffixed(*n);
            quote! { #lit }
        }
        IdlArrayLen::Generic(name) => {
            let ident = format_ident!("{}", name);
            quote! { #ident }
        }
    }
}

fn parse_const_generic_type(ty: &str) -> TokenStream {
    ty.parse::<TokenStream>()
        .unwrap_or_else(|e| panic!("invalid const-generic type `{ty}`: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang_idl::types::IdlType;

    #[test]
    fn decl_empty() {
        assert!(generic_params_decl(&[]).is_empty());
    }

    #[test]
    fn decl_type_only() {
        let gs = vec![
            IdlTypeDefGeneric::Type { name: "A".into() },
            IdlTypeDefGeneric::Type { name: "B".into() },
        ];
        assert_eq!(generic_params_decl(&gs).to_string(), "< A , B >");
    }

    #[test]
    fn decl_mixed_type_and_const() {
        let gs = vec![
            IdlTypeDefGeneric::Type { name: "A".into() },
            IdlTypeDefGeneric::Const {
                name: "N".into(),
                ty: "usize".into(),
            },
        ];
        assert_eq!(
            generic_params_decl(&gs).to_string(),
            "< A , const N : usize >"
        );
    }

    #[test]
    fn use_mixed() {
        let gs = vec![
            IdlTypeDefGeneric::Type { name: "A".into() },
            IdlTypeDefGeneric::Const {
                name: "N".into(),
                ty: "usize".into(),
            },
        ];
        assert_eq!(generic_params_use(&gs).to_string(), "< A , N >");
    }

    #[test]
    fn args_type() {
        let args = vec![IdlGenericArg::Type { ty: IdlType::U64 }];
        assert_eq!(generic_args_to_tokens(&args).to_string(), "< u64 >");
    }

    #[test]
    fn args_const() {
        let args = vec![IdlGenericArg::Const { value: "8".into() }];
        assert_eq!(generic_args_to_tokens(&args).to_string(), "< 8 >");
    }

    #[test]
    fn args_mixed() {
        let args = vec![
            IdlGenericArg::Type { ty: IdlType::U64 },
            IdlGenericArg::Const { value: "8".into() },
        ];
        assert_eq!(generic_args_to_tokens(&args).to_string(), "< u64 , 8 >");
    }

    #[test]
    fn array_len_value() {
        assert_eq!(
            array_len_to_tokens(&IdlArrayLen::Value(32)).to_string(),
            "32usize"
        );
    }

    #[test]
    fn array_len_generic() {
        assert_eq!(
            array_len_to_tokens(&IdlArrayLen::Generic("N".into())).to_string(),
            "N"
        );
    }
}
