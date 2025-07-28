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
//! ## Example: Building a Complete Knowledge Graph
//!
//! ```rust
//! use connect_things::*;
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Concept {
//!     Person(&'static str),
//!     Food(&'static str),
//!     Category(&'static str),
//! }
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Relationship {
//!     Likes,
//!     IsA,
//!     Contains,
//! }
//!
//! fn main() {
//!     let mut knowledge = Things::new();
//!
//!     // Create entities in our knowledge base
//!     let alice = knowledge.new_thing(Concept::Person("Alice"));
//!     let apples = knowledge.new_thing(Concept::Food("Apples"));
//!     let fruit = knowledge.new_thing(Concept::Category("Fruit"));
//!
//!     // Build relationships between concepts
//!     knowledge.new_directed_connection(alice.clone(), Relationship::Likes, apples.clone());
//!     knowledge.new_directed_connection(apples.clone(), Relationship::IsA, fruit.clone());
//!
//!     // Query the knowledge: What category of food does Alice like?
//!     let alice_preferences = alice.do_for_all_connections(|conn| {
//!         if conn.points_away_from(&alice) && conn == &Relationship::Likes {
//!             Do::Take(conn)
//!         } else {
//!             Do::Nothing
//!         }
//!     });
//!
//!     for preference in alice_preferences {
//!         if let Some(food) = preference.get_directed_towards() {
//!             let food_categories = food.do_for_a_connection(|conn| {
//!                 if conn == &Relationship::IsA {
//!                     Do::Take(conn.clone())
//!                 } else {
//!                     Do::Nothing
//!                 }
//!             });
//!
//!             for category_rel in food_categories {
//!                 if let Some(category) = category_rel.get_directed_towards() {
//!                     println!("Alice likes food in category: {:?}",
//!                         category.access(|data| data));
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```
extern crate alloc;
use alloc::rc::Rc;
use alloc::vec::Vec;

use core::cell::RefCell;

/// A signal to return a value or continue iterating.
/// Mainly to keep semantics clean.
pub enum Do<R> {
    Take(R),
    Nothing,
}

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
/// ## Basic Usage
/// ```rust
/// use connect_things::Thing;
///
/// // Create a simple thing holding a string
/// let person = Thing::new("Alice");
///
/// // Access the data safely
/// let name = person.access(|data| data.clone());
/// assert_eq!(name, "Alice");
/// ```
///
/// ## Complete Navigation Example
/// ```rust
/// use connect_things::*;
///
/// let mut graph = Things::new();
///
/// let person = graph.new_thing("Alice");
/// let hobby = graph.new_thing("Photography");
///
/// let enjoys = graph.new_directed_connection(person.clone(), "enjoys", hobby);
///
/// // Navigate from person to their hobby
/// let alice_hobbies = person.do_for_all_connections(|conn| {
///     conn.access(|data| return if *data == "enjoys" { Do::Take(conn) } else { Do::Nothing })
/// });
///
/// for hobby_connection in alice_hobbies {
///     if let Some(hobby_thing) = hobby_connection.get_directed_towards() {
///         let hobby_name = hobby_thing.access(|data| *data);
///         println!("Alice enjoys: {}", hobby_name);
///     }
/// }
/// ```
pub struct Thing<T: PartialEq, C: PartialEq> {
    inner: Rc<RefCell<ThingInner<T, C>>>,
}

struct ThingInner<T: PartialEq, C: PartialEq> {
    connections: Vec<Connection<T, C>>,
    data: T,
    is_alive: bool,
}

impl<T: PartialEq, C: PartialEq> ThingInner<T, C> {
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

impl<T: PartialEq, C: PartialEq> Thing<T, C> {
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
    pub unsafe fn connect(&self, connection: Connection<T, C>) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.push(connection);
    }

    /// Checks if a connection is present for a thing.
    pub fn is_connected_through(&self, other: &Connection<T, C>) -> bool {
        let inner = self.inner.borrow();
        for conn in &inner.connections {
            if conn == other {
                return true;
            }
        }
        false
    }

    /// Finds the first connection that matches the given predicate.
    ///
    /// This is useful for navigation in your graph when you know the type
    /// of relationship you're looking for. Remember to handle the Option
    /// return from directional methods when working with the result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use connect_things::*;
    /// # let person = Thing::new("Person");
    /// # let other = Thing::new("Other");
    /// # let connection = Connection::new_undirected([person.clone(),other],"friendship");
    ///
    /// // Find a "friendship" connection and navigate to the friend
    /// if let Some(friendship) = person.do_for_a_connection(|conn| {
    ///     conn.access(|data| return if *data == "friendship" { Do::Take(conn.clone()) } else { Do::Nothing })
    /// }) {
    ///     // For directed connections, get the target safely
    ///     if let Some(friend) = friendship.get_directed_towards() {
    ///         println!("Found a friend!");
    ///     }
    ///     // For undirected connections, get both connected things
    ///     let connected_people = friendship.get_things();
    /// }
    /// ```
    pub fn do_for_a_connection<R: Clone>(
        &self,
        do_for: impl Fn(&Connection<T, C>) -> Do<R>,
    ) -> Option<R> {
        let inner = self.inner.try_borrow().unwrap();
        for conn in inner.connections.iter() {
            if let Do::Take(value) = do_for(conn) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Finds all connections that match the given predicate.
    ///
    /// Useful when a thing can have multiple connections of the same type,
    /// such as a person having multiple friendships or a task having multiple dependencies.
    ///
    /// # Returns
    /// A vector containing all matching connections. Empty if no matches found.
    pub fn do_for_all_connections<R>(&self, do_for: impl Fn(&Connection<T, C>) -> Do<R>) -> Vec<R> {
        let mut connections = Vec::new();
        let inner = self.inner.borrow();
        for conn in inner.connections.iter() {
            if let Do::Take(value) = do_for(conn) {
                connections.push(value)
            }
        }
        connections
    }

    /// Removes connections that match the given predicate from this thing's connection list.
    ///
    /// Note: This only removes the connection from this thing's local list.
    /// To properly remove connections from the entire graph, use the methods
    /// on the `Things` container instead.
    pub unsafe fn remove_connections(&mut self, remove: impl Fn(&Connection<T, C>) -> bool) {
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
    /// # use connect_things::Thing;
    /// # let person = Thing::new("Alice");
    ///
    /// let name_length = person.access(|data| data.len());
    /// let is_alice = person.access(|data| *data == "Alice");
    /// ```
    pub fn access<R>(&self, access: impl Fn(&T) -> R) -> R {
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
    /// # use connect_things::Thing;
    /// # let person = Thing::new("Alice");
    ///
    /// // Update a person's name
    /// person.access_mut(|name| {
    ///     *name = "Bob";
    /// });
    /// ```
    pub fn access_mut<R>(&self, access: impl Fn(&mut T) -> R) -> R {
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
        let mut amount = 0;
        let mut inner = self.inner.borrow_mut();
        // Only kill connections that are still alive to avoid double-counting
        for connection in inner.connections.iter() {
            if connection.is_alive() {
                connection.kill();
                amount += 1;
            }
        }
        inner.is_alive = false;
        amount + 1 // +1 for this thing itself
    }

    /// Removes dead connections.
    fn clean(&mut self) {
        let mut inner = self.inner.borrow_mut();
        inner.connections.retain(|c| c.is_alive());
    }
}

impl<T: PartialEq, C: PartialEq> Clone for Thing<T, C> {
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

impl<T: PartialEq, C: PartialEq> PartialEq for Thing<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.access(|data| other.access(|other_data| data == other_data))
    }
}

impl<T: PartialEq, C: PartialEq> PartialEq<T> for Thing<T, C> {
    fn eq(&self, other: &T) -> bool {
        self.access(|data| data == other)
    }

    fn ne(&self, other: &T) -> bool {
        self.access(|data| data != other)
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
/// ## Basic Connection Creation
/// ```rust
/// use connect_things::{Thing, Connection};
///
/// let alice = Thing::new("Alice");
/// let bob = Thing::new("Bob");
///
/// // Create a directed connection (Alice likes Bob)
/// let likes = Connection::new_directed(alice, "likes", bob);
/// ```
///
/// ## Modeling Different Relationship Types
/// ```rust
/// use connect_things::Things;
///
/// let mut social_graph = Things::new();
///
/// let alice = social_graph.new_thing("Alice");
/// let bob = social_graph.new_thing("Bob");
///
/// // Symmetric relationship: friendship is mutual
/// let friendship = social_graph.new_undirected_connection(
///     [alice.clone(), bob.clone()],
///     "friendship"
/// );
///
/// // Asymmetric relationship: following can be one-way
/// let following = social_graph.new_directed_connection(
///     alice.clone(),
///     "follows",
///     bob.clone()
/// );
///
/// // Friendship works both ways
/// assert!(friendship.is_undirected());
/// let friends = friendship.get_things();
/// // Either person can find this friendship in their connections
///
/// // Following has direction
/// assert!(following.is_directed());
/// if let Some(follower) = following.get_directed_from() {
///     // Alice is the follower
/// }
/// if let Some(followed) = following.get_directed_towards() {
///     // Bob is being followed
/// }
/// ```
pub struct Connection<T: PartialEq, C: PartialEq> {
    inner: Rc<RefCell<ConnectionInner<T, C>>>,
}

enum ConnectionInner<T: PartialEq, C: PartialEq> {
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

impl<T: PartialEq, C: PartialEq> ConnectionInner<T, C> {
    fn new_directed(from: Thing<T, C>, data: C, to: Thing<T, C>) -> Self {
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

    fn contains(&self, thing: &Thing<T, C>) -> bool {
        match &self {
            &ConnectionInner::Directed { from, to, .. } => {
                if (from == thing) || (to == thing) {
                    true
                } else {
                    false
                }
            }
            &ConnectionInner::Undirected { things, .. } => {
                if (&things[0] == thing) || (&things[1] == thing) {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn get_direction_relative_to(&self, thing: &Thing<T, C>) -> Result<Direction, ()> {
        match &self {
            &ConnectionInner::Directed { from, to, .. } => {
                if thing == from {
                    Ok(Direction::AwayFrom)
                } else if thing == to {
                    Ok(Direction::Towards)
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }

    fn points_away_from(&self, thing: &Thing<T, C>) -> bool {
        if let Ok(Direction::AwayFrom) = self.get_direction_relative_to(thing) {
            true
        } else {
            false
        }
    }

    fn points_towards(&self, thing: &Thing<T, C>) -> bool {
        if let Ok(Direction::Towards) = self.get_direction_relative_to(thing) {
            true
        } else {
            false
        }
    }

    fn get_other_thing(&self, thing: &Thing<T, C>) -> Result<Thing<T, C>, ()> {
        match &self {
            &ConnectionInner::Directed { from, to, .. } => {
                if thing == from {
                    Ok(to.clone())
                } else if thing == to {
                    Ok(from.clone())
                } else {
                    Err(())
                }
            }
            &ConnectionInner::Undirected { things, .. } => {
                if thing == &things[0] {
                    Ok(things[1].clone())
                } else if thing == &things[1] {
                    Ok(things[0].clone())
                } else {
                    Err(())
                }
            }
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

/// Used to check whether a connection is directed towards or away from a thing.
#[derive(PartialEq, Debug)]
pub enum Direction {
    Towards,
    AwayFrom,
}

impl<T: PartialEq, C: PartialEq> Connection<T, C> {
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
    /// # use connect_things::*;
    /// # let parent = Thing::new(());
    /// # let child = Thing::new(());
    /// # let task_a = Thing::new(());
    /// # let task_b = Thing::new(());
    ///
    /// let parent_child = Connection::new_directed(parent, "parent_of", child);
    /// let dependency = Connection::new_directed(task_a, "depends_on", task_b);
    /// ```
    pub fn new_directed(from: Thing<T, C>, data: C, to: Thing<T, C>) -> Connection<T, C> {
        Connection {
            inner: Rc::new(RefCell::new(ConnectionInner::new_directed(from, data, to))),
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
    /// # use connect_things::*;
    /// # let alice = Thing::new(());
    /// # let bob = Thing::new(());
    /// # let item_a = Thing::new(());
    /// # let item_b = Thing::new(());
    ///
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
    /// # use connect_things::*;
    /// # let connection = Connection::new_undirected([Thing::new(()),Thing::new(())],"friendship");
    ///
    /// let relationship_type = connection.access(|data| data.clone());
    /// let is_friendship = connection.access(|data| *data == "friendship");
    pub fn access<R>(&self, access: impl Fn(&C) -> R) -> R {
        let inner = self.inner.borrow();
        access(inner.get_data())
    }

    /// Provides mutable access to this connection's data.
    ///
    /// Allows modification of the relationship data while maintaining safety.
    pub fn access_mut<R>(&self, access: impl Fn(&mut C) -> R) -> R {
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
    pub fn get_things(&self) -> [Thing<T, C>; 2] {
        let inner = self.inner.borrow();
        inner.get_things().clone()
    }

    /// Returns the source thing in a directed connection.
    ///
    /// For directed connections, this returns the "from" thing wrapped in `Some`.
    /// For undirected connections, this returns `None` since there is no meaningful
    /// direction to the relationship.
    ///
    /// # Returns
    /// - `Some(thing)`: The source thing for directed connections
    /// - `None`: For undirected connections
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use connect_things::*;
    /// # let parent_child_relationship = Connection::new_directed(Thing::new(()),(),Thing::new(()));
    ///
    /// if let Some(parent) = parent_child_relationship.get_directed_from() {
    ///     println!("Found the parent");
    /// }
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
    /// For directed connections, this returns the "to" thing wrapped in `Some`.
    /// For undirected connections, this returns `None` since there is no meaningful
    /// direction to the relationship.
    ///
    /// # Returns
    /// - `Some(thing)`: The target thing for directed connections
    /// - `None`: For undirected connections
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use connect_things::*;
    /// # let parent_child_relationship = Connection::new_directed(Thing::new(()),(),Thing::new(()));
    ///
    /// if let Some(child) = parent_child_relationship.get_directed_towards() {
    ///     println!("Found the child");
    /// }
    /// ```
    pub fn get_directed_towards(&self) -> Option<Thing<T, C>> {
        let inner = self.inner.borrow();
        if self.is_directed() {
            Some(inner.get_things()[1].clone())
        } else {
            None
        }
    }

    /// Tells you whether a thing is part of a connection.
    ///
    /// # Example
    ///
    /// ```rust
    ///  use connect_things::*;
    ///
    /// let a = Thing::new("a");
    /// let b = Thing::new("b");
    /// let ab = Connection::new_undirected([a.clone(),b.clone()],"ab");
    ///
    /// assert!(ab.contains(&a));
    /// assert!(ab.contains(&b));
    ///
    /// ```
    pub fn contains(&self, thing: &Thing<T, C>) -> bool {
        let inner = self.inner.borrow();
        inner.contains(thing)
    }

    /// Reveals whether a thing is the target or source of the directed connection.
    ///
    /// # Returns
    /// - `Ok(Direction)`: The direction if the connection is directed and the thing is part of the connection.
    /// - `Err(())`: If the above conditions were not satisfied.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use connect_things::*;
    /// # let apples = Thing::new("Apples");
    /// # let oranges = Thing::new("Oranges");
    ///
    /// let comparison = Connection::new_directed(apples.clone(),"compare_to",oranges.clone());
    ///
    /// assert_eq!(Direction::AwayFrom, comparison.get_direction_relative_to(&apples).unwrap());
    /// ```
    pub fn get_direction_relative_to(&self, thing: &Thing<T, C>) -> Result<Direction, ()> {
        let inner = self.inner.borrow();
        inner.get_direction_relative_to(thing)
    }

    /// Quickly check if a connection points away from a thing.
    ///
    /// # Returns
    /// - `true`: If `connection.get_direction_relative_to(&thing)` returns `Ok(Direction::AwayFrom)`.
    /// - `false`: Otherwise
    pub fn points_away_from(&self, thing: &Thing<T,C>) -> bool {
        let inner = self.inner.borrow();
        inner.points_away_from(thing)
    }

    /// Quickly check if a connection points towards a thing.
    ///
    /// # Returns
    /// - `true`: If `connection.get_direction_relative_to(&thing)` returns `Ok(Direction::Towards)`.
    /// - `false`: Otherwise
    pub fn points_towards(&self, thing: &Thing<T,C>) -> bool {
        let inner = self.inner.borrow();
        inner.points_towards(thing)
    }

    /// Finds the thing at the other end of a connection.
    ///
    /// # Returns
    /// - `Ok(Thing<T,C>)`: The other thing if the argument is part of the connection.
    /// - `Err(())`: Otherwise.
    ///
    /// # Example
    /// ```rust
    /// # use connect_things::*;
    ///
    /// let thing1 = Thing::new(());
    /// let thing2 = Thing::new(());
    ///
    /// let connection = Connection::new_undirected([thing1.clone(),thing2.clone()],());
    ///
    /// assert!(thing2 == connection.get_other_thing(&thing1).unwrap());
    /// ```
    pub fn get_other_thing(&self, thing: &Thing<T, C>) -> Result<Thing<T, C>, ()> {
        let inner = self.inner.borrow();
        inner.get_other_thing(thing)
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

impl<T: PartialEq, C: PartialEq> Clone for Connection<T, C> {
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

impl<T: PartialEq, C: PartialEq> PartialEq for Connection<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.access(|data| other.access(|other_data| data == other_data))
    }
}

impl<T: PartialEq, C: PartialEq> PartialEq<C> for Connection<T, C> {
    fn eq(&self, other: &C) -> bool {
        self.access(|data| data == other)
    }

    fn ne(&self, other: &C) -> bool {
        self.access(|data| data != other)
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
/// ## Basic Graph Creation
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
///
/// ## Complete Memory Management Workflow
/// ```rust
/// use connect_things::Things;
///
/// let mut graph = Things::new();
///
/// // Build a temporary subgraph for analysis
/// let temp_data = graph.new_thing("temporary_analysis");
/// let result = graph.new_thing("analysis_result");
/// graph.new_directed_connection(temp_data.clone(), "produces", result.clone());
///
/// // Check memory pressure before cleanup
/// match graph.dead_percentage() {
///     Ok(pressure) if pressure > 20 => {
///         println!("Memory pressure high: {}%", pressure);
///         graph.clean();
///     }
///     Ok(pressure) => println!("Memory pressure acceptable: {}%", pressure),
///     Err(_) => println!("Empty graph - no cleanup needed"),
/// }
///
/// // Remove temporary analysis data when done
/// graph.kill_things(|thing| {
///     thing.access(|data| data.starts_with("temporary_"))
/// });
///
/// // Keep final results, clean up intermediate data
/// graph.clean();
/// ```
pub struct Things<T: PartialEq, C: PartialEq> {
    things: Vec<Thing<T, C>>,
    connections: Vec<Connection<T, C>>,
    dead_amount: usize,
}

impl<T: PartialEq, C: PartialEq> Things<T, C> {
    /// Creates a new, empty graph container.
    ///
    /// The container starts with no things, no connections, and zero dead items.
    pub fn new() -> Things<T, C> {
        Things {
            things: Vec::new(),
            connections: Vec::new(),
            dead_amount: 0,
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
    /// # struct DocumentData {
    /// #     title: &'static str,
    /// #     pages: usize
    /// # }
    /// # use connect_things::*;
    /// # let mut graph1 = Things::new();
    /// # let mut graph2 = Things::new();
    ///
    /// let person = graph1.new_thing("Alice");
    /// let document = graph2.new_thing(DocumentData { title: "Report", pages: 10 });
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
    /// # use connect_things::*;
    /// # let alice = Thing::new(());
    /// # let bob = Thing::new(());
    /// # let manager = Thing::new(());
    /// # let employee = Thing::new(());
    /// # let mut graph = Things::new();
    ///
    /// let follows = graph.new_directed_connection(alice, "follows", bob);
    /// let manages = graph.new_directed_connection(manager, "manages", employee);
    /// ```
    pub fn new_directed_connection(
        &mut self,
        from: Thing<T, C>,
        data: C,
        to: Thing<T, C>,
    ) -> Connection<T, C> {
        let connection = Connection::<T, C>::new_directed(from.clone(), data, to.clone());
        unsafe { from.connect(connection.clone()) };
        unsafe { to.connect(connection.clone()) };
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
    /// # use connect_things::*;
    /// # let alice = Thing::new(());
    /// # let bob = Thing::new(());
    /// # let doc1 = Thing::new(());
    /// # let doc2 = Thing::new(());
    /// # let mut graph = Things::new();
    /// let friendship = graph.new_undirected_connection([alice, bob], "friendship");
    /// let similarity = graph.new_undirected_connection([doc1, doc2], "similar");
    /// ```
    pub fn new_undirected_connection(
        &mut self,
        things: [Thing<T, C>; 2],
        data: C,
    ) -> Connection<T, C> {
        let connection = Connection::<T, C>::new_undirected(things.clone(), data);
        unsafe { things[0].connect(connection.clone()) };
        unsafe { things[1].connect(connection.clone()) };
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
    /// let alice = graph.do_for_a_thing(|thing| {
    ///     thing.access(|data| return if data.name == "Alice" { Do::Take(thing) } else { Do::Nothing })
    /// });
    /// ```
    pub fn do_for_a_thing<R>(&self, do_for: impl Fn(&Thing<T, C>) -> Do<R>) -> Option<R> {
        for thing in &self.things {
            if let Do::Take(value) = do_for(thing) {
                return Some(value);
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
    pub fn do_for_all_things<R>(&self, get: impl Fn(&Thing<T, C>) -> Do<R>) -> Vec<R> {
        let mut things = Vec::new();
        for thing in &self.things {
            if let Do::Take(value) = get(thing) {
                things.push(value);
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
    /// // Remove all temporary items
    /// graph.kill_things(|thing| {
    ///     thing.access(|data| data.is_temporary)
    /// });
    /// ```
    pub fn kill_things(&mut self, kill: impl Fn(&Thing<T, C>) -> bool) {
        self.things.iter().for_each(|thing| {
            if kill(thing) {
                let amount = thing.kill();
                self.dead_amount = self
                    .dead_amount
                    .checked_add(amount)
                    .unwrap_or_else(|| usize::MAX);
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
    /// let friendship = graph.do_for_a_connection(|conn| {
    ///     conn.access(|data| return if *data == "friendship" { Do::Take(conn) } else { Do::Nothing })
    /// });
    /// ```
    pub fn do_for_a_connection<'l, R>(
        &self,
        get: impl Fn(&Connection<T, C>) -> Do<R>,
    ) -> Option<R> {
        for connection in &self.connections {
            if let Do::Take(value) = get(connection) {
                return Some(value);
            }
        }
        None
    }

    /// Finds all connections that match the given predicate.
    ///
    /// Useful for analyzing relationship patterns or finding all connections
    /// of a particular type.
    pub fn do_for_all_connections<R>(&self, found: impl Fn(&Connection<T, C>) -> Do<R>) -> Vec<R> {
        let mut connections = Vec::new();
        for connection in &self.connections {
            if let Do::Take(value) = found(connection) {
                connections.push(value);
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
    /// // Remove all temporary relationships
    /// graph.kill_connections(|conn| {
    ///     conn.access(|data| data.is_temporary)
    /// });
    /// ```
    pub fn kill_connections(&mut self, kill: impl Fn(&Connection<T, C>) -> bool) {
        self.connections.iter().for_each(|connection| {
            if kill(connection) {
                connection.kill();
                let _ = self.dead_amount.saturating_add(1);
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
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
            .unwrap_or_else(|| usize::MAX);

        if total == 0 {
            self.dead_amount = 0;
            return Err(());
        }

        let multiplied = self
            .dead_amount
            .checked_mul(100)
            .unwrap_or_else(|| usize::MAX);

        let divided = multiplied / total;

        Ok(divided)
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
    /// # use connect_things::*;
    /// # let mut graph = Things::new();
    ///
    /// // Clean up when memory pressure gets high
    /// if graph.dead_percentage().unwrap_or(0) > 30 {
    ///     graph.clean();
    ///     println!("Graph cleaned");
    /// }
    /// ```
    pub fn clean(&mut self) {
        self.things.retain_mut(|thing| {
            return if thing.is_alive() {
                thing.clean();
                true
            } else {
                false
            };
        });

        self.connections.retain(|connection| connection.is_alive());

        self.dead_amount = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::{String, ToString};

    /// Creates a sample knowledge graph for testing.
    /// This represents a simple taxonomy with foods, categories, and preferences.
    fn test_knowledge_graph<'a>() -> Things<&'a str, &'a str> {
        let mut graph = Things::<&str, &str>::new();

        let apple = graph.new_thing("Apple");
        let apples = graph.new_thing("Apples");
        graph.new_directed_connection(apples.clone(), "plural of", apple.clone());

        let pear = graph.new_thing("Pear");
        let pears = graph.new_thing("Pears");
        graph.new_directed_connection(pears.clone(), "plural of", pear.clone());

        let alice = graph.new_thing("Alice");
        graph.new_directed_connection(alice.clone(), "likes to eat", apples);
        graph.new_directed_connection(alice, "doesn't like to eat", pears);

        let fruit = graph.new_thing("Fruit");
        graph.new_directed_connection(apple, "is", fruit.clone());
        graph.new_directed_connection(pear, "is", fruit);

        graph
    }

    #[test]
    fn knowledge_representation_basic_query() {
        let graph = test_knowledge_graph();

        // Query: What does Alice like to eat?
        let alice = graph
            .do_for_a_thing(|thing| {
                return if thing.access(|data| *data == "Alice") {
                    Do::Take(thing.clone())
                } else {
                    Do::Nothing
                };
            })
            .unwrap();

        let liked_food_connection = alice
            .do_for_a_connection(|connection| {
                return if connection.access(|data| *data == "likes to eat") {
                    Do::Take(connection.clone())
                } else {
                    Do::Nothing
                };
            })
            .unwrap();

        // Use the new API that returns Option
        let liked_food = liked_food_connection.get_directed_towards().unwrap();

        let answer = format!(
            "The thing alice likes to eat is: {}.",
            liked_food.access(|data| data.to_ascii_lowercase())
        );

        assert_eq!("The thing alice likes to eat is: apples.", &answer);
    }

    #[test]
    fn knowledge_representation_taxonomy_query() {
        let graph = test_knowledge_graph();

        // Query: What are some examples of fruit?
        let fruit_concept = graph
            .do_for_a_thing(|thing| {
                return if thing.access(|data| *data == "Fruit") {
                    Do::Take(thing.clone())
                } else {
                    Do::Nothing
                };
            })
            .unwrap();

        // Find all things that are instances of fruit
        let fruit_examples: Vec<_> = graph.do_for_all_connections(|conn| {
            // Find "is" relationships pointing to the fruit concept
            return if conn.access(|data| *data == "is") {
                if let Ok(Direction::Towards) = conn.get_direction_relative_to(&fruit_concept) {
                    Do::Take(conn.get_directed_from().unwrap().access(|data| *data))
                } else {
                    Do::Nothing
                }
            } else {
                Do::Nothing
            };
        });

        assert!(fruit_examples.contains(&"Apple"));
        assert!(fruit_examples.contains(&"Pear"));
        assert_eq!(fruit_examples.len(), 2);
    }

    #[test]
    fn social_network_simulation() {
        let mut social_graph = Things::<String, String>::new();

        // Create people
        let alice = social_graph.new_thing("Alice".to_string());
        let bob = social_graph.new_thing("Bob".to_string());
        let charlie = social_graph.new_thing("Charlie".to_string());
        let diana = social_graph.new_thing("Diana".to_string());

        // Create friendships (undirected relationships)
        social_graph
            .new_undirected_connection([alice.clone(), bob.clone()], "friendship".to_string());
        social_graph
            .new_undirected_connection([bob.clone(), charlie.clone()], "friendship".to_string());
        social_graph
            .new_undirected_connection([alice.clone(), diana.clone()], "friendship".to_string());

        // Create follows relationships (directed)
        social_graph.new_directed_connection(charlie.clone(), "follows".to_string(), alice.clone());
        social_graph.new_directed_connection(diana.clone(), "follows".to_string(), bob.clone());

        // Test: Find Alice's friends
        let alice_friendships = alice.do_for_all_connections(|conn| {
            return if conn.is_undirected() && conn.access(|data| data == "friendship") {
                Do::Take(conn.clone())
            } else {
                Do::Nothing
            };
        });

        assert_eq!(alice_friendships.len(), 2); // Alice is friends with Bob and Diana

        // Test: Find who follows Alice
        let alice_followers: Vec<_> = social_graph.do_for_all_connections(|conn| {
            return if conn.is_directed() && conn.access(|data| data == "follows") {
                conn.get_directed_towards().unwrap().access(|data| {
                    return if data == "Alice" {
                        Do::Take(
                            conn.get_directed_from()
                                .unwrap()
                                .access(|data| data.clone()),
                        )
                    } else {
                        Do::Nothing
                    };
                })
            } else {
                Do::Nothing
            };
        });

        assert!(alice_followers.contains(&"Charlie".to_string()));
        assert_eq!(alice_followers.len(), 1);
    }

    #[test]
    fn gui_component_hierarchy() {
        // Simulate a simple GUI structure with containment and focus relationships
        #[derive(Debug, Clone, PartialEq)]
        struct Widget {
            name: String,
            widget_type: String,
        }

        #[derive(Debug, Clone, PartialEq)]
        enum Relationship {
            Contains,
            FocusNext,
            EventBubbles,
        }

        let mut gui = Things::<Widget, Relationship>::new();

        // Create widgets
        let window = gui.new_thing(Widget {
            name: "MainWindow".to_string(),
            widget_type: "Window".to_string(),
        });

        let dialog = gui.new_thing(Widget {
            name: "SettingsDialog".to_string(),
            widget_type: "Dialog".to_string(),
        });

        let ok_button = gui.new_thing(Widget {
            name: "OkButton".to_string(),
            widget_type: "Button".to_string(),
        });

        let cancel_button = gui.new_thing(Widget {
            name: "CancelButton".to_string(),
            widget_type: "Button".to_string(),
        });

        // Create containment hierarchy
        gui.new_directed_connection(window.clone(), Relationship::Contains, dialog.clone());
        gui.new_directed_connection(dialog.clone(), Relationship::Contains, ok_button.clone());
        gui.new_directed_connection(
            dialog.clone(),
            Relationship::Contains,
            cancel_button.clone(),
        );

        // Create focus chain
        gui.new_directed_connection(
            ok_button.clone(),
            Relationship::FocusNext,
            cancel_button.clone(),
        );
        gui.new_directed_connection(
            cancel_button.clone(),
            Relationship::FocusNext,
            ok_button.clone(),
        );

        // Create event bubbling relationships
        gui.new_directed_connection(
            ok_button.clone(),
            Relationship::EventBubbles,
            dialog.clone(),
        );
        gui.new_directed_connection(
            cancel_button.clone(),
            Relationship::EventBubbles,
            dialog.clone(),
        );

        // Test: Find all widgets contained in the dialog
        let dialog_children: Vec<_> = dialog.do_for_all_connections(|conn| {
            conn.access(|data| {
                if matches!(data, Relationship::Contains) {
                    if let Some(from) = conn.get_directed_from() {
                        if from == dialog {
                            Do::Take(
                                conn.get_directed_towards()
                                    .unwrap()
                                    .access(|data| data.name.clone()),
                            )
                        } else {
                            Do::Nothing
                        }
                    } else {
                        Do::Nothing
                    }
                } else {
                    Do::Nothing
                }
            })
        });

        assert!(dialog_children.contains(&"OkButton".to_string()));
        assert!(dialog_children.contains(&"CancelButton".to_string()));
        assert_eq!(dialog_children.len(), 2);

        // Test: Find the next widget in focus chain from OK button
        let next_focus = ok_button.do_for_a_connection(|conn| {
            conn.access(|data| {
                return if matches!(data, Relationship::FocusNext) {
                    if let Ok(Direction::AwayFrom) = conn.get_direction_relative_to(&ok_button) {
                        if let Some(to) = conn.get_directed_towards() {
                            Do::Take(to.access(|data| data.name.clone()))
                        } else {
                            Do::Nothing
                        }
                    } else {
                        Do::Nothing
                    }
                } else {
                    Do::Nothing
                };
            })
        });

        assert_eq!(next_focus, Some("CancelButton".to_string()));
    }

    #[test]
    fn task_dependency_graph() {
        #[derive(Debug, Clone, PartialEq)]
        struct Task {
            name: String,
            estimated_hours: u32,
            completed: bool,
        }

        #[derive(Debug, Clone, PartialEq)]
        enum TaskRelation {
            DependsOn,
            // Blocks,
            // PartOf,
        }

        let mut project = Things::<Task, TaskRelation>::new();

        // Create tasks
        let design = project.new_thing(Task {
            name: "Design System".to_string(),
            estimated_hours: 40,
            completed: true,
        });

        let implement_auth = project.new_thing(Task {
            name: "Implement Authentication".to_string(),
            estimated_hours: 20,
            completed: false,
        });

        let implement_ui = project.new_thing(Task {
            name: "Implement UI".to_string(),
            estimated_hours: 60,
            completed: false,
        });

        let testing = project.new_thing(Task {
            name: "Integration Testing".to_string(),
            estimated_hours: 30,
            completed: false,
        });

        let deployment = project.new_thing(Task {
            name: "Deployment".to_string(),
            estimated_hours: 10,
            completed: false,
        });

        // Create dependencies
        project.new_directed_connection(
            implement_auth.clone(),
            TaskRelation::DependsOn,
            design.clone(),
        );
        project.new_directed_connection(
            implement_ui.clone(),
            TaskRelation::DependsOn,
            design.clone(),
        );
        project.new_directed_connection(
            testing.clone(),
            TaskRelation::DependsOn,
            implement_auth.clone(),
        );
        project.new_directed_connection(
            testing.clone(),
            TaskRelation::DependsOn,
            implement_ui.clone(),
        );
        project.new_directed_connection(
            deployment.clone(),
            TaskRelation::DependsOn,
            testing.clone(),
        );

        // Test: Find all tasks that can be started now (dependencies completed)
        let incomplete_tasks: Vec<_> = project.do_for_all_things(|task| {
            return if !task.access(|data| data.completed) {
                Do::Take(task.clone())
            } else {
                Do::Nothing
            };
        });

        let ready_tasks: Vec<_> = incomplete_tasks
            .iter()
            .map(|task| {
                if task
                    .do_for_all_connections(|conn| {
                        if let Ok(Direction::AwayFrom) = conn.get_direction_relative_to(task) {
                            conn.access(|data| {
                                return if matches!(data, TaskRelation::DependsOn) {
                                    return if let Some(to) = conn.get_directed_towards() {
                                        Do::Take(to.access(|data| data.completed))
                                    } else {
                                        Do::Nothing
                                    };
                                } else {
                                    Do::Nothing
                                };
                            })
                        } else {
                            Do::Nothing
                        }
                    })
                    .iter()
                    .all(|x| *x)
                {
                    Some(task.clone())
                } else {
                    None
                }
            })
            .filter_map(|v| v.clone())
            .map(|v| v.access(|data| data.name.clone()))
            .collect();

        // Only Auth and UI should be ready (Design is completed)
        assert!(ready_tasks.contains(&"Implement Authentication".to_string()));
        assert!(ready_tasks.contains(&"Implement UI".to_string()));
        assert!(!ready_tasks.contains(&"Integration Testing".to_string())); // Depends on incomplete tasks
        assert!(!ready_tasks.contains(&"Deployment".to_string())); // Depends on incomplete tasks
    }

    #[test]
    fn memory_pressure_tracking() {
        let mut graph = Things::new();

        // Create some items
        let thing1 = graph.new_thing("Thing1");
        let thing2 = graph.new_thing("Thing2");
        let thing3 = graph.new_thing("Thing3");

        let _conn1 = graph.new_directed_connection(thing1, "relates", thing2.clone());
        let _conn2 = graph.new_directed_connection(thing2, "relates", thing3);

        // Initially, no dead items
        assert_eq!(graph.dead_percentage().unwrap(), 0);

        // Kill one thing (should kill the thing and its connections)
        graph.kill_things(|thing| thing.access(|data| data == &"Thing1"));

        // Should have some dead percentage now
        let percentage_after_kill = graph.dead_percentage().unwrap();
        assert!(percentage_after_kill > 0);
        assert!(percentage_after_kill <= 100);

        // Clean up and verify percentage returns to 0
        graph.clean();
        assert_eq!(graph.dead_percentage().unwrap(), 0);

        // Verify remaining items are still accessible
        let remaining_things = graph.do_for_all_things(|_| Do::Take(()));
        assert!(remaining_things.len() > 0); // Should have some things left
    }

    #[test]
    fn cascade_deletion_behavior() {
        let mut graph = Things::new();

        let alice = graph.new_thing("Alice");
        let bob = graph.new_thing("Bob");
        let charlie = graph.new_thing("Charlie");

        // Create connections: Alice -> Bob, Bob -> Charlie
        graph.new_directed_connection(alice.clone(), "knows", bob.clone());
        graph.new_directed_connection(bob.clone(), "knows", charlie.clone());

        // Kill Bob - this should kill Bob and all his connections
        graph.kill_things(|thing| thing.access(|data| data == &"Bob"));

        // Alice and Charlie should still be alive
        assert!(alice.access(|_| true)); // Can still access Alice's data
        assert!(charlie.access(|_| true)); // Can still access Charlie's data

        // But Bob's connections should be dead
        let alice_connections = alice.do_for_all_connections(|_| Do::Take(()));
        // Alice's connection to Bob should still exist but be marked as dead
        assert!(alice_connections.len() > 0);

        // After cleanup, dead connections should be removed
        graph.clean();
        let alice_connections_after_clean = alice.do_for_all_connections(|_| Do::Take(()));
        assert_eq!(alice_connections_after_clean.len(), 0); // Alice should have no live connections
    }

    #[test]
    fn undirected_connections_behavior() {
        let mut graph = Things::<String, String>::new();

        let alice = graph.new_thing("Alice".to_string());
        let bob = graph.new_thing("Bob".to_string());

        // Create undirected friendship
        let friendship =
            graph.new_undirected_connection([alice.clone(), bob.clone()], "friendship".to_string());

        let find_friendships = |thing: &Thing<_, _>| {
            thing.do_for_all_connections(|conn| {
                conn.access(|data| {
                    return if data == "friendship" {
                        Do::Take(conn.clone())
                    } else {
                        Do::Nothing
                    };
                })
            })
        };

        let alice_friendships = find_friendships(&alice);
        let bob_friendships = find_friendships(&bob);

        // Both Alice and Bob should have the same connection in their lists

        assert_eq!(alice_friendships.len(), 1);
        assert_eq!(bob_friendships.len(), 1);

        // The connection should be marked as undirected
        assert!(friendship.is_undirected());
        assert!(!friendship.is_directed());

        // Directional methods should return None for undirected connections
        assert!(friendship.get_directed_from().is_none());
        assert!(friendship.get_directed_towards().is_none());

        // Both people should be reachable from the connection using get_connected_things
        let connected = friendship.get_things();
        let names: Vec<String> = connected
            .iter()
            .map(|thing| thing.access(|data| data.clone()))
            .collect();

        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));
    }

    #[test]
    fn directed_connection_safety() {
        let mut graph = Things::<String, String>::new();

        let manager = graph.new_thing("Manager".to_string());
        let employee = graph.new_thing("Employee".to_string());

        // Create directed management relationship
        let manages =
            graph.new_directed_connection(manager.clone(), "manages".to_string(), employee.clone());

        // Connection should be marked as directed
        assert!(manages.is_directed());
        assert!(!manages.is_undirected());

        // Directional methods should work correctly
        let from_person = manages.get_directed_from().unwrap();
        let to_person = manages.get_directed_towards().unwrap();

        assert_eq!(from_person.access(|data| data.clone()), "Manager");
        assert_eq!(to_person.access(|data| data.clone()), "Employee");

        // get_connected_things should return [from, to]
        let connected = manages.get_things();
        assert_eq!(connected[0].access(|data| data.clone()), "Manager");
        assert_eq!(connected[1].access(|data| data.clone()), "Employee");
    }

    #[test]
    fn complex_knowledge_query() {
        // Test a more complex knowledge representation scenario
        let mut knowledge = Things::<String, String>::new();

        // Create a small taxonomy
        let animal = knowledge.new_thing("Animal".to_string());
        let mammal = knowledge.new_thing("Mammal".to_string());
        let dog = knowledge.new_thing("Dog".to_string());
        let cat = knowledge.new_thing("Cat".to_string());

        let fido = knowledge.new_thing("Fido".to_string());
        let whiskers = knowledge.new_thing("Whiskers".to_string());

        // Build taxonomy relationships
        knowledge.new_directed_connection(mammal.clone(), "is_a".to_string(), animal.clone());
        knowledge.new_directed_connection(dog.clone(), "is_a".to_string(), mammal.clone());
        knowledge.new_directed_connection(cat.clone(), "is_a".to_string(), mammal.clone());

        // Instance relationships
        knowledge.new_directed_connection(fido.clone(), "instance_of".to_string(), dog.clone());
        knowledge.new_directed_connection(whiskers.clone(), "instance_of".to_string(), cat.clone());

        // Query: Find all animals (instances that are transitively related to Animal)
        // This tests multi-hop traversal
        let mut animal_instances = Vec::new();

        // Find all instances
        for instance_conn in knowledge.do_for_all_connections(|conn| {
            conn.access(|data| {
                return if data == "instance_of" {
                    Do::Take(conn.clone())
                } else {
                    Do::Nothing
                };
            })
        }) {
            if let Some(instance) = instance_conn.get_directed_from() {
                if let Some(species) = instance_conn.get_directed_towards() {
                    // Check if this species is ultimately an animal
                    let mut current = species;
                    let mut is_animal = false;

                    // Traverse up the hierarchy
                    for _ in 0..10 {
                        // Prevent infinite loops
                        if current.access(|data| data == "Animal") {
                            is_animal = true;
                            break;
                        }

                        // Find parent class
                        if let Some(parent_conn) = current.do_for_a_connection(|conn| {
                            conn.access(|data| {
                                return if data == "is_a" {
                                    Do::Take(conn.clone())
                                } else {
                                    Do::Nothing
                                };
                            })
                        }) {
                            if let Some(parent) = parent_conn.get_directed_towards() {
                                current = parent;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if is_animal {
                        animal_instances.push(instance.access(|data| data.clone()));
                    }
                }
            }
        }

        assert!(animal_instances.contains(&"Fido".to_string()));
        assert!(animal_instances.contains(&"Whiskers".to_string()));
        assert_eq!(animal_instances.len(), 2);
    }
}
