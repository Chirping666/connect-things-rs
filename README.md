# Connect Things
A `no_std` + `alloc` compatible and threadsafe lib for connecting things to other things.

## Example
```rust
use connect_things::*;

enum MyData {
    Alice,
    Bob,
    Friends
}

fn main() {
    use MyData::*;

    let mut things_and_connections
        = ThingsAndConnections::<MyData>::new();

    let alice: Arc<RwLock<MyData>>
        = things_and_connections.new_thing(Alice);
    
    let bob = things_and_connections.new_thing(Bob);
    
    /// Will block the thread until the connection
    /// can be added.
    let friendship = things_and_connections
        .new_undirected_connection(
            [alice.clone(),bob.clone()],
            Friends
        ); 
}


```

### Todo
- Make documentation.
- Possibly add more methods to interact with things and connections.
