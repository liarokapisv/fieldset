use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens};

use crate::{Member, Model};

pub(crate) fn expand(model: &Model<'_>) -> TokenStream {
    let ident = model.ident;
    let path_ident = &model.path_ident;
    let mut_ident = &model.mut_ident;
    let leaves = model
        .members
        .iter()
        .filter(|member| member.count().is_none())
        .count();
    let leaf_term = (leaves > 0).then(|| Literal::usize_unsuffixed(leaves).into_token_stream());
    let other_terms = model.members.iter().filter_map(Member::count);
    let terms = leaf_term.into_iter().chain(other_terms);
    quote! {
        impl ::fieldset::Fields for #ident {
            const COUNT: usize = #(#terms)+*;

            type Words = [u32; (<#ident as ::fieldset::Fields>::COUNT).div_ceil(32)];

            type Path = #path_ident;

            type Mut<'a> = #mut_ident<'a>;

            fn handle<'a>(
                &'a mut self,
                mask: &'a mut ::fieldset::Mask,
                base: u16,
            ) -> #mut_ident<'a> {
                #mut_ident {
                    model: self,
                    mask,
                    base,
                }
            }
        }
    }
}
