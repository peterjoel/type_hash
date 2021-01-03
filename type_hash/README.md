# type_hash

Generate a hash for a Rust type.

The primary use-case for this crate is for detecting differences in message
types between versions of a crate.

The `TypeHash` trait is implemented for most built-in types and a derive macro
is provided, for implementing it for your own types.

## Examples

```rust
use type_hash::TypeHash;

#[derive(TypeHash)]
pub enum Message {
    LaunchMissiles { destination: String },
    CancelMissiles,
}

fn main() {
    let hash = Message::type_hash();
    // this will only change if the type definition changes
    assert_eq!(hash, 11652809455620829461);
}

```
