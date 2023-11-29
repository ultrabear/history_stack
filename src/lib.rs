//! A crate implementing generic history managers that can act as building blocks for transactional
//! state and reversible computations

#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::alloc_instead_of_core, clippy::std_instead_of_alloc)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
extern crate alloc;

use core::{cmp, fmt, hash, ops};

use alloc::vec::Vec;

/// A wrapper over a `T` that provides a primitive history mechanism by use of a stack of `T`. It
/// can be pushed to or popped from to save the current value or pop out a previously saved value
/// in LIFO (stack) order.
///
/// `HistoryStack` is also "transparently T", meaning the default traits it implements all act like
/// the current value of T, so hashing `HistoryStack<T>` and T produce the same hash, Eq and Ord work
/// the same etc. This also includes `Display`, but does not include `Debug`.
#[derive(Clone, Default, Debug)]
pub struct HistoryStack<T> {
    /// The history stack, this starts out empty and should only be modified via pushing and popping
    stack: Vec<T>,
    /// The current value, since `HistoryStack<T>` acts like a T, this is always initialized to
    /// some value
    current: T,
}

impl<T: fmt::Display> fmt::Display for HistoryStack<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.current.fmt(f)
    }
}

impl<T> HistoryStack<T> {
    /// Create a new `HistoryStack` whose current value is set to `v`, with no history
    pub const fn new(v: T) -> Self {
        Self {
            stack: Vec::new(),
            current: v,
        }
    }

    /// Pop a value from the stack and set it as the current value, returning the previous current
    /// value.
    ///
    /// Returns `None` in the case that there was no previous value from the stack, current is also
    /// unchanged.
    pub fn pop(&mut self) -> Option<T> {
        match self.stack.pop() {
            Some(last) => Some(core::mem::replace(&mut self.current, last)),
            None => None,
        }
    }

    /// Pushes a value into the current value, and pushes the previously current value into the
    /// stack.
    pub fn push_value(&mut self, v: T) {
        self.stack.push(core::mem::replace(&mut self.current, v));
    }

    /// Makes a [`Clone::clone`] of the current value and pushes it to the stack, leaving the
    /// current value untouched
    pub fn push(&mut self)
    where
        T: Clone,
    {
        self.stack.push(self.current.clone());
    }
}

impl<T> ops::Deref for HistoryStack<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl<T> ops::DerefMut for HistoryStack<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

impl<T: PartialEq> PartialEq<T> for HistoryStack<T> {
    fn eq(&self, other: &T) -> bool {
        &self.current == other
    }
}

impl<T: PartialEq> PartialEq for HistoryStack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }
}

impl<T: Eq> Eq for HistoryStack<T> {}

impl<T: PartialOrd> PartialOrd for HistoryStack<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.current.partial_cmp(&other.current)
    }
}

impl<T: PartialOrd> PartialOrd<T> for HistoryStack<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.current.partial_cmp(other)
    }
}

impl<T: Ord> Ord for HistoryStack<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.current.cmp(&other.current)
    }
}

impl<T: hash::Hash> hash::Hash for HistoryStack<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.current.hash(state);
    }
}

/// A structure which allows you to undo and redo changes based on saved states of `T`.
///
/// To use, simply [`save`](UndoStack::save), [`undo`](UndoStack::undo), and
/// [`redo`](UndoStack::redo) later if needed;
/// ```rust
/// # use history_stack::UndoStack;
/// // Create with initial state
/// let mut undo = UndoStack::new(5u8);
///
/// // make a savepoint and get a reference to the new current value
/// // our stack looks like [5, 5] currently, our current value being the second
/// let newref = undo.save();
///
/// // we modified the new current value, our stack looks like [5, 10] now
/// *newref *= 2;
///
/// // but we made a mistake! we want to go back now, and since we are
/// // sure we saved earlier we can unwrap here to get the Ok variant
/// // our stack still looks like [5, 10], but now we point to the 5
/// let oldref = undo.undo().unwrap();
///
/// // turns out it wasnt a mistake, lets redo and unwrap to be sure we got the newer value
/// undo.redo().unwrap();
///
/// // UndoStack implements Deref and DerefMut, we can make sure we got the new value like this
/// assert_eq!(undo, 10);
/// ```
///
/// This is useful when you want to be able to make changes in a way where you can undo a change,
/// and then reapply it later, but do not wish to write a complex incremental structure that could
/// track changes like that. This type provides a generic (read: you can use it on anything)
/// interface to achieve that effect, even if it may use more memory than a more targeted approach.
///
/// `UndoStack` is also "transparently T", meaning the default traits it implements all act like
/// the current value of T, so hashing `UndoStack<T>` and T produce the same hash, Eq and Ord work
/// the same etc. This also includes `Display`, but does not include `Debug`.
#[derive(Clone, Debug)]
pub struct UndoStack<T> {
    /// History of the undostack that includes the current value somewhere within
    history: Vec<T>,
    /// Index into history that represents the current value
    current: usize,
}

impl<T: Default> Default for UndoStack<T> {
    fn default() -> Self {
        Self {
            history: alloc::vec![T::default()],
            current: 0,
        }
    }
}

impl<T: fmt::Display> fmt::Display for UndoStack<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner().fmt(f)
    }
}

impl<T> UndoStack<T> {
    /// Creates a new `UndoStack` with a starting value to act as the current value
    pub fn new(start: T) -> Self {
        Self {
            history: alloc::vec![start],
            current: 0,
        }
    }

    /// Drops any values that exist after the current value
    fn invalidate_future(&mut self) {
        // we have hit undo if these values do not match up, so we must invalidate the redo stack
        // it is safe to do current+1 because current is <history.len() which is stored
        // as usize aswell
        if self.current + 1 != self.history.len() {
            // see above for +1 safety
            self.history.truncate(self.current + 1);
        }
    }

    /// Pushes a value assuming the current value is the last value
    /// returns a reference to the new current value (the value that was just pushed)
    fn push_unchecked(&mut self, val: T) -> &mut T {
        self.history.push(val);

        // +1 safety: current is always less than history.len(), which would panic on overflow
        self.current += 1;

        &mut self.history[self.current]
    }

    /// Saves the current T to history and invalidates any data that may be used to redo
    /// This will [`Drop`] any T that exist later in history than the current edit point.
    ///
    /// Returns a reference to the new current value
    ///
    /// # Panics
    /// This will panic if allocation failed
    pub fn save(&mut self) -> &mut T
    where
        T: Clone,
    {
        self.invariant_ck();

        self.invalidate_future();

        // safe to unwrap here because history is always nonempty
        let val = self.history.last().unwrap().clone();

        self.push_unchecked(val)
    }

    /// Pushes the given value to the stack, making it the new current value and invalidating
    /// future history, returns a reference to the new current value
    ///
    /// This is functionally identical to [`save`](UndoStack::save) but does not have a `Clone`
    /// bound, instead sourcing its new value from the caller.
    ///
    /// # Panics
    /// This will panic if allocation failed
    pub fn push(&mut self, new_current: T) -> &mut T {
        self.invariant_ck();

        self.invalidate_future();

        self.push_unchecked(new_current)
    }

    /// If there is a previous state in the history stack, backtrack to that and return `Ok(&mut T)`
    /// to the new current value, otherwise return `Err(&mut T)` to the unchanged current value.
    #[allow(clippy::missing_errors_doc)]
    pub fn undo(&mut self) -> Result<&mut T, &mut T> {
        self.invariant_ck();

        match self.current.checked_sub(1) {
            Some(n) => {
                self.current = n;
                Ok(&mut self.history[self.current])
            }
            None => {
                // current was 0
                Err(&mut self.history[0])
            }
        }
    }

    /// If there is a future state in the history stack that has been undone from, redo to that
    /// position and return `Ok(&mut T)` of the new current value after advancing, else return
    /// `Err(&mut T)` of the current unchanged value, if there was no future history.
    #[allow(clippy::missing_errors_doc)]
    pub fn redo(&mut self) -> Result<&mut T, &mut T> {
        self.invariant_ck();

        if self.current + 1 == self.history.len() {
            Err(&mut self.history[self.current])
        } else {
            self.current += 1;

            Ok(&mut self.history[self.current])
        }
    }

    /// function that runs in debug and checks all trivial invariants of `UndoStack`
    fn invariant_ck(&self) {
        debug_assert!(
            !self.history.is_empty(),
            "UndoStack: history was empty, this indicates a bug in UndoStack"
        );
        debug_assert!(self.current < self.history.len(), "UndoStack: current was not less than history length, this indicates a bug in UndoStack");
    }

    /// Gets a reference to the current value
    /// used to implement traits via T without accidental recursion
    fn inner(&self) -> &T {
        self
    }
}

impl<T> ops::Deref for UndoStack<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.history[self.current]
    }
}

impl<T> ops::DerefMut for UndoStack<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.history[self.current]
    }
}

impl<T: PartialEq> PartialEq<T> for UndoStack<T> {
    fn eq(&self, other: &T) -> bool {
        self.inner() == other
    }
}

impl<T: PartialEq> PartialEq for UndoStack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner() == other.inner()
    }
}

impl<T: Eq> Eq for UndoStack<T> {}

impl<T: PartialOrd> PartialOrd for UndoStack<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner().partial_cmp(other.inner())
    }
}

impl<T: PartialOrd> PartialOrd<T> for UndoStack<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.inner().partial_cmp(other)
    }
}

impl<T: Ord> Ord for UndoStack<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner().cmp(other.inner())
    }
}

impl<T: hash::Hash> hash::Hash for UndoStack<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.inner().hash(state);
    }
}

#[test]
fn undo_stack() {
    let mut g = UndoStack::new(0u8);

    *g.save() += 1;

    assert_eq!(g, 1);

    assert_eq!(*g.undo().unwrap(), 0);

    assert_eq!(*g.redo().unwrap(), 1);

    assert!(g.undo().is_ok());

    *g.save() += 2;

    assert!(g.redo().is_err());
}

#[test]
fn history_stack() {
    let mut g = HistoryStack::new(0u8);

    g.push_value(5);

    assert_eq!(g, 5);

    assert_eq!(g.pop(), Some(5));

    assert_eq!(g, 0);
}
