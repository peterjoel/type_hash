use either::Either;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Fields, GenericParam, Generics, Ident, ImplGenerics, TypeGenerics, WhereClause,
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

fn type_hash_struct(ident: &Ident, generics: &Generics, data: &DataStruct) -> TokenStream {
    let name = ident.to_string();
    let fields = write_field_hashes(&data.fields).into_iter().flatten();
    let (impl_generics, ty_generics, where_clause) = split_generics(generics);
    quote! {
        impl#impl_generics type_hash::TypeHash for #ident#ty_generics #where_clause {
            fn write_hash(hasher: &mut impl std::hash::Hasher) {
                hasher.write(#name.as_bytes());
                #(#fields)*
            }
        }
    }
}

fn type_hash_enum(ident: &Ident, generics: &Generics, data: &DataEnum) -> TokenStream {
    let name = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = split_generics(generics);
    let variants = data.variants.iter().flat_map(|v| {
        v.discriminant
            .iter()
            .map(|(_, discriminant)| {
                quote! {
                    std::hash::Hash::hash(&(#discriminant as isize), hasher);
                }
            })
            .chain(write_field_hashes(&v.fields).into_iter().flatten())
    });
    quote! {
        impl#impl_generics type_hash::TypeHash for #ident#ty_generics #where_clause{
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

struct DeriveWhereClause<'a>(&'a Generics, Option<&'a WhereClause>);

impl<'a> ToTokens for DeriveWhereClause<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let generics = &self.0;
        let mut predicates = self.1.iter().flat_map(|w| &w.predicates).peekable();
        let mut params = generics
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
        if params.peek().is_some() || predicates.next().is_some() {
            let clause = quote! {
                where #(#params: type_hash::TypeHash,)* #(#predicates,)*
            };
            clause.to_tokens(tokens);
        }
    }
}

fn split_generics(generics: &Generics) -> (ImplGenerics, TypeGenerics, DeriveWhereClause<'_>) {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let where_clause = DeriveWhereClause(&generics, where_clause);
    (impl_generics, ty_generics, where_clause)
}
