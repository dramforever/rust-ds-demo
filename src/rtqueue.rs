//! Real-time `O(1)` fully persistent FIFO queue implementation.
//!
//! **Note**: If you just want a queue data structure,
//! [`VecDeque`](std::collections::VecDeque) from [`std`] is the way to go. The
//! key here is providing *persistence*, which is explained below, and is rarely
//! if ever needed.
//!
//! 'Fully persistent' means that each operation on a queue creates a new queue
//! and does not invalidate the original one.
//!
//! For example:
//!
//! ```
//! use rust_ds_demo::rtqueue::Queue;
//! use std::iter::FromIterator;
//!
//! // A new queue with only one element 3:
//! let a: Queue<i32> = Queue::new().push_back(3);
//!
//! assert_eq!(Vec::from_iter(&a), vec![ 3 ]);
//!
//! // Use `a` twice here:
//! let b1 = a.push_back(4);
//! let b2 = a.push_back(5);
//!
//! // `a` is unchanged:
//! assert_eq!(Vec::from_iter(&a), vec![ 3 ]);
//!
//! // `b1` and `b2` have 4 and 5 pushed into it respectively:
//! assert_eq!(Vec::from_iter(&b1), vec![ 3, 4 ]);
//! assert_eq!(Vec::from_iter(&b2), vec![ 3, 5 ]);
//! ```
//!
//! Real-time `O(1)` means that each operation: [`new`](rtqueue::Queue::new),
//! [`push_back`](rtqueue::Queue::push_back),
//! [`pop_front`](rtqueue::Queue::pop_front) and [`clone`](Clone) on a
//! [`Queue<T>`](rtqueue::Queue) clones values of type `T` a number of times
//! bounded by a constant, and takes a further, bounded by a constant time doing
//! other organizational work. In other words, the maximum time taken by each
//! operation stays the same no matter how many items are contained in it.
//!
//! # Note on the `Clone` trait bound
//!
//! The data structure needs to clone items as part of its normal operation. If
//! cloning is not possible or too expensive, consider using
//! [`Rc<T>`](std::rc::Rc) as the item type.
//!
//! # References
//!
//! - Chris Okasaki, *Purely Functional Data Structures*
//! - Edsko de Vries, [*Efficient Amortised and Real-Time Queues in
//!   Haskell*](https://www.well-typed.com/blog/2016/01/efficient-queues/)
//! - [*Queue*, subsection *Real-time queue* on
//!   Wikipedia](https://en.wikipedia.org/wiki/Queue_(abstract_data_type)#Real-time_queue)

use std::cell::{Cell, RefCell};
use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;

#[derive(Clone)]
enum Zipper<T> {
    Clean,
    Dirty(Rc<Node<T>>, Option<Rc<Node<T>>>),
}

struct Node<T> {
    value: T,
    next: RefCell<Option<Rc<Node<T>>>>,
    zipper: Cell<Zipper<T>>,
}

impl<T: Clone> Node<T> {
    fn create_lazy(x: &Option<Rc<Node<T>>>, y: Option<Rc<Node<T>>>) -> Option<Rc<Node<T>>> {
        if let Some(x) = x {
            let y = y.expect("create_lazy: imbalance");

            Some(Rc::new(Node {
                value: x.value.clone(),
                next: x.next.clone(),
                zipper: Cell::new(Zipper::Dirty(y, None)),
            }))
        } else {
            y
        }
    }

    fn rotate_zipper(&self) {
        if let Zipper::Dirty(c, d) = self.zipper.replace(Zipper::Clean) {

            let node = Rc::new(Node {
                value: c.value.clone(),
                next: RefCell::new(d),
                zipper: Cell::new(Zipper::Clean),
            });

            let c_next = c.next.borrow().clone()
                .expect("rotate_zipper: imbalance");

            let new_next =
                if let Some(next) = self.next.borrow().clone() {
                    Node {
                        value: next.value.clone(),
                        next: next.next.clone(),
                        zipper: Cell::new(
                            Zipper::Dirty(c_next, Some(node))),
                    }
                } else {
                    Node {
                        value: c_next.value.clone(),
                        next: RefCell::new(Some(node)),
                        zipper: Cell::new(Zipper::Clean)
                    }
                };

            self.next.replace(Some(Rc::new(new_next)));
        }
    }
}

/// A real-time `O(1)` fully persistent FIFO queue.
///
/// See [module level documentation](crate::rtqueue) for usage details.
#[derive(Clone)]
pub struct Queue<T> {
    front: Option<Rc<Node<T>>>,
    back: Option<Rc<Node<T>>>,
    jump: Option<Rc<Node<T>>>,
}

impl<T: Clone> Default for Queue<T> {
    fn default() -> Queue<T> {
        Queue {
            front: None,
            back: None,
            jump: None,
        }
    }
}

impl<T: Clone> Queue<T> {
    /// Create a new, empty queue.
    pub fn new() -> Queue<T> {
        Default::default()
    }

    /// Pop an item from the front of the queue.
    ///
    /// Returns `None` if the queue is empty, and `Some` with new queue and
    /// popped element otherwise.
    pub fn pop_front(&self) -> Option<(Queue<T>, T)> {
        let front = self.front.as_ref()?;
        let res = front.value.clone();

        let res_queue =
            if let Some(jump) = &self.jump {
                jump.rotate_zipper();

                Queue {
                    front: front.next.borrow().clone(),
                    back: self.back.clone(),
                    jump: jump.next.borrow().clone(),
                }
            } else {
                let zipper = Node::create_lazy(&front.next.borrow(), self.back.clone());

                Queue {
                    front: zipper.clone(),
                    back: None,
                    jump: zipper,
                }
            };

        Some((res_queue, res))
    }

    /// Push an item into the back of the queue.
    pub fn push_back(&self, v: T) -> Queue<T> {
        let new_node = Rc::new(Node {
            value: v,
            next: RefCell::new(self.back.clone()),
            zipper: Cell::new(Zipper::Clean),
        });

        if let Some(jump) = &self.jump {
            jump.rotate_zipper();

            Queue {
                front: self.front.clone(),
                back: Some(new_node),
                jump: jump.next.borrow().clone(),
            }
        } else {
            let zipper = Node::create_lazy(&self.front, Some(new_node));

            Queue {
                front: zipper.clone(),
                back: None,
                jump: zipper,
            }
        }
    }
}

pub struct QueueIter<T> {
    dat: Queue<T>
}

impl<T: Clone> Iterator for QueueIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let (next, res) = self.dat.pop_front()?;
        self.dat = next;
        Some(res)
    }
}

impl<T: Clone> IntoIterator for &'_ Queue<T> {
    type Item = T;
    type IntoIter = QueueIter<T>;

    fn into_iter(self) -> QueueIter<T> {
        QueueIter { dat: self.clone() }
    }
}

impl<T: Clone> FromIterator<T> for Queue<T> {
    fn from_iter<I: IntoIterator<Item=T>>(x: I) -> Queue<T> {
        x.into_iter().fold(Queue::new(), |q, v| Queue::push_back(&q, v))
    }
}

impl<T: fmt::Debug + Clone> fmt::Debug for Queue<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

#[test]
fn test_queue() {
    let a = Queue::new()
        .push_back(1)
        .push_back(2);
    let b = a
        .push_back(3)
        .push_back(4);
    let c = a
        .push_back(5)
        .push_back(6);
    let b1 = b
        .pop_front().unwrap().0
        .pop_front().unwrap().0;
    let c1 = c
        .pop_front().unwrap().0
        .pop_front().unwrap().0;

    assert!(Vec::from_iter(&a) == vec![ 1, 2 ]);
    assert!(Vec::from_iter(&b) == vec![ 1, 2, 3, 4 ]);
    assert!(Vec::from_iter(&c) == vec![ 1, 2, 5, 6 ]);
    assert!(Vec::from_iter(&b1) == vec![ 3, 4 ]);
    assert!(Vec::from_iter(&c1) == vec![ 5, 6 ]);
}
