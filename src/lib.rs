//! A crate implementing a HistoryStack that lets you push to store a state and pop to retrieve it
//! later

#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::alloc_instead_of_core, clippy::std_instead_of_alloc)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

extern crate alloc;

use core::{cmp, hash, ops};

use alloc::vec::Vec;

#[derive(Clone, Default, Debug)]
pub struct HistoryStack<T> {
    stack: Vec<T>,
    current: T,
}

impl<T> HistoryStack<T> {
    pub fn pop(&mut self) -> Option<T> {
        match self.stack.pop() {
            Some(last) => Some(core::mem::replace(&mut self.current, last)),
            None => None,
        }
    }

    pub fn push_value(&mut self, v: T) {
        self.stack.push(core::mem::replace(&mut self.current, v));
    }

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
        &self.current == &other.current
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
        self.current.partial_cmp(&other)
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
