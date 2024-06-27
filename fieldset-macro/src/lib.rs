use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Field, FieldsNamed, Ident, Type};

fn is_fieldset(field: Field) -> bool {
    field
        .attrs
        .iter()
        .filter_map(|a| a.path().get_ident())
        .any(|i| *i == format_ident!("fieldset"))
}

fn get_type_identifier(ty: Type) -> Ident {
    match ty {
        Type::Path(p) => {
            assert!(p.clone().qself.is_none());
            p.path
                .get_ident()
                .expect("field type must be a path with an identifier")
                .clone()
        }
        _ => panic!("unsupported field type"),
    }
}

fn get_field_identifier(field: Field) -> Ident {
    field
        .ident
        .expect("Cannot derive field type from tuple structs")
}

fn derive_field_type(name: String, fields: FieldsNamed) -> TokenStream {
    let derived_field_type_identifier = format_ident!("{}FieldType", name);
    let enum_variants = {
        let mut res = Vec::new();
        for field in fields.named {
            let variant_name = format_ident!(
                "{}",
                get_field_identifier(field.clone())
                    .to_string()
                    .to_upper_camel_case()
            );
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let field_type_identifier = format_ident!("{}FieldType", type_identifier);
                res.push(quote!(#variant_name(#field_type_identifier)));
            } else {
                let ty = field.ty;
                res.push(quote!(#variant_name(#ty)));
            }
        }
        res
    };
    quote!(
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum #derived_field_type_identifier {
            #(#enum_variants ,)*
        }
    )
    .into()
}

fn derive_into_iterator(name: String, fields: FieldsNamed) -> TokenStream {
    let identifier = format_ident!("{}", name);
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let iter_chains = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_identifier = get_field_identifier(field.clone());
            let variant_name = format_ident!(
                "{}",
                field_identifier.clone().to_string().to_upper_camel_case()
            );
            if is_fieldset(field.clone()) {
                res.push(quote!(let iter = iter.chain(self.#field_identifier.into_iter().map(#fieldtype_identifier::#variant_name))));
            } else {
                res.push(quote!(let iter = iter.chain(once(#fieldtype_identifier::#variant_name(self.#field_identifier)))));
            }
        }
        res
    };
    quote!(
        impl IntoIterator for #identifier {
            type Item = #fieldtype_identifier;
            type IntoIter = impl Iterator<Item = Self::Item> + Clone + core::fmt::Debug;

            fn into_iter(self) -> Self::IntoIter {
                use core::iter::empty;
                use core::iter::once;
                let iter = empty();

                #( #iter_chains ;)*

                iter
            }
        }
    )
    .into()
}

fn derive_setter_trait(name: String, fields: FieldsNamed) -> TokenStream {
    let derived_setter_trait_identifier = format_ident!("{}FieldSetter", name);
    let field_type_identifier = format_ident!("{}FieldType", name);
    let methods = {
        let mut res = Vec::new();
        for field in fields.clone().named {
            let method_name = get_field_identifier(field.clone());
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let field_setter_trait_identifier =
                    format_ident!("{}FieldSetter", type_identifier);
                res.push(quote!(fn #method_name(&mut self) -> impl #field_setter_trait_identifier));
            } else {
                let ty = field.ty;
                res.push(quote!(fn #method_name(&mut self) -> impl fieldset::FieldSetter<#ty>));
            }
        }
        res
    };
    let match_arms = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_identifier = get_field_identifier(field.clone());
            let variant_name = format_ident!(
                "{}",
                field_identifier.clone().to_string().to_upper_camel_case()
            );
            if is_fieldset(field.clone()) {
                res.push(quote!(#field_type_identifier::#variant_name(x) => self.#field_identifier().apply(x)));
            } else {
                res.push(
                    quote!(#field_type_identifier::#variant_name(x) => self.#field_identifier().set(x)),
                );
            }
        }
        res
    };

    quote!(
        pub trait #derived_setter_trait_identifier {
            #( #methods ;)*

            fn apply(&mut self, field: #field_type_identifier) {
                match field {
                    #( #match_arms ,)*
                }
            }
        }
    )
    .into()
}

fn get_variance_identifier(ty: Ident) -> Ident {
    format_ident!("{}_VARIANCE", ty.to_string().to_shouty_snake_case())
}

fn derive_fieldset_variance(name: String, fields: FieldsNamed) -> TokenStream {
    let identifier = format_ident!("{}", name);
    let variance_identifier = get_variance_identifier(identifier);
    let variance = {
        let mut variances = Vec::new();
        let mut field_count: usize = 0;
        for field in fields.named {
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let variance_identifier = get_variance_identifier(type_identifier);
                variances.push(quote!(#variance_identifier));
            } else {
                field_count += 1;
            }
        }
        quote!(#( #variances +)* #field_count)
    };
    quote!(
        const #variance_identifier : usize = #variance;
    )
    .into()
}

fn derive_raw_fieldset_setter_trait_impl(name: String, fields: FieldsNamed) -> TokenStream {
    let identifier = format_ident!("{}", name);
    let setter_trait_identifier = format_ident!("{}FieldSetter", name);
    let methods = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_name = get_field_identifier(field.clone());
            let method_name = field_name.clone();
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let field_setter_trait_identifier =
                    format_ident!("{}FieldSetter", type_identifier);
                res.push(quote!(fn #method_name(&mut self) -> impl #field_setter_trait_identifier { &mut self.#field_name }));
            } else {
                let ty = field.ty;
                res.push(
                    quote!(fn #method_name(&mut self) -> impl fieldset::FieldSetter<#ty> { fieldset::RawFieldSetter(&mut self.#field_name) }),
                );
            }
        }
        res
    };

    quote!(
        impl #setter_trait_identifier for &mut #identifier {
            #( #methods )*
        }

        impl #setter_trait_identifier for #identifier {
            #( #methods )*
        }
    )
    .into()
}

fn derive_opt_fieldset_type(name: String, fields: FieldsNamed) -> TokenStream {
    let derived_fieldset_identifier = format_ident!("{}OptFieldSet", name);
    let opt_fields = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_identifier = get_field_identifier(field.clone());
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let fieldset_identifier = format_ident!("{}OptFieldSet", type_identifier);
                res.push(quote!(#field_identifier : #fieldset_identifier));
            } else {
                let ty = field.ty;
                res.push(quote!(#field_identifier : Option<#ty>))
            }
        }
        res
    };
    quote!(
        #[derive(Debug, Default)]
        pub struct #derived_fieldset_identifier {
            #(#opt_fields ,)*
        }

        impl #derived_fieldset_identifier {
            pub fn new() -> Self {
                Default::default()
            }
        }
    )
    .into()
}

fn derive_opt_fieldset_setter_trait_impl(name: String, fields: FieldsNamed) -> TokenStream {
    let setter_trait_identifier = format_ident!("{}FieldSetter", name);
    let fieldset_identifier = format_ident!("{}OptFieldSet", name);
    let methods = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_name = get_field_identifier(field.clone());
            let method_name = field_name.clone();
            if is_fieldset(field.clone()) {
                let type_identifier = get_type_identifier(field.ty);
                let field_setter_trait_identifier =
                    format_ident!("{}FieldSetter", type_identifier);
                res.push(quote!(fn #method_name(&mut self) -> impl #field_setter_trait_identifier { &mut self.#field_name }));
            } else {
                let ty = field.ty;
                res.push(
                    quote!(fn #method_name(&mut self) -> impl fieldset::FieldSetter<#ty> { fieldset::OptFieldSetter(&mut self.#field_name) }),
                );
            }
        }
        res
    };

    quote!(
        impl #setter_trait_identifier for &mut #fieldset_identifier {
            #( #methods )*
        }

        impl #setter_trait_identifier for #fieldset_identifier {
            #( #methods )*
        }
    )
    .into()
}

fn derive_opt_fieldset_into_iterator(name: String, fields: FieldsNamed) -> TokenStream {
    let fieldset_identifier = format_ident!("{}OptFieldSet", name);
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let iter_chains = {
        let mut res = Vec::new();
        for field in fields.named {
            let field_identifier = get_field_identifier(field.clone());
            let variant_name = format_ident!(
                "{}",
                field_identifier.clone().to_string().to_upper_camel_case()
            );
            if is_fieldset(field.clone()) {
                res.push(quote!(let iter = iter.chain(self.#field_identifier.opt_iter().map(|x| x.map(#fieldtype_identifier::#variant_name)))));
            } else {
                res.push(quote!(let iter = iter.chain(once(self.#field_identifier.map(#fieldtype_identifier::#variant_name)))));
            }
        }
        res
    };
    quote!(
        impl #fieldset_identifier {
            fn opt_iter(self) -> impl Iterator<Item = Option<#fieldtype_identifier>> + Clone + core::fmt::Debug {
                use core::iter::empty;
                use core::iter::once;
                let iter = empty();

                #( #iter_chains ;)*

                iter
            }
        }

        impl IntoIterator for #fieldset_identifier {
            type Item = #fieldtype_identifier;
            type IntoIter = impl Iterator<Item = Self::Item> + Clone + core::fmt::Debug;

            fn into_iter(self) -> Self::IntoIter {
                self.opt_iter().flatten()
            }
        }
    )
    .into()
}

fn common_trait_impl_methods(
    bitset_expr: proc_macro2::TokenStream,
    fields_expr: proc_macro2::TokenStream,
    len_expr: proc_macro2::TokenStream,
    fun_expr: proc_macro2::TokenStream,
    is_bitset: bool,
    name: String,
    fields: FieldsNamed,
) -> proc_macro2::TokenStream {
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let mut res = Vec::new();
    let mut prev_expr = None;
    let mut index: usize = 0;
    for field in fields.named {
        let method_name = get_field_identifier(field.clone());
        let field_name_upper = format_ident!("{}", method_name.to_string().to_upper_camel_case());
        let index_expr = match (prev_expr.clone(), index) {
            (None, 0) => None,
            (None, y) => Some(quote!(#y)),
            (Some(x), 0) => Some(x),
            (Some(x), y) => Some(quote!(#x + #y)),
        };
        if is_fieldset(field.clone()) {
            let type_identifier = get_type_identifier(field.ty);
            let start_expr = match index_expr.clone() {
                Some(x) => quote!(#x),
                None => quote!(),
            };
            let start_bit_expr = if is_bitset {
                match index_expr.clone() {
                    Some(x) => quote!((#x) / 32),
                    None => quote!(),
                }
            } else {
                start_expr.clone()
            };
            let variance_identifier = get_variance_identifier(type_identifier.clone());
            let end_expr = match index_expr.clone() {
                Some(x) => quote!(#x + #variance_identifier),
                None => quote!(#variance_identifier),
            };
            let end_bit_expr = if is_bitset {
                match index_expr.clone() {
                    Some(x) => quote!((#x + #variance_identifier)/32),
                    None => quote!(#variance_identifier / 32),
                }
            } else {
                end_expr.clone()
            };
            prev_expr = Some(end_expr.clone());
            index = 0;
            let field_setter_trait_identifier = format_ident!("{}FieldSetter", type_identifier);
            let setter_name = if is_bitset {
                format_ident!("BitFieldSetter")
            } else {
                format_ident!("PerfFieldSetter")
            };
            res.push(quote!(
                fn #method_name(&mut self) -> impl #field_setter_trait_identifier {
                    let f = #fun_expr;
                    fieldset::#setter_name(
                    &mut #bitset_expr[#start_bit_expr..#end_bit_expr],
                    &mut #fields_expr[#start_expr..#end_expr],
                    &mut #len_expr,
                    move |x|
                            f(#fieldtype_identifier::#field_name_upper(x)))
                }
            ));
        } else {
            let ty = field.ty;
            let index_expr = index_expr.or_else(|| quote!(0usize).into());
            index += 1;
            let leaf_setter_name = if is_bitset {
                format_ident!("BitFieldLeafSetter")
            } else {
                format_ident!("PerfFieldLeafSetter")
            };
            res.push(quote!(
                fn #method_name(&mut self) -> impl fieldset::FieldSetter<#ty> {
                    let f = #fun_expr;
                    fieldset::#leaf_setter_name::<#ty, _, _>(
                        &mut #bitset_expr,
                        &mut #fields_expr,
                        &mut #len_expr,
                        #index_expr, move |x| f(#fieldtype_identifier::#field_name_upper(x)), core::marker::PhantomData)
                }
            ));
        }
    }
    quote!(#(#res )*)
}

fn derive_common_fieldset_setter_trait_impl(
    is_bitset: bool,
    name: String,
    fields: FieldsNamed,
) -> TokenStream {
    let bitset_expr = quote!(self.0);
    let fields_expr = quote!(self.1);
    let len_expr = quote!(self.2);
    let fun_expr = quote!(self.3);
    let trait_identifier = format_ident!("{}FieldSetter", name);
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let methods = common_trait_impl_methods(
        bitset_expr,
        fields_expr,
        len_expr,
        fun_expr,
        is_bitset,
        name,
        fields,
    );
    let setters_name = if is_bitset {
        format_ident!("BitFieldSetter")
    } else {
        format_ident!("PerfFieldSetter")
    };
    quote!(
        impl<'a, T, F: Fn(#fieldtype_identifier) -> T + Copy> #trait_identifier for fieldset::#setters_name<'a, T, F> {
            #methods
        }
    ).into()
}

fn derive_common_fieldset_trait_impl(
    is_bitset: bool,
    name: String,
    fields: FieldsNamed,
) -> TokenStream {
    let bitset_expr = quote!(self.bitset);
    let fields_expr = quote!(self.fields);
    let len_expr = quote!(self.len);
    let fun_expr = quote!(Some);
    let trait_identifier = format_ident!("{}FieldSetter", name);
    let fieldset_identifier = if is_bitset {
        format_ident!("{}BitFieldSet", name)
    } else {
        format_ident!("{}PerfFieldSet", name)
    };
    let methods = common_trait_impl_methods(
        bitset_expr,
        fields_expr,
        len_expr,
        fun_expr,
        is_bitset,
        name,
        fields,
    );
    quote!(
        impl #trait_identifier for #fieldset_identifier {
            #methods
        }

        impl #trait_identifier for &mut #fieldset_identifier {
            #methods
        }
    )
    .into()
}

fn derive_common_fieldset_into_iterator(
    is_bitset: bool,
    name: String,
    _fields: FieldsNamed,
) -> TokenStream {
    let fieldset_identifier = if is_bitset {
        format_ident!("{}BitFieldSet", name)
    } else {
        format_ident!("{}PerfFieldSet", name)
    };
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    quote!(
        impl IntoIterator for #fieldset_identifier {
            type Item = #fieldtype_identifier;
            type IntoIter = impl Iterator<Item = Self::Item> + Clone + core::fmt::Debug;

            fn into_iter(self) -> Self::IntoIter {
                self.fields.into_iter().map_while(|x| x)
            }
        }
    )
    .into()
}

fn derive_bitset_fieldset(name: String, _fields: FieldsNamed) -> TokenStream {
    let identifier = format_ident!("{}", name);
    let fieldset_identifier = format_ident!("{}BitFieldSet", name);
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let fieldset_variance = get_variance_identifier(identifier);
    quote!(
        #[derive(Debug)]
        struct #fieldset_identifier  {
            bitset: [u32 ; (#fieldset_variance + 31) / 32],
            fields: [Option<#fieldtype_identifier> ; #fieldset_variance],
            len: usize,
        }

        impl #fieldset_identifier {
            pub fn new() -> Self {
                Self {
                    bitset: [() ; (#fieldset_variance + 31) / 32].map(|_| 0),
                    fields: [() ; #fieldset_variance].map(|_| None),
                    len: 0,
                }
            }
        }

        impl Default for #fieldset_identifier {
            fn default() -> Self {
                Self::new()
            }
        }

    )
    .into()
}

fn derive_perf_fieldset(name: String, _fields: FieldsNamed) -> TokenStream {
    let identifier = format_ident!("{}", name);
    let fieldset_identifier = format_ident!("{}PerfFieldSet", name);
    let fieldtype_identifier = format_ident!("{}FieldType", name);
    let fieldset_variance = get_variance_identifier(identifier);
    quote!(
        #[derive(Debug)]
        pub struct #fieldset_identifier  {
            bitset: [u16 ; #fieldset_variance],
            fields: [Option<#fieldtype_identifier> ; #fieldset_variance],
            len: usize,
        }

        impl #fieldset_identifier {
            pub fn new() -> Self {
                Self {
                    bitset: [() ; #fieldset_variance].map(|_| 0),
                    fields: [() ; #fieldset_variance].map(|_| None),
                    len: 0,
                }
            }
        }

        impl Default for #fieldset_identifier {
            fn default() -> Self {
                Self::new()
            }
        }
    )
    .into()
}

#[proc_macro_derive(FieldSet, attributes(fieldset))]
pub fn derive_fieldset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if let syn::Data::Struct(ref data) = input.data {
        if let syn::Fields::Named(ref fields) = data.fields {
            let mut result = TokenStream::default();
            result.extend(derive_field_type(input.ident.to_string(), fields.clone()));
            result.extend(derive_into_iterator(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_setter_trait(input.ident.to_string(), fields.clone()));
            result.extend(derive_fieldset_variance(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_raw_fieldset_setter_trait_impl(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_opt_fieldset_type(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_opt_fieldset_setter_trait_impl(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_opt_fieldset_into_iterator(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_bitset_fieldset(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_setter_trait_impl(
                true,
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_trait_impl(
                true,
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_into_iterator(
                true,
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_perf_fieldset(
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_setter_trait_impl(
                false,
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_trait_impl(
                false,
                input.ident.to_string(),
                fields.clone(),
            ));
            result.extend(derive_common_fieldset_into_iterator(
                false,
                input.ident.to_string(),
                fields.clone(),
            ));
            return result;
        }
    }

    TokenStream::from(
        syn::Error::new(
            input.ident.span(),
            "Only structs with named fields can derive `FieldEvents`",
        )
        .to_compile_error(),
    )
}
