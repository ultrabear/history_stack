//! A crate implementing a `HistoryStack` that lets you push to store a state and pop to retrieve it
//! later

#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::alloc_instead_of_core, clippy::std_instead_of_alloc)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
//#![warn(missing_docs, clippy::missing_docs_in_private_items)]
extern crate alloc;

use core::{cmp, hash, ops};

use alloc::vec::Vec;

/// A wrapper over a `T` that provides a primitive history mechanism by use of a stack of `T`. It
/// can be pushed to or popped from to save the current value or pop out a previously saved value
/// in LIFO (stack) order.
///
/// This is useful when you want to be able to make changes in a way where you can undo a change,
/// and then reapply it later, but do not wish to write a complex incremental structure that could
/// track changes like that. This type provides a generic (read: you can use it on anything)
/// interface to achieve that effect, even if it may use more memory than a more targeted approach.
#[derive(Clone, Default, Debug)]
pub struct HistoryStack<T> {
    /// The history stack, this starts out empty and should only be modified via pushing and popping
    stack: Vec<T>,
    /// The current value, since `HistoryStack<T>` acts like a T, this is always initialized to
    /// some value
    current: T,
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

    /// Gets an immutable reference to the current value, an explicit version of `Deref`
    pub fn get(&self) -> &T {
        self
    }

    /// Gets a mutable reference to the current value, an explicit version of `DerefMut`
    pub fn get_mut(&mut self) -> &mut T {
        self
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

/// A structure which allows you to undo and redo changes based on saved states of `T`
#[derive(Clone, Debug)]
pub struct UndoStack<T> {
    history: Vec<T>,
    current: usize,
}

impl<T> UndoStack<T> {
    pub fn new(start: T) -> Self {
        Self {
            history: alloc::vec![start],
            current: 0,
        }
    }

    /// Saves the current T to history and invalidates any data that may be used to redo
    /// This will [`Drop`] any T that exist later in history than the current edit point.
    ///
    /// Returns a reference to the new current value
    pub fn save(&mut self) -> &mut T
    where
        T: Clone,
    {
        // we have hit undo if these values do not match up, so we must invalidate the redo stack
        // it is safe to do current+1 because current is <history.len() which is stored
        // as usize aswell
        if self.current + 1 != self.history.len() {
            // see above for +1 safety
            self.history.truncate(self.current + 1);
        }

        // safe to unwrap here because history is always nonempty, however after this point it may
        // be empty so we cannot assume this until we push again
        let val = self.history.pop().unwrap();

        // history is nonempty again
        self.history.push(val.clone());
        self.history.push(val);

        // we popped once and pushed twice, see above for +1 safety
        self.current += 1;

        &mut self.history[self.current]
    }

    /// If there is a previous state in the history stack, backtrack to that and return Ok(&mut T)
    /// to the new current value, otherwise return Err(&mut T) to the unchanged current value
    pub fn undo(&mut self) -> Result<&mut T, &mut T> {
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

    /// If there is a future state in the history stack that we have undone from, redo to that
    /// position and return Ok(&mut T) of the new current value after advancing, else return
    /// Err(&mut T) of the current unchanged value.
    pub fn redo(&mut self) -> Result<&mut T, &mut T> {
        if self.current + 1 != self.history.len() {
            Err(&mut self.history[self.current])
        } else {
            self.current += 1;

            Ok(&mut self.history[self.current])
        }
    }

    pub fn get(&self) -> &T {
        &self.history[self.current]
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.history[self.current]
    }
}

impl<T> ops::Deref for UndoStack<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> ops::DerefMut for UndoStack<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: PartialEq> PartialEq<T> for UndoStack<T> {
    fn eq(&self, other: &T) -> bool {
        self.get() == other
    }
}

impl<T: PartialEq> PartialEq for UndoStack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for UndoStack<T> {}

impl<T: PartialOrd> PartialOrd for UndoStack<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T: PartialOrd> PartialOrd<T> for UndoStack<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl<T: Ord> Ord for UndoStack<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<T: hash::Hash> hash::Hash for UndoStack<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}
