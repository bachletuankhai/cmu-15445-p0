use rand_core::RngCore;
use std::{
    array,
    fmt::Display,
    sync::{Arc, RwLock},
};

use mt19937::MT19937;

pub struct SkipList<K: Ord, const MAX_HEIGHT: usize = 14, const SEED: u32 = 15445> {
    header: Arc<RwLock<Node<K>>>,
    height: usize,
    size: usize,
    rng: MT19937,
}

impl<K: Ord, const MAX_HEIGHT: usize, const SEED: u32> SkipList<K, MAX_HEIGHT, SEED> {
    pub fn new() -> Self {
        let header = Arc::new(RwLock::new(Node::new_header(MAX_HEIGHT)));
        let rng = MT19937::new_with_slice_seed(&[SEED]);
        SkipList {
            header,
            height: 1,
            size: 0,
            rng,
        }
    }

    pub fn empty(&self) -> bool {
        self.height == 1
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn insert(&mut self, key: K) -> bool {
        let mut cur = self.header.clone();
        let mut found = false;
        let update: [Arc<RwLock<Node<K>>>; MAX_HEIGHT] = array::from_fn(|i| {
            let level = MAX_HEIGHT - i;
            loop {
                let next = {
                    let cur_read_lock = cur.read().unwrap();
                    match cur_read_lock.next(level) {
                        Some(arc) => arc,
                        None => break,
                    }
                };
                let next_node_read_lock = next.read().unwrap();
                let next_key = next_node_read_lock.key.as_ref();
                match next_key {
                    Some(k) if *k < key => cur = next.clone(),
                    Some(k) if *k == key => {
                        // Key already exists
                        found = true;
                        break;
                    }
                    _ => break,
                }
            }
            cur.clone()
        });

        if found {
            return false;
        }

        let new_height = self.random_height();
        if new_height > self.height {
            self.height = new_height;
        }
        let new_node = Arc::new(RwLock::new(Node::new(key, new_height)));

        for i in 0..new_height {
            let node_to_update = update[MAX_HEIGHT - i].clone();
            {
                let node_to_update_read_lock = node_to_update.read().unwrap();
                if let Some(arc) = node_to_update_read_lock.next(i) {
                    let mut new_node_write_lock = new_node.write().unwrap();
                    new_node_write_lock.set_next(i, arc);
                }
            }
            {
                let mut node_to_update_write_lock = node_to_update.write().unwrap();
                node_to_update_write_lock.set_next(i, new_node.clone());
            }
        }

        true
    }

    pub fn erase(&mut self, key: &K) -> bool {
        let mut cur = self.header.clone();
        let mut found = false;
        let update: [Arc<RwLock<Node<K>>>; MAX_HEIGHT] = array::from_fn(|i| {
            let level = MAX_HEIGHT - i;
            loop {
                let next = {
                    let cur_read_lock = cur.read().unwrap();
                    match cur_read_lock.next(level) {
                        Some(arc) => arc,
                        None => break,
                    }
                };
                let next_node_read_lock = next.read().unwrap();
                let next_key = next_node_read_lock.key.as_ref();
                match next_key {
                    Some(k) if *k < *key => cur = next.clone(),
                    Some(k) if *k == *key => {
                        // Key already exists
                        found = true;
                        break;
                    }
                    _ => break,
                }
            }
            cur.clone()
        });

        if !found {
            return false;
        }

        let node_to_delete = cur.read().map(|node| node.next(1)).unwrap().expect("Node to erase should be found here");

        for i in (0..MAX_HEIGHT).rev() {
            let node_to_update = update[MAX_HEIGHT - i].clone();
            let next = node_to_update.read().map(|node| node.next(i)).unwrap();
            match next {
                Some(arc) if Arc::ptr_eq(&arc, &node_to_delete) => {
                    let delete_next_i = node_to_delete.read().map(|node| node.next(i)).unwrap();

                    let mut node_to_update_write_lock = node_to_update.write().unwrap();
                    match delete_next_i {
                        Some(arc) => {
                            node_to_update_write_lock.set_next(i, arc);
                        }
                        None => {
                            node_to_update_write_lock.remove_next(i);
                        }
                    }
                }
                _ => continue,
            }
        }
        true
    }

    pub fn contains(&self, key: &K) -> bool {
        return self.find(key).is_some();
    }

    fn find(&self, key: &K) -> Option<Arc<RwLock<Node<K>>>> {
        let mut cur = self.header.clone();
        for level in (0..self.height).rev() {
            loop {
                let next = {
                    let cur_read_lock = cur.read().unwrap();
                    match cur_read_lock.next(level) {
                        Some(arc) => arc,
                        None => break,
                    }
                };
                let next_read_lock = next.read().unwrap();
                match next_read_lock.key.as_ref() {
                    Some(k) if *k < *key => cur = next.clone(),
                    Some(k) if *k == *key => return Some(next.clone()),
                    _ => break,
                }
            }
        }
        None
    }

    fn random_height(&mut self) -> usize {
        let mut height: usize = 1;
        while height < MAX_HEIGHT && self.rng.next_u32() % 4 == 0 {
            height += 1;
        }
        height
    }

    pub fn print(&self) {}
}

impl<K: Ord, const MAX_HEIGHT: usize, const SEED: u32> Display for SkipList<K, MAX_HEIGHT, SEED> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

struct Node<K: Ord> {
    key: Option<K>,
    links: Vec<Arc<RwLock<Node<K>>>>,
}

impl<K: Ord> Node<K> {
    fn new_header(height: usize) -> Self {
        Node {
            key: None,
            links: Vec::with_capacity(height),
        }
    }

    fn new(key: K, height: usize) -> Self {
        Node {
            key: Some(key),
            links: Vec::with_capacity(height),
        }
    }

    fn height(&self) -> usize {
        self.links.len()
    }

    fn next(&self, level: usize) -> Option<Arc<RwLock<Node<K>>>> {
        self.links.get(level).cloned()
    }

    fn set_next(&mut self, level: usize, next: Arc<RwLock<Node<K>>>) {
        assert!(level < self.links.capacity());

        if level < self.links.len() {
            self.links[level] = next;
            return;
        }

        self.links.resize(level + 1, next);
    }

    fn remove_next(&mut self, level: usize) {
        self.links.truncate(level);
    }
}
