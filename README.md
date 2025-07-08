# Connect Things
A `no_std` + `alloc` compatible crate for connecting things to other things.

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

    let alice: Rc<RefCell<MyData>>
        = things_and_connections.new_thing(Alice);
    
    let bob = things_and_connections.new_thing(Bob);
    
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
- Make threadsafe, using `lock_api`.
- Make `alloc` use optional (maybe).
