use either::Either;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Field,
    Fields, GenericParam, Generics, Ident, ImplGenerics, Lit, Meta, MetaNameValue, Type,
    TypeGenerics, WhereClause,
};

#[proc_macro_derive(TypeHash, attributes(type_hash))]
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
                match field_attrs(f) {
                    Ok(Some(tokens)) => tokens,
                    Ok(None) => TokenStream::new(),
                    Err(tokens) => tokens,
                }
                // // safe to unwrap because we've matched named fields
                // let field_name = f.ident.as_ref().unwrap().to_string();
                // let field_type = field_type(&f);
                // quote! {
                //     hasher.write(#field_name.as_bytes());
                //     <#field_type as type_hash::TypeHash>::write_hash(hasher);
                // }
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

// TODO: This is gnarly. Use something like darling to parse the attributes more cleanly
fn field_attrs(field: &Field) -> Result<Option<TokenStream>, TokenStream> {
    // safe to unwrap because this is only used for named fields
    let field_name = field.ident.as_ref().unwrap().to_token_stream().to_string();
    for att in &field.attrs {
        if let Some(name) = att.path.get_ident() {
            if name == "type_hash" {
                match att.parse_args() {
                    Ok(Meta::NameValue(MetaNameValue { path, lit, .. })) => {
                        if let (Some(name), Lit::Str(val)) = (path.get_ident(), lit) {
                            if name == "as" {
                                if let Ok(ty) = val.parse::<Type>() {
                                    return Ok(Some(quote! {
                                        hasher.write(#field_name.as_bytes());
                                        <#ty as type_hash::TypeHash>::write_hash(hasher);
                                    }));
                                } else {
                                    return Err(quote_spanned! {
                                        val.span()=>
                                        compile_error!("Invalid type");
                                    });
                                }
                            }
                        }
                        return Err(quote_spanned! {
                            att.span()=>
                            compile_error!("Unsupported metadata");
                        });
                    }
                    Ok(Meta::Path(path)) => {
                        if let Some(name) = path.get_ident() {
                            if name == "skip" {
                                return Ok(None);
                            } else if name == "foreign_type" {
                                let type_str = field.ty.to_token_stream().to_string();
                                return Ok(Some(quote! {
                                    hasher.write(#field_name.as_bytes());
                                    hasher.write(#type_str.as_bytes());
                                }));
                            }
                        }
                        return Err(quote_spanned! {
                            name.span()=>
                            compile_error!("Unsupported metadata");
                        });
                    }
                    Ok(m) => {
                        return Err(quote_spanned! {
                            m.span()=>
                            compile_error!("Unsupported metadata");
                        });
                    }
                    Err(e) => {
                        let e = e.to_string();
                        return Err(quote_spanned! {
                            att.span()=>
                            compile_error!("{}", #e);
                        });
                    }
                }
            }
        }
    }
    let field_type = &field.ty;
    Ok(Some(quote! {
        hasher.write(#field_name.as_bytes());
        <#field_type as type_hash::TypeHash>::write_hash(hasher);
    }))
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
