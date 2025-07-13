# Connect Things
A `no_std` + `alloc` compatible crate for connecting things to other things.

## Example
```rust
use connect_things::*;

enum ThingData {
    Alice,
    Bob
}

enum ConnectionData {
    Friends
}

fn main() {
    use ThingData::*;
    use ConnectionData::*;

    let mut things = Things::<ThingData,ConnectionData>::new();

    let alice = things_and_connections.new_thing(Alice);
    
    let bob = things_and_connections.new_thing(Bob);
    
    let relationship = things_and_connections
        .new_undirected_connection(
            [alice.clone(),bob.clone()],
            Friends
        ); 
}


```

### Todo
- Make documentation & better examples.
- Optional: add more methods to interact with things and connections.
