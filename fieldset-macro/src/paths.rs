use proc_macro2::TokenStream;
use quote::quote;

use crate::{Kind, Model};

pub(crate) fn expand(model: &Model<'_>) -> TokenStream {
    let vis = model.vis;
    let ident = model.ident;
    let path_ident = &model.path_ident;
    let members = &model.members;
    let doc = format!("A leaf of `{ident}`, as data.");
    let variants = members.iter().map(|member| {
        let variant = &member.variant;
        let ty = member.ty;
        match member.kind {
            Kind::Leaf => {
                let doc = format!("The leaf `{}`.", member.ident);
                quote! {
                    #[doc = #doc]
                    #variant,
                }
            }
            Kind::Nested => {
                let doc = format!("A leaf of the subtree `{}`.", member.ident);
                quote! {
                    #[doc = #doc]
                    #variant(<#ty as ::fieldset::Fields>::Path),
                }
            }
            Kind::Indexed { len, .. } => {
                let doc = format!("An element of the field `{}`.", member.ident);
                quote! {
                    #[doc = #doc]
                    #variant(::fieldset::Index<{ #len }>),
                }
            }
            Kind::IndexedNested { elem, len } => {
                let doc = format!("A leaf of one element of the field `{}`.", member.ident);
                quote! {
                    #[doc = #doc]
                    #variant(::fieldset::Index<{ #len }>, <#elem as ::fieldset::Fields>::Path),
                }
            }
        }
    });
    let bases = members.iter().enumerate().map(|(i, member)| {
        let base = member.base_ident();
        let expr = match i.checked_sub(1).and_then(|j| members.get(j)) {
            None => quote!(0),
            Some(prev) => {
                let prev_base = prev.base_ident();
                let prev_count = prev.count_u16();
                quote!(Self::#prev_base + #prev_count)
            }
        };
        quote! {
            const #base: u16 = #expr;
        }
    });
    let lens = members.iter().filter_map(|member| {
        let len_ident = member.len_ident();
        let len = member.len()?;
        Some(quote! {
            const #len_ident: usize = #len;
        })
    });
    let leaf_arms = members.iter().map(|member| {
        let variant = &member.variant;
        let base = member.base_ident();
        match member.kind {
            Kind::Leaf => quote! {
                Self::#variant => Self::#base,
            },
            Kind::Nested => quote! {
                Self::#variant(__inner) => Self::#base + ::fieldset::Path::leaf(__inner),
            },
            Kind::Indexed { .. } => quote! {
                Self::#variant(__i) => Self::#base + (::fieldset::Index::get(__i) as u16),
            },
            Kind::IndexedNested { elem, .. } => quote! {
                Self::#variant(__i, __inner) => Self::#base
                    + (::fieldset::Index::get(__i) as u16)
                        * (<#elem as ::fieldset::Fields>::COUNT as u16)
                    + ::fieldset::Path::leaf(__inner),
            },
        }
    });
    let from_leaf_arms = members.iter().enumerate().map(|(i, member)| {
        let variant = &member.variant;
        let base = member.base_ident();
        let len_ident = member.len_ident();
        let ty = member.ty;
        let in_range = match members.get(i + 1) {
            Some(next) => {
                let next_base = next.base_ident();
                quote!(leaf < Self::#next_base)
            }
            None => quote!((leaf as usize) < <#ident as ::fieldset::Fields>::COUNT),
        };
        let value = match member.kind {
            Kind::Leaf => quote!(::core::option::Option::Some(Self::#variant)),
            Kind::Nested => quote! {
                match <<#ty as ::fieldset::Fields>::Path>::from_leaf(leaf - Self::#base) {
                    ::core::option::Option::Some(__inner) => {
                        ::core::option::Option::Some(Self::#variant(__inner))
                    }
                    ::core::option::Option::None => ::core::option::Option::None,
                }
            },
            Kind::Indexed { .. } => {
                quote! {
                match ::fieldset::Index::<{ Self::#len_ident }>::new((leaf - Self::#base) as usize) {
                    ::core::option::Option::Some(__i) => {
                        ::core::option::Option::Some(Self::#variant(__i))
                    }
                    ::core::option::Option::None => ::core::option::Option::None,
                }
            }
            }
            Kind::IndexedNested { elem, .. } => {
                quote! {
                {
                    let __rel = (leaf - Self::#base) as usize;
                    let __per = <#elem as ::fieldset::Fields>::COUNT;
                    match (
                        ::fieldset::Index::<{ Self::#len_ident }>::new(__rel / __per),
                        <<#elem as ::fieldset::Fields>::Path>::from_leaf((__rel % __per) as u16),
                    ) {
                        (::core::option::Option::Some(__i), ::core::option::Option::Some(__inner)) => {
                            ::core::option::Option::Some(Self::#variant(__i, __inner))
                        }
                        _ => ::core::option::Option::None,
                    }
                }
            }
            }
        };
        quote! {
            if #in_range {
                return #value;
            }
        }
    });
    let wildcard = (members.len() > 1).then(|| quote!(_ => ::core::option::Option::None,));
    let projections = members.iter().filter_map(|member| {
        let field = member.ident;
        let variant = &member.variant;
        let ty = member.ty;
        match member.kind {
            Kind::Leaf | Kind::Indexed { .. } => None,
            Kind::Nested => {
                let doc = format!(
                    "The path within the subtree `{field}`, or `None` when this path lies elsewhere."
                );
                Some(quote! {
                    #[doc = #doc]
                    #vis fn #field(self) -> ::core::option::Option<<#ty as ::fieldset::Fields>::Path> {
                        match self {
                            Self::#variant(__inner) => ::core::option::Option::Some(__inner),
                            #wildcard
                        }
                    }
                })
            }
            Kind::IndexedNested { elem, len } => {
                let doc = format!(
                    "The element index and the path within that element of `{field}`, or `None` when this path lies elsewhere."
                );
                Some(quote! {
                    #[doc = #doc]
                    #vis fn #field(
                        self,
                    ) -> ::core::option::Option<(
                        ::fieldset::Index<{ #len }>,
                        <#elem as ::fieldset::Fields>::Path,
                    )> {
                        match self {
                            Self::#variant(__i, __inner) => ::core::option::Option::Some((__i, __inner)),
                            #wildcard
                        }
                    }
                })
            }
        }
    });
    let from_leaf_doc = format!(
        "The path for a leaf index relative to `{ident}`, or `None` when the index is out of range."
    );
    quote! {
        #[doc = #doc]
        #[derive(
            ::core::marker::Copy,
            ::core::clone::Clone,
            ::core::cmp::Eq,
            ::core::cmp::PartialEq,
            ::core::fmt::Debug
        )]
        #vis enum #path_ident {
            #(#variants)*
        }

        impl #path_ident {
            #(#bases)*
            #(#lens)*

            #[doc = #from_leaf_doc]
            #vis const fn from_leaf(leaf: u16) -> ::core::option::Option<Self> {
                #(#from_leaf_arms)*
                ::core::option::Option::None
            }

            #(#projections)*
        }

        impl ::fieldset::Path for #path_ident {
            fn leaf(self) -> u16 {
                match self {
                    #(#leaf_arms)*
                }
            }

            fn from_leaf(leaf: u16) -> ::core::option::Option<Self> {
                Self::from_leaf(leaf)
            }
        }
    }
}
