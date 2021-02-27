#![allow(unused)]
use type_hash::TypeHash;

#[test]
fn type_hashes_same_for_skipped_field_as_type_without_field() {
    assert_eq!(v1::MyStruct::type_hash(), v2::MyStruct::type_hash(),);
}

mod v1 {
    use type_hash::TypeHash;
    #[derive(TypeHash)]
    pub struct MyStruct {
        #[type_hash(skip)]
        a: bool,
        b: u32,
    }
}

mod v2 {
    use type_hash::TypeHash;
    #[derive(TypeHash)]
    pub struct MyStruct {
        b: u32,
    }
}
