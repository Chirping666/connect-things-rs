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
            [alice,bob],
            Friends
        );
    
    // What is the nature of the relationship
    // between Alice and Bob?
    
    let alice = things.find_thing(|thing| {
       thing.access_data(|data| {
            matches!(data,Alice)
        })
    });
    
    let friends = alice.find_connection(|connection| {
       matches!(connection.directed_towards(),Bob)
    }).unwrap();
}


```

### Todo
- Make documentation & better examples.
- Optional: add more methods to interact with things and connections.
