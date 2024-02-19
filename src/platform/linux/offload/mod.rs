//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

use crate::Error;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;

mod coalesce;
mod packet;
mod tcp;
mod udp;
mod virtio;

const IDEAL_BATCH_SIZE: usize = 128;

pub(crate) struct GroTable<K, T> {
    items_by_flow: HashMap<K, Vec<T>>,
}

impl<K: Hash + Eq, T> GroTable<K, T> {
    pub(crate) fn new() -> Self {
        Self {
            items_by_flow: HashMap::with_capacity(IDEAL_BATCH_SIZE),
        }
    }

    /// Get or insert a new item into the table.
    pub(crate) fn get_or_insert(&mut self, key: K, item: T) -> &mut Vec<T> {
        match self.items_by_flow.entry(key) {
            Entry::Occupied(mut entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut items = Vec::with_capacity(IDEAL_BATCH_SIZE);
                items.push(item);
                entry.insert(items)
            }
        }
    }

    /// Updates the item for the given flow at the given index.
    pub(crate) fn update_at(&mut self, key: K, index: usize, item: T) -> crate::Result<()> {
        *self
            .items_by_flow
            .get_mut(&key)
            .ok_or(Error::OffloadFlowNotFound)?
            .get_mut(index)
            .ok_or(Error::OffloadFlowNotFound)? = item;

        Ok(())
    }

    /// Deletes the item for the given flow at the given index.
    pub(crate) fn delete_at(&mut self, key: K, index: usize) -> crate::Result<()> {
        self.items_by_flow
            .get_mut(&key)
            .map(|items| items.remove(index))
            .ok_or(Error::OffloadFlowNotFound)?;

        Ok(())
    }

    /// Clears the table.
    pub(crate) fn clear(&mut self) {
        self.items_by_flow.clear();
    }
}
