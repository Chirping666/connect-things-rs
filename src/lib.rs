#![no_std]
//! # Connect Things
//!
//! A `no_std` + `alloc` compatible crate for creating and managing graphs of interconnected entities.
//! This library provides flexible primitives for building knowledge representation systems,
//! GUI component hierarchies, social networks, or any domain where entities have relationships.
//!
//! ## Core Concepts
//!
//! - **Things**: Nodes in your graph that hold data and maintain lists of their connections
//! - **Connections**: Edges that can be directed or undirected, also carrying their own data
//! - **Soft Deletion**: Items are marked as "dead" but remain in memory until explicitly cleaned up
//! - **Memory Pressure Tracking**: Built-in monitoring of how much memory is consumed by dead items
//!
//! ## Example: Building a Knowledge Graph
//!
//! ```rust
//! use connect_things::*;
//!
//! enum Concept {
//!     Animal(String),
//!     Property(String),
//! }
//!
//! enum Relationship {
//!     IsA,
//!     HasProperty,
//! }
//!
//! let mut knowledge = Things::new();
//!
//! let dog = knowledge.new_thing(Concept::Animal("Dog".to_string()));
//! let mammal = knowledge.new_thing(Concept::Animal("Mammal".to_string()));
//! let warm_blooded = knowledge.new_thing(Concept::Property("Warm-blooded".to_string()));
//!
//! // Create relationships
//! knowledge.new_directed_connection(dog.clone(), mammal, Relationship::IsA);
//! knowledge.new_directed_connection(dog, warm_blooded, Relationship::HasProperty);
//! ```

extern crate alloc;
use alloc::rc::Rc;
use alloc::vec::Vec;

use core::cell::RefCell;

/// A node in the graph that holds data and maintains connections to other things.
///
/// Things use reference counting (`Rc`) and interior mutability (`RefCell`) to allow
/// shared ownership while maintaining the ability to modify connections and data.
/// This design enables flexible graph structures where multiple connections can
/// reference the same thing.
///
/// # Type Parameters
/// - `T`: The type of data stored in this thing
/// - `C`: The type of data stored in connections to this thing
///
/// # Examples
///
/// ```rust
/// use connect_things::Thing;
///
/// // Create a simple thing holding a string
/// let person = Thing::new("Alice");
///
/// // Access the data
/// let name = person.access_data(|data| data.clone());
/// assert_eq!(name, "Alice");
/// ```
pub struct Thing<T, C> {
    inner: Rc<RefCell<ThingInner<T, C>>>,
}

struct ThingInner<T, C> {
    connections: Vec<Connection<T, C>>,
    data: T,
    is_alive: bool,
}

impl<T, C> ThingInner<T, C> {
    pub fn new(data: T) -> Self {
        ThingInner {
            connections: Vec::new(),
            data,
            is_alive: true,
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
    /// Creates a new thing with the provided data.
    ///
    /// The thing starts alive and with no connections. Connections must be
    /// added through the `Things` container to ensure proper graph consistency.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use connect_things::Thing;
    ///
    /// let widget = Thing::new("Button");
    /// ```
    pub fn new(data: T) -> Self {
        Thing {
            inner: Rc::new(RefCell::new(ThingInner::new(data))),
        }
    }

    /// Adds a connection to this thing's list of connections.
    ///
    /// This is typically called internally by the `Things` container when
    /// creating connections. Manual use should be done carefully to maintain
    /// graph consistency.
    pub fn add_connection(&self, connection: Connection<T, C>) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.push(connection);
    }

    /// Finds the first connection that matches the given predicate.
    ///
    /// This is useful for navigation in your graph when you know the type
    /// of relationship you're looking for.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Find a "friendship" connection
    /// if let Some(friendship) = person.find_connection(|conn| {
    ///     conn.access_data(|data| *data == "friendship")
    /// }) {
    ///     let friend = friendship.get_directed_towards().unwrap();
    /// }
    /// ```
    pub fn find_connection(&self, find: fn(&Connection<T, C>) -> bool) -> Option<Connection<T, C>> {
        let inner = self.inner.try_borrow().unwrap();
        for conn in inner.connections.iter() {
            if find(conn) {
                return Some((*conn).clone());
            }
        }
        None
    }

    /// Finds all connections that match the given predicate.
    ///
    /// Useful when a thing can have multiple connections of the same type,
    /// such as a person having multiple friendships.
    ///
    /// # Returns
    /// A vector containing all matching connections. Empty if no matches found.
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

    /// Removes connections that match the given predicate from this thing's connection list.
    ///
    /// Note: This only removes the connection from this thing's local list.
    /// To properly remove connections from the entire graph, use the methods
    /// on the `Things` container instead.
    pub fn remove_connections(&mut self, remove: fn(&Connection<T, C>) -> bool) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.retain(|c| !remove(c))
    }

    /// Provides read-only access to this thing's data.
    ///
    /// The closure receives a reference to the data and can return any value.
    /// This pattern ensures memory safety while allowing flexible data access.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let name_length = person.access_data(|data| data.len());
    /// let is_alice = person.access_data(|data| data == "Alice");
    /// ```
    pub fn access_data<R>(&self, access: fn(&T) -> R) -> R {
        let inner = self.inner.try_borrow().unwrap();
        access(inner.get_data())
    }

    /// Provides mutable access to this thing's data.
    ///
    /// Similar to `access_data` but allows modification of the stored data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Update a person's name
    /// person.access_data_mut(|name| {
    ///     *name = "Bob".to_string();
    /// });
    /// ```
    pub fn access_data_mut<R>(&self, access: fn(&mut T) -> R) -> R {
        let mut inner = self.inner.borrow_mut();
        access(inner.get_data_mut())
    }

    /// Returns whether this thing is still alive (not marked for deletion).
    fn is_alive(&self) -> bool {
        let inner = self.inner.borrow();
        inner.is_alive
    }

    /// Marks this thing and all its connections as dead.
    ///
    /// When a thing is killed, it cascades to kill all connections attached to it.
    /// This represents the semantic that when an entity ceases to exist, all its
    /// relationships also cease to exist.
    ///
    /// # Returns
    /// The number of items killed (this thing plus any live connections that were killed).
    fn kill(&self) -> usize {
        let mut amnt = 0;
        let mut inner = self.inner.borrow_mut();
        // Only kill connections that are still alive to avoid double-counting
        for connection in inner.connections.iter() {
            if connection.is_alive() {
                connection.kill();
                amnt += 1;
            }
        }
        inner.is_alive = false;
        amnt + 1 // +1 for this thing itself
    }
}

impl<T, C> Clone for Thing<T, C> {
    /// Creates a new reference to the same thing.
    ///
    /// This is a shallow clone - both instances refer to the same underlying
    /// data and connection list. This enables the shared ownership model
    /// that makes flexible graph structures possible.
    fn clone(&self) -> Self {
        Thing {
            inner: self.inner.clone(),
        }
    }
}

/// A relationship between two things in the graph.
///
/// Connections can be either directed (representing asymmetric relationships like
/// "parent of" or "depends on") or undirected (representing symmetric relationships
/// like "friendship" or "similarity"). Each connection carries its own data to
/// describe the nature of the relationship.
///
/// # Type Parameters
/// - `T`: The type of data stored in connected things
/// - `C`: The type of data stored in this connection
///
/// # Examples
///
/// ```rust
/// use connect_things::{Thing, Connection};
///
/// let alice = Thing::new("Alice");
/// let bob = Thing::new("Bob");
///
/// // Create a directed connection (Alice likes Bob)
/// let likes = Connection::new_directed(alice, bob, "likes");
/// ```
pub struct Connection<T, C> {
    inner: Rc<RefCell<ConnectionInner<T, C>>>,
}

enum ConnectionInner<T, C> {
    Directed {
        from: Thing<T, C>,
        to: Thing<T, C>,
        data: C,
        is_alive: bool,
    },
    Undirected {
        things: [Thing<T, C>; 2],
        data: C,
        is_alive: bool,
    },
}

impl<T, C> ConnectionInner<T, C> {
    fn new_directed(from: Thing<T, C>, to: Thing<T, C>, data: C) -> Self {
        Self::Directed {
            from,
            to,
            data,
            is_alive: true,
        }
    }

    fn new_undirected(things: [Thing<T, C>; 2], data: C) -> Self {
        Self::Undirected {
            things,
            data,
            is_alive: true,
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

    fn is_alive(&self) -> bool {
        match self {
            &ConnectionInner::Directed { is_alive, .. } => is_alive,
            &ConnectionInner::Undirected { is_alive, .. } => is_alive,
        }
    }

    fn kill(&mut self) {
        match self {
            &mut ConnectionInner::Directed {
                ref mut is_alive, ..
            } => {
                *is_alive = false;
            }
            &mut ConnectionInner::Undirected {
                ref mut is_alive, ..
            } => {
                *is_alive = false;
            }
        }
    }
}

impl<T, C> Connection<T, C> {
    /// Creates a new directed connection from one thing to another.
    ///
    /// Directed connections represent asymmetric relationships. The order matters:
    /// the first thing is the "source" and the second is the "target" of the relationship.
    ///
    /// # Parameters
    /// - `from`: The source thing in the relationship
    /// - `to`: The target thing in the relationship
    /// - `data`: Data describing the nature of this relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// let parent_child = Connection::new_directed(parent, child, "parent_of");
    /// let dependency = Connection::new_directed(task_a, task_b, "depends_on");
    /// ```
    pub fn new_directed(from: Thing<T, C>, to: Thing<T, C>, data: C) -> Connection<T, C> {
        Connection {
            inner: Rc::new(RefCell::new(ConnectionInner::new_directed(from, to, data))),
        }
    }

    /// Creates a new undirected connection between two things.
    ///
    /// Undirected connections represent symmetric relationships where the order
    /// of things doesn't matter. The relationship applies equally in both directions.
    ///
    /// # Parameters
    /// - `things`: Array of exactly two things to connect
    /// - `data`: Data describing the nature of this relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// let friendship = Connection::new_undirected([alice, bob], "friendship");
    /// let similarity = Connection::new_undirected([item_a, item_b], "similar_to");
    /// ```
    pub fn new_undirected(things: [Thing<T, C>; 2], data: C) -> Connection<T, C> {
        Connection {
            inner: Rc::new(RefCell::new(ConnectionInner::new_undirected(things, data))),
        }
    }

    /// Returns true if this is a directed connection.
    ///
    /// Use this to determine the type of relationship before accessing
    /// directional properties.
    pub fn is_directed(&self) -> bool {
        let inner = self.inner.borrow();
        matches!(*inner, ConnectionInner::Directed { .. })
    }

    /// Returns true if this is an undirected connection.
    ///
    /// Undirected connections represent symmetric relationships.
    pub fn is_undirected(&self) -> bool {
        let inner = self.inner.borrow();
        matches!(*inner, ConnectionInner::Undirected { .. })
    }

    /// Provides read-only access to this connection's data.
    ///
    /// The closure receives a reference to the connection data and can return any value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let relationship_type = connection.access_data(|data| data.clone());
    /// let is_friendship = connection.access_data(|data| *data == "friendship");
    pub fn access_data<R>(&self, access: fn(&C) -> R) -> R {
        let inner = self.inner.borrow();
        access(inner.get_data())
    }

    /// Provides mutable access to this connection's data.
    ///
    /// Allows modification of the relationship data while maintaining safety.
    pub fn access_data_mut<R>(&self, access: fn(&mut C) -> R) -> R {
        let mut inner = self.inner.borrow_mut();
        access(inner.get_data_mut())
    }

    /// Returns the two things connected by this connection.
    ///
    /// For directed connections, returns [from, to]. For undirected connections,
    /// returns the two connected things in the order they were specified during creation.
    ///
    /// # Returns
    /// An array containing exactly two things.
    pub fn get_connected_things(&self) -> [Thing<T, C>; 2] {
        let inner = self.inner.borrow();
        inner.get_things().clone()
    }

    /// Returns the source thing in a directed connection.
    ///
    /// For directed connections, this is the "from" thing wrapped in a `Some(_)`. For undirected connections,
    /// this returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let parent = parent_child_relationship.get_directed_from();
    /// ```
    pub fn get_directed_from(&self) -> Option<Thing<T, C>> {
        let inner = self.inner.borrow();
        if self.is_directed() {
            Some(inner.get_things()[0].clone())
        } else {
            None
        }
    }

    /// Returns the target thing in a directed connection.
    ///
    /// For directed connections, this is the "to" thing wrapped in a `Some(_)`. For undirected connections,
    /// this returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let child = parent_child_relationship.get_directed_towards();
    /// ```
    pub fn get_directed_towards(&self) -> Option<Thing<T, C>> {
        let inner = self.inner.borrow();
        if self.is_directed() {
            Some(inner.get_things()[1].clone())
        } else {
            None
        }
    }

    /// Returns whether this connection is still alive (not marked for deletion).
    fn is_alive(&self) -> bool {
        let inner = self.inner.borrow();
        inner.is_alive()
    }

    /// Marks this connection as dead.
    ///
    /// Unlike thing.kill(), connection.kill() only affects the connection itself,
    /// not the things it connects. This represents the semantic that a relationship
    /// can end without the entities ceasing to exist.
    fn kill(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.kill();
    }
}

impl<T, C> Clone for Connection<T, C> {
    /// Creates a new reference to the same connection.
    ///
    /// This is a shallow clone - both instances refer to the same underlying
    /// connection data and connected things.
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
        }
    }
}

/// A container that manages a collection of things and their connections.
///
/// This is the primary interface for building and manipulating graphs. It provides
/// factory methods for creating things and connections while maintaining graph
/// consistency, and includes memory management features like cleanup and dead
/// item tracking.
///
/// # Type Parameters
/// - `T`: The type of data stored in things
/// - `C`: The type of data stored in connections
///
/// # Memory Management
///
/// The container uses a "soft deletion" approach where killed items remain in memory
/// but are marked as dead. This provides better performance during active graph
/// manipulation while allowing users to control when expensive cleanup operations occur.
///
/// # Examples
///
/// ```rust
/// use connect_things::Things;
///
/// let mut social_network = Things::new();
///
/// let alice = social_network.new_thing("Alice");
/// let bob = social_network.new_thing("Bob");
///
/// social_network.new_undirected_connection([alice, bob], "friendship");
/// ```
pub struct Things<T, C> {
    things: Vec<Thing<T, C>>,
    connections: Vec<Connection<T, C>>,
    dead_amnt: usize,
}

impl<T, C> Things<T, C> {
    /// Creates a new, empty graph container.
    ///
    /// The container starts with no things, no connections, and zero dead items.
    pub fn new() -> Things<T, C> {
        Things {
            things: Vec::new(),
            connections: Vec::new(),
            dead_amnt: 0,
        }
    }

    /// Creates a new thing with the provided data and adds it to the graph.
    ///
    /// The thing is automatically registered with the container and can be
    /// used immediately in connections.
    ///
    /// # Returns
    /// A `Thing` that can be used to create connections or access data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let person = graph.new_thing("Alice");
    /// let document = graph.new_thing(DocumentData { title: "Report", pages: 10 });
    pub fn new_thing(&mut self, data: T) -> Thing<T, C> {
        let thing = Thing::<T, C>::new(data);
        self.things.push(thing.clone());
        thing
    }

    /// Creates a directed connection between two things.
    ///
    /// The connection is automatically added to both things' connection lists
    /// and registered with the container. This ensures graph consistency.
    ///
    /// # Parameters
    /// - `from`: The source thing in the relationship
    /// - `to`: The target thing in the relationship
    /// - `data`: Data describing the relationship
    ///
    /// # Returns
    /// A `Connection` that can be used for navigation or data access.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let follows = graph.new_directed_connection(alice, bob, "follows");
    /// let manages = graph.new_directed_connection(manager, employee, "manages");
    /// ```
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

    /// Creates an undirected connection between two things.
    ///
    /// Like directed connections, this is automatically registered with both
    /// things and the container to maintain consistency.
    ///
    /// # Parameters
    /// - `things`: Array of exactly two things to connect
    /// - `data`: Data describing the symmetric relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// let friendship = graph.new_undirected_connection([alice, bob], "friendship");
    /// let similarity = graph.new_undirected_connection([doc1, doc2], "similar");
    /// ```
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

    /// Finds the first thing that matches the given predicate.
    ///
    /// This is useful for locating specific entities in your graph when you
    /// know something about their data but don't have a direct reference.
    ///
    /// # Returns
    /// `Some(thing)` if a match is found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let alice = graph.find_thing(|thing| {
    ///     thing.access_data(|data| data.name == "Alice")
    /// });
    /// ```
    pub fn find_thing(&self, search: fn(&Thing<T, C>) -> bool) -> Option<Thing<T, C>> {
        for thing in &self.things {
            if search(thing) {
                return Some(thing.clone());
            }
        }
        None
    }

    /// Finds all things that match the given predicate.
    ///
    /// Useful for finding groups of related entities or filtering the graph
    /// based on data properties.
    ///
    /// # Returns
    /// A vector containing all matching things. Empty if no matches found.
    pub fn find_things(&self, find: fn(&Thing<T, C>) -> bool) -> Vec<Thing<T, C>> {
        let mut things = Vec::new();
        for thing in &self.things {
            if find(thing) {
                things.push(thing.clone());
            }
        }
        things
    }

    /// Marks things matching the predicate as dead.
    ///
    /// When a thing is killed, all its connections are also marked as dead.
    /// Dead items remain in memory until `clean()` is called, allowing for
    /// better performance during active graph manipulation.
    ///
    /// The dead count is automatically updated to track memory pressure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Remove all temporary items
    /// graph.kill_things(|thing| {
    ///     thing.access_data(|data| data.is_temporary)
    /// });
    /// ```
    pub fn kill_things(&mut self, kill: fn(&Thing<T, C>) -> bool) {
        self.things.iter().for_each(|thing| {
            if kill(thing) {
                let amnt = thing.kill();
                let _ = self.dead_amnt.saturating_add(amnt);
            }
        });
    }

    /// Finds the first connection that matches the given predicate.
    ///
    /// Useful for locating specific relationships in your graph.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let friendship = graph.find_connection(|conn| {
    ///     conn.access_data(|data| *data == "friendship")
    /// });
    /// ```
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

    /// Finds all connections that match the given predicate.
    ///
    /// Useful for analyzing relationship patterns or finding all connections
    /// of a particular type.
    pub fn find_connections(&self, search: fn(&Connection<T, C>) -> bool) -> Vec<Connection<T, C>> {
        let mut connections = Vec::new();
        for connection in &self.connections {
            if search(connection) {
                connections.push(connection.clone());
            }
        }
        connections
    }

    /// Marks connections matching the predicate as dead.
    ///
    /// Unlike `kill_things`, this only affects the connections themselves,
    /// not the things they connect. The connected things remain alive.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Remove all temporary relationships
    /// graph.kill_connections(|conn| {
    ///     conn.access_data(|data| data.is_temporary)
    /// });
    /// ```
    pub fn kill_connections(&mut self, kill: fn(&Connection<T, C>) -> bool) {
        self.connections.iter().for_each(|connection| {
            if kill(connection) {
                connection.kill();
                let _ = self.dead_amnt.saturating_add(1);
            }
        });
    }

    /// Calculates the percentage of dead items relative to total items.
    ///
    /// This provides a "memory pressure" metric to help decide when cleanup
    /// might be beneficial. The percentage represents how much of your graph's
    /// memory is consumed by logically deleted items.
    ///
    /// # Returns
    /// - `Ok(percentage)`: The percentage (0-100) of dead items
    /// - `Err(())`: If the graph is empty (division by zero)
    ///
    /// # Memory Pressure Guidelines
    /// - 0-10%: Minimal waste, cleanup probably unnecessary
    /// - 10-25%: Moderate waste, consider cleanup during idle periods
    /// - 25-50%: Significant waste, cleanup recommended
    /// - 50%+: High waste, cleanup should be prioritized
    ///
    /// # Examples
    ///
    /// ```rust
    /// match graph.dead_percentage() {
    ///     Ok(percent) if percent > 25 => {
    ///         println!("High memory pressure: {}%", percent);
    ///         graph.clean();
    ///     }
    ///     Ok(percent) => println!("Memory pressure: {}%", percent),
    ///     Err(_) => println!("Empty graph"),
    /// }
    /// ```
    pub fn dead_percentage(&mut self) -> Result<usize, ()> {
        let total = self
            .things
            .len()
            .checked_add(self.connections.len())
            .unwrap_or_else(|| 100);

        if total == 0 {
            self.dead_amnt = 0;
            return Err(());
        }

        let mulled = self.dead_amnt.checked_mul(100).unwrap_or_else(|| 100);

        let dived = mulled / total;

        Ok(dived)
    }

    /// Removes all dead things and connections from memory.
    ///
    /// This performs the actual cleanup of items that were previously marked
    /// as dead. After cleaning, only live items remain in the graph and the
    /// dead count is reset to zero.
    ///
    /// This operation can be expensive for large graphs, so it's typically
    /// called strategically based on memory pressure or at natural breakpoints
    /// in your application.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Clean up when memory pressure gets high
    /// if graph.dead_percentage().unwrap_or(0) > 30 {
    ///     graph.clean();
    ///     println!("Graph cleaned");
    /// }
    /// ```
    pub fn clean(&mut self) {
        self.things.retain(|thing| thing.is_alive());

        self.connections.retain(|connection| connection.is_alive());

        self.dead_amnt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::String;

    /// Creates a sample knowledge graph for testing.
    /// This represents a simple taxonomy with foods, categories, and preferences.
    fn test_knowledge_graph<'a>() -> Things<&'a str, &'a str> {
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

    
}
