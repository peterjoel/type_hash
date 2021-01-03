use either::Either;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Fields, Generics, Ident,
};

#[proc_macro_derive(TypeHash)]
pub fn derive_type_hash(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    type_hash_impl(input).into()
}

fn type_hash_impl(input: DeriveInput) -> TokenStream {
    match &input.data {
        Data::Struct(data) => type_hash_struct(&input.ident, &input.generics, data),
        Data::Enum(data) => type_hash_enum(&input.ident, &input.generics, data),
        Data::Union(data) => type_hash_union(&input.ident, data),
    }
}

// TODO handle generics
fn type_hash_struct(ident: &Ident, _generics: &Generics, data: &DataStruct) -> TokenStream {
    let name = ident.to_string();
    let fields = write_field_hashes(&data.fields).into_iter().flatten();
    quote! {
        impl type_hash::TypeHash for #ident {
            fn write_hash(hasher: &mut impl std::hash::Hasher) {
                hasher.write(#name.as_bytes());
                #(#fields)*
            }
        }
    }
}

// TODO handle generics
fn type_hash_enum(ident: &Ident, _generics: &Generics, data: &DataEnum) -> TokenStream {
    let name = ident.to_string();
    let variants = data.variants.iter().flat_map(|v| {
        v.discriminant
            .iter()
            .map(|(_, discriminant)| {
                quote! {
                    <isize as std::hash::Hash>::hash(&(#discriminant), hasher);
                }
            })
            .chain(write_field_hashes(&v.fields).into_iter().flatten())
    });
    quote! {
        impl type_hash::TypeHash for #ident {
            fn write_hash(hasher: &mut impl std::hash::Hasher) {
                hasher.write(#name.as_bytes());
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

// TODO Support unions
fn type_hash_union(_ident: &Ident, data: &DataUnion) -> TokenStream {
    quote_spanned! {
        data.union_token.span()=>
        compile_error!("Unions are not currently supported");
    }
}
