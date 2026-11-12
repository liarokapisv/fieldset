use proc_macro2::TokenStream;
use quote::quote;

use crate::Model;

pub(crate) fn expand(model: &Model<'_>) -> TokenStream {
    let vis = model.vis;
    let ident = model.ident;
    let path_ident = &model.path_ident;
    let mut_ident = &model.mut_ident;
    let doc = format!("A write handle over `{ident}`.");
    let methods = model.members.iter().map(|member| {
        let field = member.ident;
        let base = member.base_ident();
        let leaf_index = if member.position == 0 {
            quote!(self.base)
        } else {
            quote!(self.base + #path_ident::#base)
        };
        let elem = member.element();
        let (params, target, offset) = match member.len() {
            None => (quote!(), quote!(&mut self.model.#field), quote!()),
            Some(len) => (
                quote!(, i: ::fieldset::Index<{ #len }>),
                quote!(&mut self.model.#field[::fieldset::Index::get(i)]),
                if member.is_nested() {
                    quote! {
                        + (::fieldset::Index::get(i) as u16)
                            * (<#elem as ::fieldset::Fields>::COUNT as u16)
                    }
                } else {
                    quote!(+ (::fieldset::Index::get(i) as u16))
                },
            ),
        };
        let doc = match (member.len(), member.is_nested()) {
            (None, false) => {
                format!("A handle over the field `{field}`; every write through it marks the leaf.")
            }
            (None, true) => format!("A handle over the subtree `{field}`."),
            (Some(_), false) => format!(
                "A handle over the element `i` of the field `{field}`; every write through it marks the leaf."
            ),
            (Some(_), true) => format!("A handle over the element `i` of the field `{field}`."),
        };
        if member.is_nested() {
            quote! {
                #[doc = #doc]
                #vis fn #field(&mut self #params) -> <#elem as ::fieldset::Fields>::Mut<'_> {
                    <#elem as ::fieldset::Fields>::handle(
                        #target,
                        &mut *self.mask,
                        #leaf_index #offset,
                    )
                }
            }
        } else {
            quote! {
                #[doc = #doc]
                #vis fn #field(&mut self #params) -> ::fieldset::Field<'_, #elem> {
                    ::fieldset::Field::new(#target, &mut *self.mask, #leaf_index #offset)
                }
            }
        }
    });
    let model_doc = format!("Reads the `{ident}` under this handle.");
    let set_doc = format!("Replaces the whole `{ident}` and marks every leaf under it.");
    let mark_all_doc = format!("Marks every leaf of this `{ident}` as written.");
    quote! {
        #[doc = #doc]
        #vis struct #mut_ident<'a> {
            model: &'a mut #ident,
            mask: &'a mut ::fieldset::Mask,
            base: u16,
        }

        impl #mut_ident<'_> {
            #(#methods)*

            #[doc = #model_doc]
            #vis fn model(&self) -> &#ident {
                &*self.model
            }

            #[doc = #set_doc]
            #vis fn set(&mut self, value: #ident) {
                *self.model = value;
                self.mark_all();
            }

            #[doc = #mark_all_doc]
            #vis fn mark_all(&mut self) {
                ::fieldset::MaskOps::set_range(
                    &mut *self.mask,
                    self.base,
                    self.base + (<#ident as ::fieldset::Fields>::COUNT as u16),
                );
            }
        }
    }
}
