#![no_std]
extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Deref;
use spin::rwlock::RwLock;

pub struct ThingsAndConnections<D> {
    things: Vec<Arc<RwLock<Thing<D>>>>,
    connections: Vec<Arc<RwLock<Connection<D>>>>,
}

impl<D> ThingsAndConnections<D> {
    pub fn new() -> ThingsAndConnections<D> {
        ThingsAndConnections {
            things: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn new_thing(&mut self, data: D) -> Arc<RwLock<Thing<D>>> {
        let node = Arc::new(RwLock::new(Thing::new(data)));
        self.things.push(node.clone());
        node
    }

    pub fn new_directed_connection(
        &mut self,
        from: Arc<RwLock<Thing<D>>>,
        to: Arc<RwLock<Thing<D>>>,
        data: D,
    ) -> Arc<RwLock<Connection<D>>> {
        let edge = Arc::new(RwLock::new(Connection::new_directed(
            from.clone(),
            to.clone(),
            data,
        )));
        let mut first = from.write();
        let mut second = to.write();
        first.add_connection(edge.clone());
        second.add_connection(edge.clone());
        self.connections.push(edge.clone());
        edge
    }

    pub fn try_new_directed_connection(
        &mut self,
        from: Arc<RwLock<Thing<D>>>,
        to: Arc<RwLock<Thing<D>>>,
        data: D,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        let edge = Arc::new(RwLock::new(Connection::new_directed(
            from.clone(),
            to.clone(),
            data,
        )));
        let first = from.try_write();
        let second = to.try_write();
        if let (Some(mut first), Some(mut second)) = (first, second) {
            first.add_connection(edge.clone());
            second.add_connection(edge.clone());
            self.connections.push(edge.clone());
            return Some(edge);
        }
        None
    }

    pub fn new_undirected_connection(
        &mut self,
        nodes: [Arc<RwLock<Thing<D>>>; 2],
        data: D,
    ) -> Arc<RwLock<Connection<D>>> {
        let edge = Arc::new(RwLock::new(Connection::new_undirected(
            [nodes[0].clone(), nodes[1].clone()],
            data,
        )));
        let mut first = nodes[0].write();
        let mut second = nodes[1].write();
        first.add_connection(edge.clone());
        second.add_connection(edge.clone());
        self.connections.push(edge.clone());
        edge
    }

    pub fn try_new_undirected_connection(
        &mut self,
        nodes: [Arc<RwLock<Thing<D>>>; 2],
        data: D,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        let edge = Arc::new(RwLock::new(Connection::new_undirected(
            [nodes[0].clone(), nodes[1].clone()],
            data,
        )));
        let first = nodes[0].try_write();
        let second = nodes[1].try_write();
        if let (Some(mut first), Some(mut second)) = (first, second) {
            first.add_connection(edge.clone());
            second.add_connection(edge.clone());
            self.connections.push(edge.clone());
            return Some(edge);
        }
        None
    }

    pub fn get_things(&self) -> &[Arc<RwLock<Thing<D>>>] {
        &self.things
    }

    pub fn get_connections(&self) -> &[Arc<RwLock<Connection<D>>>] {
        &self.connections
    }

    pub fn find_thing(&self, finder: fn(&Thing<D>) -> bool) -> Option<Arc<RwLock<Thing<D>>>> {
        for node in self.things.iter() {
            let node_guard = node.read();
            if finder(node_guard.deref()) {
                return Some(node.clone());
            }
        }
        None
    }

    pub fn try_find_thing(&self, finder: fn(&Thing<D>) -> bool) -> Option<Arc<RwLock<Thing<D>>>> {
        for node in self.things.iter() {
            if let Some(node_guard) = node.try_read() {
                if finder(node_guard.deref()) {
                    return Some(node.clone());
                }
            }
        }
        None
    }

    pub fn find_connection(
        &self,
        finder: fn(&Connection<D>) -> bool,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        for edge in self.connections.iter() {
            let edge_guard = edge.read();
            if finder(edge_guard.deref()) {
                return Some(edge.clone());
            }
        }
        None
    }

    pub fn try_find_connection(
        &self,
        finder: fn(&Connection<D>) -> bool,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        for edge in self.connections.iter() {
            if let Some(edge_guard) = edge.try_read() {
                if finder(edge_guard.deref()) {
                    return Some(edge.clone());
                }
            }
        }
        None
    }

    pub fn filter_things(&self, filter: fn(&Thing<D>) -> bool) -> Vec<Arc<RwLock<Thing<D>>>> {
        let mut nodes = Vec::new();
        for node in self.things.iter() {
            let node_guard = node.read();
            if filter(node_guard.deref()) {
                nodes.push(node.clone());
            }
        }
        nodes
    }

    pub fn try_filter_things(&self, filter: fn(&Thing<D>) -> bool) -> Vec<Arc<RwLock<Thing<D>>>> {
        let mut nodes = Vec::new();
        for node in self.things.iter() {
            if let Some(node_guard) = node.try_read() {
                if filter(node_guard.deref()) {
                    nodes.push(node.clone());
                }
            }
        }
        nodes
    }

    pub fn filter_connections(
        &self,
        filter: fn(&Connection<D>) -> bool,
    ) -> Vec<Arc<RwLock<Connection<D>>>> {
        let mut edges = Vec::new();
        for edge in self.connections.iter() {
            let edge_guard = edge.read();
            if filter(edge_guard.deref()) {
                edges.push(edge.clone());
            }
        }
        edges
    }

    pub fn try_filter_connections(
        &self,
        filter: fn(&Connection<D>) -> bool,
    ) -> Vec<Arc<RwLock<Connection<D>>>> {
        let mut edges = Vec::new();
        for edge in self.connections.iter() {
            if let Some(edge_guard) = edge.try_read() {
                if filter(edge_guard.deref()) {
                    edges.push(edge.clone());
                }
            }
        }
        edges
    }
}

pub struct Thing<D> {
    connections: Vec<Arc<RwLock<Connection<D>>>>,
    data: D,
}

impl<D> Thing<D> {
    pub fn new(data: D) -> Self {
        Thing {
            connections: Vec::new(),
            data,
        }
    }

    pub fn add_connection(&mut self, edge: Arc<RwLock<Connection<D>>>) {
        self.connections.push(edge);
    }

    pub fn get_connections(&self) -> &[Arc<RwLock<Connection<D>>>] {
        &self.connections
    }

    pub fn find_connection(
        &self,
        finder: fn(&Connection<D>) -> bool,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        for edge in self.connections.iter() {
            let edge_lock_read = edge.read();
            if finder(edge_lock_read.deref()) {
                return Some(edge.clone());
            }
        }
        None
    }

    pub fn try_find_connection(
        &self,
        finder: fn(&Connection<D>) -> bool,
    ) -> Option<Arc<RwLock<Connection<D>>>> {
        for edge in self.connections.iter() {
            if let Some(edge_lock_read) = edge.try_read() {
                if finder(edge_lock_read.deref()) {
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
        from: Arc<RwLock<Thing<D>>>,
        to: Arc<RwLock<Thing<D>>>,
        data: D,
    },
    Undirected {
        nodes: [Arc<RwLock<Thing<D>>>; 2],
        data: D,
    },
}

impl<D> Connection<D> {
    pub fn new_directed(from: Arc<RwLock<Thing<D>>>, to: Arc<RwLock<Thing<D>>>, data: D) -> Self {
        Connection::Directed { from, to, data }
    }

    pub fn new_undirected(nodes: [Arc<RwLock<Thing<D>>>; 2], data: D) -> Self {
        Connection::Undirected { nodes, data }
    }

    pub fn get_things(&self) -> (Arc<RwLock<Thing<D>>>, Arc<RwLock<Thing<D>>>) {
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
        let alice = Arc::new(RwLock::new(Thing::new("Alice")));
        let bob = Arc::new(RwLock::new(Thing::new("Bob")));

        let relationship = Arc::new(RwLock::new(Connection::new_undirected(
            [alice.clone(), bob.clone()],
            "friends",
        )));

        let mut alice_node = alice.write();
        let mut bob_node = bob.write();

        alice_node.add_connection(Arc::clone(&relationship));
        bob_node.add_connection(Arc::clone(&relationship));
    }

    #[test]
    fn graph_works() {
        let mut graph = ThingsAndConnections::<&str>::new();

        let alice = graph.new_thing("Alice");
        let bob = graph.new_thing("Bob");

        graph.new_undirected_connection([alice.clone(), bob.clone()], "Alice");
        graph.new_directed_connection(bob.clone(), alice.clone(), "admires");

        drop(alice);
        drop(bob);

        let _alice = graph.find_thing(|node| node.get_data().eq(&"Alice"));
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

        graph.new_directed_connection(first_person.clone(), brick.clone(), "has");
        graph.new_directed_connection(brick.clone(), yellow.clone(), "is");
        graph.new_directed_connection(yellow.clone(), color.clone(), "is");
        graph.new_directed_connection(black.clone(), color.clone(), "is");
        graph.new_directed_connection(hat.clone(), black.clone(), "is");
        graph.new_directed_connection(first_person.clone(), hat.clone(), "has");

        drop(first_person);
        drop(brick);
        drop(hat);
        drop(color);
        drop(yellow);
        drop(black);

        let brick = graph
            .find_thing(|thing| thing.get_data().eq(&"Brick"))
            .unwrap();
    }
}
