use bind_match::bind_match;
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::Span;
use syn::{punctuated::Punctuated, Ident, Path, Type, TypePath};

#[derive(FromDeriveInput, Default)]
pub struct Reprs {
    reprs: Vec<Repr>,
}

impl Reprs {
    pub fn packed(&self) -> bool {
        self.reprs.iter().any(Repr::packed)
    }

    pub fn transparent(&self) -> bool {
        self.reprs.iter().any(Repr::transparent)
    }

    pub fn int(&self) -> Option<&ReprInt> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::Int(repr) => repr))
            .next()
    }

    pub fn align(&self) -> Option<usize> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::Align(Align(align)) => *align))
            .next()
    }

    pub fn rust(&self) -> Option<&RustOrCRepr> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::Rust(repr) => repr))
            .next()
    }

    pub fn c(&self) -> Option<&RustOrCRepr> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::C(repr) => repr))
            .next()
    }
}

#[derive(FromMeta)]
pub enum Repr {
    Int(ReprInt),
    Rust(RustOrCRepr),
    C(RustOrCRepr),
    Packed(Packed),
    Transparent(Transparent),
    Align(Align),
}

impl Repr {
    pub fn packed(&self) -> bool {
        match self {
            Repr::Packed(_) => true,
            Repr::Rust(RustOrCRepr { packed, .. }) | Repr::C(RustOrCRepr { packed, .. })
                if packed.is_some() =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn transparent(&self) -> bool {
        match self {
            Repr::Transparent(_) => true,
            Repr::Rust(RustOrCRepr { transparent, .. })
            | Repr::C(RustOrCRepr { transparent, .. })
                if transparent.is_some() =>
            {
                true
            }
            _ => false,
        }
    }
}

#[derive(FromMeta)]
pub struct Transparent;

#[derive(FromMeta)]
pub struct Packed;

#[derive(FromMeta)]
pub struct Align(usize);

#[derive(FromMeta)]
pub struct RustOrCRepr {
    transparent: Option<Transparent>,
    packed: Option<Packed>,
}

#[derive(Copy, Clone, FromMeta)]
pub enum ReprIntType {
    I8,
    I16,
    I32,
    I64,
    ISize,
    U8,
    U16,
    U32,
    U64,
    USize,
}

#[derive(FromMeta)]
pub struct ReprInt {
    ty: ReprIntType,
}

impl ReprInt {
    pub fn ty(&self, span: Option<Span>) -> Type {
        let ty = match self.ty {
            ReprIntType::I8 => "i8",
            ReprIntType::I16 => "i16",
            ReprIntType::I32 => "i32",
            ReprIntType::I64 => "i64",
            ReprIntType::ISize => "iSize",
            ReprIntType::U8 => "u8",
            ReprIntType::U16 => "u16",
            ReprIntType::U32 => "u32",
            ReprIntType::U64 => "u64",
            ReprIntType::USize => "uSize",
        };
        let mut segments = Punctuated::new();
        segments.push_value(Ident::new(ty, span.unwrap_or_else(Span::call_site)).into());
        Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reprs() {
        let derive_input = syn::parse_str(
            r#"
            #[repr(C, transparent)]
            enum MyEnum {
                A, B(u64),
            }
        "#,
        )
        .unwrap();

        let repr: Reprs = FromDeriveInput::from_derive_input(&derive_input).unwrap();

        assert!(repr.transparent());
        assert!(!repr.packed());
        // assert!(reprs.int().is_none());
        // assert!(reprs.c().is_some());
        // assert!(reprs.rust().is_some());
    }
}
