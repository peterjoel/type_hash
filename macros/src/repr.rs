use bind_match::bind_match;
use proc_macro2::Span;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::convert::TryFrom;
use std::str::FromStr;
use syn::{punctuated::Punctuated, AttrStyle, Attribute, Ident, Path, Type, TypePath};
use thiserror::Error;

#[derive(Default)]
pub struct Reprs {
    reprs: Vec<Repr>,
}

impl<'a> TryFrom<&'a [Attribute]> for Reprs {
    type Error = ReprError;
    fn try_from(attrs: &'a [Attribute]) -> Result<Self, Self::Error> {
        let reprs = attrs
            .iter()
            .map(Repr::try_from)
            .filter_map(Result::ok)
            .collect();
        Ok(Reprs { reprs })
    }
}

impl Reprs {
    pub fn packed(&self) -> bool {
        self.reprs.iter().any(Repr::packed)
    }

    pub fn transparent(&self) -> bool {
        self.reprs.iter().any(Repr::transparent)
    }

    pub fn int(&self) -> Option<ReprInt> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::Rust(RustRepr { int_type: Some(ty),.. }) =>*ty))
            .next()
    }

    pub fn align(&self) -> Option<usize> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r,Repr::Rust(RustRepr { align: Some(align),.. }) => *align))
            .next()
    }

    pub fn c(&self) -> Option<&CRepr> {
        self.reprs
            .iter()
            .filter_map(|r| bind_match!(r, Repr::C(repr) => repr))
            .next()
    }
}

pub enum Repr {
    Rust(RustRepr),
    C(CRepr),
}

impl Repr {
    pub fn packed(&self) -> bool {
        match self {
            Repr::Rust(RustRepr { packed, .. }) => *packed,
            Repr::C(CRepr { packed, .. }) => *packed,
        }
    }

    pub fn transparent(&self) -> bool {
        match self {
            Repr::Rust(RustRepr { transparent, .. }) => *transparent,
            Repr::C(CRepr { transparent, .. }) => *transparent,
        }
    }
}

#[derive(Error, Debug)]
pub enum ReprError {
    #[error("Not a repr attribute")]
    NotARepr,
    #[error("Repr must be an outer attribute")]
    MustBeOuter,
    #[error("Invalid repr type")]
    InvalidType,
    #[error("Not valid attribute syntax")]
    SyntaxError,
}

impl<'a> TryFrom<&'a Attribute> for Repr {
    type Error = ReprError;
    fn try_from(att: &'a Attribute) -> Result<Self, Self::Error> {
        if let AttrStyle::Inner(_) = att.style {
            return Err(ReprError::MustBeOuter);
        } else if att.path.segments.len() == 1 {
            if let Some(segment) = att.path.segments.iter().next() {
                if segment.ident.to_string() == "repr" {
                    return parse_repr(att.tokens.clone());
                }
            }
        }
        Err(ReprError::NotARepr)
    }
}

fn parse_repr(tokens: TokenStream) -> Result<Repr, ReprError> {
    let mut it = tokens.into_iter();
    let mut is_c = false;
    let mut transparent = false;
    let mut packed = false;
    let mut align = None;
    let mut int_type = None;

    while let Some(tt) = it.next() {
        if let TokenTree::Punct(p) = &tt {
            if p.as_char() == ',' {
                continue;
            }
        } else if let TokenTree::Ident(ident) = tt {
            let name = ident.to_string();
            match name.as_str() {
                "c" => is_c = true,
                "transparent" => transparent = true,
                "packed" => packed = true,
                "align" => {
                    if let Some(TokenTree::Group(group)) = it.next() {
                        if let Delimiter::Parenthesis = group.delimiter() {
                            if let Some(tt) = group.stream().into_iter().next() {
                                if let TokenTree::Literal(lit) = tt {
                                    if let Some(size) = lit.to_string().parse::<usize>().ok() {
                                        align = Some(size);
                                    }
                                }
                            }
                        }
                        // no error, be forgiving
                    }
                }
                other => {
                    if let Ok(ty) = ReprInt::from_str(other) {
                        int_type = Some(ty);
                    }
                    // no error, be forgiving
                }
            }
        } else {
            return Err(ReprError::SyntaxError);
        }
    }
    if is_c {
        Ok(Repr::C(CRepr {
            transparent,
            packed,
        }))
    } else if transparent || packed || align.is_some() || int_type.is_some() {
        Ok(Repr::Rust(RustRepr {
            transparent,
            packed,
            align,
            int_type,
        }))
    } else {
        Err(ReprError::NotARepr)
    }
}

pub struct CRepr {
    transparent: bool,
    packed: bool,
}

pub struct RustRepr {
    transparent: bool,
    packed: bool,
    align: Option<usize>,
    int_type: Option<ReprInt>,
}

#[derive(Copy, Clone)]
pub enum ReprInt {
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

impl Default for ReprInt {
    fn default() -> Self {
        ReprInt::ISize
    }
}

impl FromStr for ReprInt {
    type Err = ReprError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(match input {
            "u8" => ReprInt::U8,
            "i8" => ReprInt::I8,
            "u16" => ReprInt::U16,
            "i16" => ReprInt::I16,
            "u32" => ReprInt::U32,
            "i32" => ReprInt::I32,
            "u64" => ReprInt::U64,
            "i64" => ReprInt::I64,
            "usize" => ReprInt::USize,
            "isize" => ReprInt::ISize,
            _ => return Err(ReprError::InvalidType),
        })
    }
}

impl ReprInt {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReprInt::I8 => "i8",
            ReprInt::I16 => "i16",
            ReprInt::I32 => "i32",
            ReprInt::I64 => "i64",
            ReprInt::ISize => "isize",
            ReprInt::U8 => "u8",
            ReprInt::U16 => "u16",
            ReprInt::U32 => "u32",
            ReprInt::U64 => "u64",
            ReprInt::USize => "usize",
        }
    }

    pub fn ty(&self, span: Option<Span>) -> Type {
        let mut segments = Punctuated::new();
        segments.push_value(Ident::new(self.as_str(), span.unwrap_or_else(Span::call_site)).into());
        Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments,
            },
        })
    }
}
