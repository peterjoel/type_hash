mod repr;
use either::Either;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use repr::Reprs;
use std::convert::TryFrom;
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Fields, GenericParam, Generics, Ident, ImplGenerics, TypeGenerics,
};

#[proc_macro_derive(TypeHash)]
pub fn derive_type_hash(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    type_hash_impl(input).into()
}

fn type_hash_impl(input: DeriveInput) -> TokenStream {
    let reprs = Reprs::try_from(input.attrs.as_slice()).unwrap_or_default();
    match &input.data {
        Data::Struct(data) => type_hash_struct(&input.ident, &reprs, &input.generics, data),
        Data::Enum(data) => type_hash_enum(&input.ident, &reprs, &input.generics, data),
        Data::Union(data) => type_hash_union(&input.ident, &reprs, data),
    }
}

fn type_hash_struct(
    ident: &Ident,
    reprs: &Reprs,
    generics: &Generics,
    data: &DataStruct,
) -> TokenStream {
    let name = ident.to_string();
    let fields = write_field_hashes(&data.fields).into_iter().flatten();
    let (impl_generics, ty_generics, where_clause) = split_generics(generics);
    let reprs = write_repr_hashes(&reprs);
    quote! {
        impl#impl_generics type_hash::TypeHash for #ident#ty_generics #where_clause {
            fn write_hash(hasher: &mut impl std::hash::Hasher) {
                hasher.write(#name.as_bytes());
                #reprs
                #(#fields)*
            }
        }
    }
}

fn type_hash_enum(
    ident: &Ident,
    reprs: &Reprs,
    generics: &Generics,
    data: &DataEnum,
) -> TokenStream {
    let name = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = split_generics(generics);
    let variants = data.variants.iter().flat_map(|v| {
        v.discriminant
            .iter()
            .map(|(_, discriminant)| {
                let discriminant_type = reprs.int().unwrap_or_default().ty(None);
                quote! {
                    <#discriminant_type as std::hash::Hash>::hash(&(#discriminant), hasher);
                }
            })
            .chain(write_field_hashes(&v.fields).into_iter().flatten())
    });
    let reprs = write_repr_hashes(&reprs);
    quote! {
        impl#impl_generics type_hash::TypeHash for #ident#ty_generics #where_clause{
            fn write_hash(hasher: &mut impl std::hash::Hasher) {
                hasher.write(#name.as_bytes());
                #reprs
                #(#variants)*
            }
        }
    }
}

fn write_field_hashes(fields: &Fields) -> Option<impl Iterator<Item = TokenStream> + '_> {
    match &fields {
        Fields::Unit => None,
        Fields::Named(fields) => {
            Some(Either::Left(fields.named.iter().map(|f| {
                // safe to unwrap because we've matched named fields
                let field_name = f.ident.as_ref().unwrap().to_string();
                let field_type = &f.ty;
                quote! {
                    hasher.write(#field_name.as_bytes());
                    <#field_type as type_hash::TypeHash>::write_hash(hasher);
                }
            })))
        }
        Fields::Unnamed(fields) => Some(Either::Right(fields.unnamed.iter().map(|f| {
            let field_type = f.ty.clone();
            quote! {
                <#field_type as type_hash::TypeHash>::write_hash(hasher);
            }
        }))),
    }
}

fn write_repr_hashes(reprs: &Reprs) -> TokenStream {
    let packed = reprs.packed();
    let transparent = reprs.transparent();
    let int_type = reprs.int().into_iter().map(|t| t.as_str());
    let align = reprs.align().into_iter();
    let c = reprs.c().is_some();
    quote! {
        std::hash::Hash::hash(&#packed, hasher);
        std::hash::Hash::hash(&#transparent, hasher);
        #(hasher.write(#int_type.as_bytes());)*
        #(std::hash::Hash::hash(&#align, hasher);)*
        std::hash::Hash::hash(&#c, hasher);
    }
}

// TODO Support unions
fn type_hash_union(_ident: &Ident, _reprs: &Reprs, data: &DataUnion) -> TokenStream {
    quote_spanned! {
        data.union_token.span()=>
        compile_error!("Unions are not currently supported");
    }
}

struct DeriveWhereClause<'a>(&'a Generics);

impl<'a> ToTokens for DeriveWhereClause<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut params = self
            .0
            .params
            .iter()
            .filter_map(|param| {
                if let GenericParam::Type(type_param) = param {
                    Some(&type_param.ident)
                } else {
                    None
                }
            })
            .peekable();
        if params.peek().is_some() {
            let where_clause = quote! {
                where #(#params: type_hash::TypeHash),*
            };
            where_clause.to_tokens(tokens);
        }
    }
}

fn split_generics(generics: &Generics) -> (ImplGenerics, TypeGenerics, DeriveWhereClause<'_>) {
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let where_clause = DeriveWhereClause(generics);
    (impl_generics, ty_generics, where_clause)
}
