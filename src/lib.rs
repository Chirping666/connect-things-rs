#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use alloc::vec::Vec;

use core::cell::RefCell;

pub struct Thing<T, C> {
    inner: Rc<RefCell<ThingInner<T, C>>>,
}

struct ThingInner<T, C> {
    connections: Vec<Connection<T, C>>,
    data: T,
    exists: bool,
}

impl<T, C> ThingInner<T, C> {
    pub fn new(data: T) -> Self {
        ThingInner {
            connections: Vec::new(),
            data,
            exists: true,
        }
    }

    fn get_data(&self) -> &T {
        &self.data
    }

    fn get_data_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T, C> Thing<T, C> {
    pub fn new(data: T) -> Self {
        Thing {
            inner: Rc::new(RefCell::new(ThingInner::new(data))),
        }
    }

    pub fn add_connection(&self, connection: Connection<T, C>) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.push(connection);
    }

    pub fn find_connection(&self, find: fn(&Connection<T, C>) -> bool) -> Option<Connection<T, C>> {
        let inner = self.inner.try_borrow().unwrap();
        for conn in inner.connections.iter() {
            if find(conn) {
                return Some((*conn).clone());
            }
        }
        None
    }

    pub fn find_connections(&self, find: fn(&Connection<T, C>) -> bool) -> Vec<Connection<T, C>> {
        let mut connections = Vec::new();
        let inner = self.inner.borrow();
        for conn in inner.connections.iter() {
            if find(conn) {
                connections.push(conn.clone())
            }
        }
        connections
    }

    pub fn remove_connections(&mut self, remove: fn(&Connection<T, C>) -> bool) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.retain(|c| !remove(c))
    }

    pub fn access_data<R>(&self, access: fn(&T) -> R) -> R {
        let inner = self.inner.try_borrow().unwrap();
        access(inner.get_data())
    }

    pub fn access_data_mut<R>(&self, access: fn(&mut T) -> R) -> R {
        let mut inner = self.inner.borrow_mut();
        access(inner.get_data_mut())
    }

    fn exists(&self) -> bool {
        let inner = self.inner.borrow();
        inner.exists
    }

    fn kill(&self) {
        let mut inner = self.inner.borrow_mut();
        for connection in inner.connections.iter() {
            connection.kill();
        }
        inner.exists = false;
    }
}

impl<T, C> Clone for Thing<T, C> {
    fn clone(&self) -> Self {
        Thing {
            inner: self.inner.clone(),
        }
    }
}

pub struct Connection<T, C> {
    inner: Rc<RefCell<ConnectionInner<T, C>>>,
}

enum ConnectionInner<T, C> {
    Directed {
        from: Thing<T, C>,
        to: Thing<T, C>,
        data: C,
        exists: bool,
    },
    Undirected {
        things: [Thing<T, C>; 2],
        data: C,
        exists: bool,
    },
}

impl<T, C> ConnectionInner<T, C> {
    fn new_directed(from: Thing<T, C>, to: Thing<T, C>, data: C) -> Self {
        Self::Directed {
            from,
            to,
            data,
            exists: true,
        }
    }

    fn new_undirected(things: [Thing<T, C>; 2], data: C) -> Self {
        Self::Undirected {
            things,
            data,
            exists: true,
        }
    }

    fn get_things(&self) -> [Thing<T, C>; 2] {
        match self {
            &ConnectionInner::Directed {
                ref from, ref to, ..
            } => [from.clone(), to.clone()],
            &ConnectionInner::Undirected { ref things, .. } => {
                [things[0].clone(), things[1].clone()]
            }
        }
    }

    fn get_data(&self) -> &C {
        match self {
            &ConnectionInner::Directed { ref data, .. } => data,
            &ConnectionInner::Undirected { ref data, .. } => data,
        }
    }

    fn get_data_mut(&mut self) -> &mut C {
        match self {
            &mut ConnectionInner::Directed { ref mut data, .. } => data,
            &mut ConnectionInner::Undirected { ref mut data, .. } => data,
        }
    }

    fn exists(&self) -> bool {
        match self {
            &ConnectionInner::Directed { ref exists, .. } => *exists,
            &ConnectionInner::Undirected { ref exists, .. } => *exists,
        }
    }

    fn kill(&mut self) {
        match self {
            &mut ConnectionInner::Directed { ref mut exists, .. } => {
                *exists = false;
            }
            &mut ConnectionInner::Undirected { ref mut exists, .. } => {
                *exists = false;
            }
        }
    }
}

impl<T, C> Connection<T, C> {
    pub fn new_directed(from: Thing<T, C>, to: Thing<T, C>, data: C) -> Connection<T, C> {
        Connection {
            inner: Rc::new(RefCell::new(ConnectionInner::new_directed(from, to, data))),
        }
    }

    pub fn new_undirected(things: [Thing<T, C>; 2], data: C) -> Connection<T, C> {
        Connection {
            inner: Rc::new(RefCell::new(ConnectionInner::new_undirected(things, data))),
        }
    }

    pub fn is_directed(&self) -> bool {
        let inner = self.inner.borrow();
        matches!(*inner, ConnectionInner::Directed { .. })
    }

    pub fn is_undirected(&self) -> bool {
        let inner = self.inner.borrow();
        matches!(*inner, ConnectionInner::Undirected { .. })
    }

    pub fn access_data<R>(&self, access: fn(&C) -> R) -> R {
        let inner = self.inner.borrow();
        access(inner.get_data())
    }

    pub fn access_data_mut<R>(&self, access: fn(&mut C) -> R) -> R {
        let mut inner = self.inner.borrow_mut();
        access(inner.get_data_mut())
    }

    pub fn connected_things(&self) -> [Thing<T, C>; 2] {
        let inner = self.inner.borrow();
        inner.get_things().clone()
    }

    pub fn directed_from(&self) -> Thing<T, C> {
        let inner = self.inner.borrow();
        inner.get_things()[0].clone()
    }

    pub fn directed_towards(&self) -> Thing<T, C> {
        let inner = self.inner.borrow();
        inner.get_things()[1].clone()
    }

    fn exists(&self) -> bool {
        let inner = self.inner.borrow();
        inner.exists()
    }

    fn kill(&self) {
        let mut inner = self.inner.borrow_mut();
        let things = inner.get_things();
        for thing in things {
            thing.kill();
        }
        inner.kill();
    }
}

impl<T, C> Clone for Connection<T, C> {
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
        }
    }
}

pub struct Things<T, C> {
    things: Vec<Thing<T, C>>,
    connections: Vec<Connection<T, C>>,
}

impl<T, C> Things<T, C> {
    pub fn new() -> Things<T, C> {
        Things {
            things: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn new_thing(&mut self, data: T) -> Thing<T, C> {
        let thing = Thing::<T, C>::new(data);
        self.things.push(thing.clone());
        thing
    }

    pub fn new_directed_connection(
        &mut self,
        from: Thing<T, C>,
        to: Thing<T, C>,
        data: C,
    ) -> Connection<T, C> {
        let connection = Connection::<T, C>::new_directed(from.clone(), to.clone(), data);
        from.add_connection(connection.clone());
        to.add_connection(connection.clone());
        self.connections.push(connection.clone());
        connection
    }

    pub fn new_undirected_connection(
        &mut self,
        things: [Thing<T, C>; 2],
        data: C,
    ) -> Connection<T, C> {
        let connection = Connection::<T, C>::new_undirected(things.clone(), data);
        things[0].add_connection(connection.clone());
        things[1].add_connection(connection.clone());
        self.connections.push(connection.clone());
        connection
    }

    pub fn find_thing(&self, search: fn(&Thing<T, C>) -> bool) -> Option<Thing<T, C>> {
        for thing in &self.things {
            if search(thing) {
                return Some(thing.clone());
            }
        }
        None
    }

    pub fn find_things(&self, find: fn(&Thing<T, C>) -> bool) -> Vec<Thing<T, C>> {
        let mut things = Vec::new();
        for thing in &self.things {
            if find(thing) {
                things.push(thing.clone());
            }
        }
        things
    }

    pub fn remove_things(&mut self, remove: fn(&Thing<T, C>) -> bool) {
        self.things.retain(|thing| {
            if remove(thing) {
                thing.kill();
                false
            } else {
                true
            }
        });
    }

    pub fn find_connection(
        &self,
        search: fn(&Connection<T, C>) -> bool,
    ) -> Option<Connection<T, C>> {
        for connection in &self.connections {
            if search(connection) {
                return Some(connection.clone());
            }
        }
        None
    }

    pub fn find_connections(&self, search: fn(&Connection<T, C>) -> bool) -> Vec<Connection<T, C>> {
        let mut connections = Vec::new();
        for connection in &self.connections {
            if search(connection) {
                connections.push(connection.clone());
            }
        }
        connections
    }

    pub fn remove_connections(&mut self, delete: fn(&Connection<T, C>) -> bool) {
        self.connections.retain(|connection| {
            if delete(connection) {
                connection.kill();
                false
            } else {
                true
            }
        });
    }

    pub fn clean(&mut self) {
        self.things.retain(|thing| thing.exists());

        self.connections.retain(|connection| connection.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    fn test_graph<'a>() -> Things<&'a str, &'a str> {
        let mut graph = Things::<&str, &str>::new();

        let apple = graph.new_thing("Apple");

        let apples = graph.new_thing("Apples");

        graph.new_directed_connection(apples.clone(), apple.clone(), "plural of");

        let pear = graph.new_thing("Pear");

        let pears = graph.new_thing("Pears");

        graph.new_directed_connection(pears.clone(), pear.clone(), "plural of");

        let alice = graph.new_thing("Alice");

        graph.new_directed_connection(alice.clone(), apples, "likes to eat");

        graph.new_directed_connection(alice, pears, "doesn't like to eat");

        let fruit = graph.new_thing("Fruit");

        graph.new_directed_connection(apple, fruit.clone(), "is");

        graph.new_directed_connection(pear, fruit, "is");

        graph
    }

    #[test]
    fn it_works() {
        let graph = test_graph();

        // What does Alice like to eat?

        let alice = graph
            .find_thing(|thing| thing.access_data(|data| *data == "Alice"))
            .unwrap();

        let apple = alice
            .find_connection(|connection| connection.access_data(|data| *data == "likes to eat"))
            .unwrap()
            .directed_towards();

        let answer = format!(
            "The thing alice likes to eat is: {}.",
            apple
                .access_data(|data| { *data })
                .to_ascii_lowercase()
        );

        assert_eq!("The thing alice likes to eat is: apples.", &answer);
    }
}
