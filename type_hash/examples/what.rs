use type_hash::TypeHash;

#[derive(TypeHash)]
#[repr(C)]
pub struct Foo {
    a: i64,
    b: String,
}

#[derive(TypeHash)]
#[repr(u8)]
pub enum Foob {
    X = 1,
    Y,
}

fn main() {}
