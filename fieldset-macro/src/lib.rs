//! Derive macro for the `fieldset` crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod fields_impl;
mod handles;
mod paths;

use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Field, Fields, Ident, Result, Type, Visibility,
};

const MAX_FIELDS: usize = u8::MAX as usize + 1;
const RESERVED_HANDLE_NAMES: [&str; 3] = ["model", "set", "mark_all"];
const RESERVED_PATH_NAMES: [&str; 2] = ["leaf", "from_leaf"];

/// Derives `fieldset::Fields` for a struct of `Copy` leaves, nested `Fields`
/// subtrees and indexed arrays of either.
///
/// For a struct `X` this generates the path enum `XPath`, the write handle
/// `XMut<'a>` and `impl fieldset::Fields for X`, all with the visibility of
/// `X`. Leaves are numbered in declaration order; a `#[fields(nested)]` field
/// contributes the leaves of its own type, contiguously; a
/// `#[fields(indexed)]` field of type `[T; N]` contributes one leaf per
/// element; a `#[fields(indexed, nested)]` field of type `[S; N]` contributes
/// the leaves of `S` once per element, element after element; and a
/// `#[fields(skip)]` field contributes nothing.
#[proc_macro_derive(Fields, attributes(fields))]
pub fn derive_fields(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// How a non-skipped field takes part in the derive.
#[derive(Clone, Copy)]
enum Kind<'a> {
    /// A `Copy` value addressed by one leaf.
    Leaf,
    /// A `Fields` value whose leaves are spliced in at this field's base.
    Nested,
    /// A `[T; N]` of `Copy` elements, one leaf per element.
    Indexed { elem: &'a Type, len: &'a Expr },
    /// A `[S; N]` of `Fields` elements, the leaves of `S` once per element.
    IndexedNested { elem: &'a Type, len: &'a Expr },
}

/// A non-skipped field.
struct Member<'a> {
    /// Position among the non-skipped fields, in declaration order.
    position: u8,
    ident: &'a Ident,
    variant: Ident,
    ty: &'a Type,
    kind: Kind<'a>,
}

impl<'a> Member<'a> {
    /// The private associated const on the path enum holding this member's
    /// first leaf, relative to the struct.
    fn base_ident(&self) -> Ident {
        format_ident!("BASE_{}", self.position)
    }

    /// The private associated const on the path enum holding the element
    /// count of this indexed member.
    fn len_ident(&self) -> Ident {
        format_ident!("LEN_{}", self.position)
    }

    /// The type one leaf or subtree of this member has: the element type of
    /// an indexed member, the field type otherwise.
    fn element(&self) -> &'a Type {
        match self.kind {
            Kind::Leaf | Kind::Nested => self.ty,
            Kind::Indexed { elem, .. } | Kind::IndexedNested { elem, .. } => elem,
        }
    }

    /// Whether each element is a `Fields` subtree rather than a `Copy` leaf.
    fn is_nested(&self) -> bool {
        matches!(self.kind, Kind::Nested | Kind::IndexedNested { .. })
    }

    /// The array length of an indexed member.
    fn len(&self) -> Option<&'a Expr> {
        match self.kind {
            Kind::Leaf | Kind::Nested => None,
            Kind::Indexed { len, .. } | Kind::IndexedNested { len, .. } => Some(len),
        }
    }

    /// The number of leaves this member contributes, as a `usize` expression;
    /// `None` for a plain leaf, which the caller counts as one.
    fn count(&self) -> Option<TokenStream> {
        let elem = self.element();
        match (self.len(), self.is_nested()) {
            (None, false) => None,
            (None, true) => Some(quote::quote!(<#elem as ::fieldset::Fields>::COUNT)),
            (Some(len), false) => Some(quote::quote!((#len))),
            (Some(len), true) => Some(quote::quote!((#len) * <#elem as ::fieldset::Fields>::COUNT)),
        }
    }

    /// The number of leaves this member contributes, as a `u16` expression.
    fn count_u16(&self) -> TokenStream {
        let elem = self.element();
        match (self.len(), self.is_nested()) {
            (None, false) => quote::quote!(1),
            (None, true) => quote::quote!((<#elem as ::fieldset::Fields>::COUNT as u16)),
            (Some(len), false) => quote::quote!(((#len) as u16)),
            (Some(len), true) => {
                quote::quote!((((#len) * <#elem as ::fieldset::Fields>::COUNT) as u16))
            }
        }
    }
}

struct Model<'a> {
    vis: &'a Visibility,
    ident: &'a Ident,
    path_ident: Ident,
    mut_ident: Ident,
    members: Vec<Member<'a>>,
}

fn expand(input: &DeriveInput) -> Result<TokenStream> {
    let model = parse(input)?;
    let ident = model.ident;
    let bound_checks = model.members.iter().map(|member| {
        let checked = member.element();
        if member.is_nested() {
            quote_spanned! {checked.span()=>
                const _: () = {
                    const fn nested<T: ::fieldset::Fields>() {}
                    nested::<#checked>();
                };
            }
        } else {
            quote_spanned! {checked.span()=>
                const _: () = {
                    const fn leaf<T: ::core::marker::Copy>() {}
                    leaf::<#checked>();
                };
            }
        }
    });
    let paths = paths::expand(&model);
    let handles = handles::expand(&model);
    let fields_impl = fields_impl::expand(&model);
    Ok(quote::quote! {
        #(#bound_checks)*
        #paths
        #handles
        #fields_impl
        const _: () = ::core::assert!(
            <#ident as ::fieldset::Fields>::COUNT <= u16::MAX as usize,
            "`Fields` supports at most 65535 leaves"
        );
    })
}

fn parse(input: &DeriveInput) -> Result<Model<'_>> {
    let ident = &input.ident;
    reject_fields_attrs(&input.attrs, ident)?;
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            format!("`{ident}`: `Fields` cannot be derived for a generic struct"),
        ));
    }
    if let Some(where_clause) = &input.generics.where_clause {
        return Err(Error::new_spanned(
            where_clause,
            format!("`{ident}`: `Fields` cannot be derived for a generic struct"),
        ));
    }
    let data = match &input.data {
        Data::Struct(data) => data,
        Data::Enum(_) | Data::Union(_) => {
            return Err(Error::new_spanned(
                ident,
                format!("`{ident}`: `Fields` can only be derived for a struct"),
            ));
        }
    };
    let named = match &data.fields {
        Fields::Named(named) => &named.named,
        Fields::Unnamed(unnamed) => {
            return Err(Error::new_spanned(
                unnamed,
                format!("`{ident}`: `Fields` requires named fields"),
            ));
        }
        Fields::Unit => {
            return Err(Error::new_spanned(
                ident,
                format!("`{ident}`: `Fields` requires named fields"),
            ));
        }
    };
    let mut members: Vec<Member<'_>> = Vec::with_capacity(named.len());
    for field in named {
        if let Some(member) = parse_field(ident, members.len(), field)? {
            if let Some(earlier) = members.iter().find(|m| m.variant == member.variant) {
                return Err(Error::new_spanned(
                    member.ident,
                    format!(
                        "`{ident}::{}`: variant `{}` is already used by field `{}`",
                        member.ident, member.variant, earlier.ident
                    ),
                ));
            }
            members.push(member);
        }
    }
    if members.is_empty() {
        return Err(Error::new_spanned(
            ident,
            format!("`{ident}`: `Fields` requires at least one field that is not skipped"),
        ));
    }
    Ok(Model {
        vis: &input.vis,
        ident,
        path_ident: format_ident!("{ident}Path"),
        mut_ident: format_ident!("{ident}Mut"),
        members,
    })
}

/// Parses one field; `None` for a skipped one.
fn parse_field<'a>(owner: &Ident, position: usize, field: &'a Field) -> Result<Option<Member<'a>>> {
    field.modifiers.require_empty()?;
    let ident = field.ident.as_ref().ok_or_else(|| {
        Error::new_spanned(field, format!("`{owner}`: `Fields` requires named fields"))
    })?;
    if let Some((_, default)) = &field.default {
        return Err(Error::new_spanned(
            default,
            format!("`{owner}::{ident}`: `Fields` does not support default field values"),
        ));
    }
    let Some(keys) = parse_field_attrs(owner, ident, field)? else {
        return Ok(None);
    };
    let name = ident.unraw().to_string();
    if RESERVED_HANDLE_NAMES.contains(&name.as_str()) {
        return Err(Error::new_spanned(
            ident,
            format!("`{owner}::{ident}`: `{name}` is reserved on the write handle"),
        ));
    }
    let kind = match (keys.indexed, keys.nested) {
        (false, false) => Kind::Leaf,
        (false, true) => Kind::Nested,
        (true, nested) => {
            let (elem, len) = indexed_parts(owner, ident, &field.ty)?;
            if nested {
                Kind::IndexedNested { elem, len }
            } else {
                Kind::Indexed { elem, len }
            }
        }
    };
    if keys.nested && RESERVED_PATH_NAMES.contains(&name.as_str()) {
        return Err(Error::new_spanned(
            ident,
            format!("`{owner}::{ident}`: `{name}` is reserved on the path enum"),
        ));
    }
    let position = u8::try_from(position).map_err(|_| {
        Error::new_spanned(
            ident,
            format!("`{owner}::{ident}`: `Fields` supports at most {MAX_FIELDS} fields"),
        )
    })?;
    let variant_name = name.to_upper_camel_case();
    let variant = syn::parse_str::<Ident>(&variant_name).map_err(|_| {
        Error::new_spanned(
            ident,
            format!("`{owner}::{ident}`: `{variant_name}` is not a usable variant name"),
        )
    })?;
    Ok(Some(Member {
        position,
        ident,
        variant,
        ty: &field.ty,
        kind,
    }))
}

/// The `#[fields(..)]` keys of a field that is not skipped.
struct Keys {
    nested: bool,
    indexed: bool,
}

/// Reads the `#[fields(..)]` attributes of a field; `None` means skipped.
fn parse_field_attrs(owner: &Ident, ident: &Ident, field: &Field) -> Result<Option<Keys>> {
    let mut nested = false;
    let mut skip = false;
    let mut indexed = false;
    for attr in field.attrs.iter().filter(|attr| is_fields(attr)) {
        let mut seen = false;
        attr.parse_nested_meta(|meta| {
            seen = true;
            let flag = if meta.path.is_ident("nested") {
                &mut nested
            } else if meta.path.is_ident("skip") {
                &mut skip
            } else if meta.path.is_ident("indexed") {
                &mut indexed
            } else {
                return Err(meta.error(format!(
                    "`{owner}::{ident}`: unknown `#[fields(..)]` key; expected `nested`, `skip` or `indexed`"
                )));
            };
            if *flag {
                return Err(
                    meta.error(format!("`{owner}::{ident}`: duplicate `#[fields(..)]` key"))
                );
            }
            *flag = true;
            Ok(())
        })?;
        if !seen {
            return Err(Error::new_spanned(
                attr,
                format!(
                    "`{owner}::{ident}`: empty `#[fields()]`; expected `nested`, `skip` or `indexed`"
                ),
            ));
        }
    }
    if skip {
        if nested || indexed {
            return Err(Error::new_spanned(
                field,
                format!("`{owner}::{ident}`: `skip` cannot be combined with other keys"),
            ));
        }
        return Ok(None);
    }
    Ok(Some(Keys { nested, indexed }))
}

/// Splits the type of an `indexed` field into its element type and length.
fn indexed_parts<'a>(owner: &Ident, ident: &Ident, ty: &'a Type) -> Result<(&'a Type, &'a Expr)> {
    let Type::Array(array) = ty else {
        return Err(Error::new_spanned(
            ty,
            format!("`{owner}::{ident}`: `indexed` requires an array type `[T; N]`"),
        ));
    };
    let elem = &*array.elem;
    if matches!(elem, Type::Array(_)) {
        return Err(Error::new_spanned(
            elem,
            format!("`{owner}::{ident}`: `indexed` supports one level of indexing"),
        ));
    }
    Ok((elem, &array.len))
}

fn is_fields(attr: &Attribute) -> bool {
    attr.path().is_ident("fields")
}

fn reject_fields_attrs(attrs: &[Attribute], ident: &Ident) -> Result<()> {
    attrs
        .iter()
        .find(|attr| is_fields(attr))
        .map_or(Ok(()), |attr| {
            Err(Error::new_spanned(
                attr,
                format!("`{ident}`: `#[fields(..)]` attributes are not supported"),
            ))
        })
}
