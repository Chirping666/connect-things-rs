#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::{BorrowMutError, Ref, RefCell};

pub struct ThingsAndConnections<D> {
    things: Vec<Rc<RefCell<Thing<D>>>>,
    connection: Vec<Rc<RefCell<Connection<D>>>>,
}

impl<D> ThingsAndConnections<D> {
    pub fn new() -> ThingsAndConnections<D> {
        ThingsAndConnections {
            things: Vec::new(),
            connection: Vec::new(),
        }
    }

    pub fn new_thing(&mut self, data: D) -> Rc<RefCell<Thing<D>>> {
        let node = Rc::new(RefCell::new(Thing::new(data)));
        self.things.push(node.clone());
        node
    }

    pub fn new_directed_connection(
        &mut self,
        from: Rc<RefCell<Thing<D>>>,
        to: Rc<RefCell<Thing<D>>>,
        data: D,
    ) -> Result<Rc<RefCell<Connection<D>>>, BorrowMutError> {
        let edge = Rc::new(RefCell::new(Connection::new_directed(
            from.clone(),
            to.clone(),
            data,
        )));
        let mut first = from.try_borrow_mut()?;
        let mut second = to.try_borrow_mut()?;
        first.add_connection(edge.clone());
        second.add_connection(edge.clone());
        self.connection.push(edge.clone());
        Ok(edge)
    }

    pub fn new_undirected_connection(
        &mut self,
        nodes: [Rc<RefCell<Thing<D>>>; 2],
        data: D,
    ) -> Result<Rc<RefCell<Connection<D>>>, BorrowMutError> {
        let edge = Rc::new(RefCell::new(Connection::new_undirected(
            [nodes[0].clone(), nodes[1].clone()],
            data,
        )));
        let mut first = nodes[0].try_borrow_mut()?;
        let mut second = nodes[1].try_borrow_mut()?;
        first.add_connection(edge.clone());
        second.add_connection(edge.clone());
        self.connection.push(edge.clone());
        Ok(edge)
    }

    pub fn find_thing(&self, finder: fn(Ref<Thing<D>>) -> bool) -> Option<Rc<RefCell<Thing<D>>>> {
        for node in self.things.iter() {
            if let Ok(node_ref) = node.try_borrow() {
                if finder(node_ref) {
                    return Some(node.clone());
                }
            }
        }
        None
    }

    pub fn find_connection(&self, finder: fn(Ref<Connection<D>>) -> bool) -> Option<Rc<RefCell<Connection<D>>>> {
        for edge in self.connection.iter() {
            if let Ok(edge_ref) = edge.try_borrow() {
                if finder(edge_ref) {
                    return Some(edge.clone());
                }
            }
        }
        None
    }

    pub fn filter_things(&self, filter: fn(Ref<Thing<D>>) -> bool) -> Vec<Rc<RefCell<Thing<D>>>> {
        let mut nodes = Vec::new();
        for node in self.things.iter() {
            if let Ok(node_ref) = node.try_borrow() {
                if filter(node_ref) {
                    nodes.push(node.clone());
                }
            }
        }
        nodes
    }

    pub fn filter_connections(&self, filter: fn(Ref<Connection<D>>) -> bool) -> Vec<Rc<RefCell<Connection<D>>>> {
        let mut edges = Vec::new();
        for edge in self.connection.iter() {
            if let Ok(edge_ref) = edge.try_borrow() {
                if filter(edge_ref) {
                    edges.push(edge.clone());
                }
            }
        }
        edges
    }
}

pub struct Thing<D> {
    connections: Vec<Rc<RefCell<Connection<D>>>>,
    data: D,
}

impl<D> Thing<D> {
    pub fn new(data: D) -> Self {
        Thing {
            connections: Vec::new(),
            data,
        }
    }

    pub fn add_connection(&mut self, edge: Rc<RefCell<Connection<D>>>) {
        self.connections.push(edge);
    }

    pub fn get_connections(&self) -> &[Rc<RefCell<Connection<D>>>] {
        &self.connections
    }

    pub fn find_connection(&self, finder: fn(Ref<Connection<D>>) -> bool) -> Option<Rc<RefCell<Connection<D>>>> {
        for edge in self.connections.iter() {
            if let Ok(edge_ref) = edge.try_borrow() {
                if finder(edge_ref) {
                    return Some(edge.clone());
                }
            }
        }
        None
    }

    pub fn get_data(&self) -> &D {
        &self.data
    }

    pub fn get_data_mut(&mut self) -> &mut D {
        &mut self.data
    }
}

pub enum Connection<D> {
    Directed {
        from: Rc<RefCell<Thing<D>>>,
        to: Rc<RefCell<Thing<D>>>,
        data: D,
    },
    Undirected {
        nodes: [Rc<RefCell<Thing<D>>>; 2],
        data: D,
    },
}

impl<D> Connection<D> {
    pub fn new_directed(from: Rc<RefCell<Thing<D>>>, to: Rc<RefCell<Thing<D>>>, data: D) -> Self {
        Connection::Directed { from, to, data }
    }

    pub fn new_undirected(nodes: [Rc<RefCell<Thing<D>>>; 2], data: D) -> Self {
        Connection::Undirected { nodes, data }
    }

    pub fn get_things(&self) -> (Rc<RefCell<Thing<D>>>, Rc<RefCell<Thing<D>>>) {
        match self {
            Connection::Directed { from, to, .. } => (from.clone(), to.clone()),
            Connection::Undirected { nodes, .. } => (nodes[0].clone(), nodes[1].clone()),
        }
    }

    pub fn get_data(&self) -> &D {
        match self {
            Connection::Directed { data, .. } => data,
            Connection::Undirected { data, .. } => data,
        }
    }

    pub fn get_data_mut(&mut self) -> &mut D {
        match self {
            Connection::Directed { data, .. } => data,
            Connection::Undirected { data, .. } => data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let alice = Rc::new(RefCell::new(Thing::new("Alice")));
        let bob = Rc::new(RefCell::new(Thing::new("Bob")));

        let relationship = Rc::new(RefCell::new(Connection::new_undirected(
            [alice.clone(), bob.clone()],
            "friends",
        )));

        let mut alice_node = alice.try_borrow_mut().unwrap();
        let mut bob_node = bob.try_borrow_mut().unwrap();

        alice_node.add_connection(Rc::clone(&relationship));
        bob_node.add_connection(Rc::clone(&relationship));
    }

    #[test]
    fn graph_works() {
        let mut graph = ThingsAndConnections::<&str>::new();

        let alice = graph.new_thing("Alice");
        let bob = graph.new_thing("Bob");

        graph
            .new_undirected_connection([alice.clone(), bob.clone()], "Alice")
            .unwrap();
        graph
            .new_directed_connection(bob.clone(), alice.clone(), "admires")
            .unwrap();

        drop(alice);
        drop(bob);

        let alice = graph
            .find_thing(|node| node.get_data().eq(&"Alice"))
            .unwrap();
    }

    #[test]
    fn more_complicated_graph() {
        let mut graph = ThingsAndConnections::<&str>::new();
        // I have a brick. The brick is yellow. I also have a hat. The hat is black.

        let first_person = graph.new_thing("FirstPerson");
        let brick = graph.new_thing("Brick");
        let hat = graph.new_thing("Hat");
        let color = graph.new_thing("Color");
        let yellow = graph.new_thing("Yellow");
        let black = graph.new_thing("Black");

        graph
            .new_directed_connection(first_person.clone(), brick.clone(), "has")
            .unwrap();
        graph
            .new_directed_connection(brick.clone(), yellow.clone(), "is")
            .unwrap();
        graph
            .new_directed_connection(yellow.clone(), color.clone(), "is")
            .unwrap();
        graph
            .new_directed_connection(black.clone(), color.clone(), "is")
            .unwrap();
        graph
            .new_directed_connection(hat.clone(), black.clone(), "is")
            .unwrap();
        graph
            .new_directed_connection(first_person.clone(), hat.clone(), "has")
            .unwrap();
    }
}
