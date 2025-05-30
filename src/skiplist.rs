use rand_core::RngCore;
use std::{
    array,
    borrow::Borrow,
    cmp::Ordering,
    fmt::{Debug, Display},
    sync::{Arc, RwLock},
};

use mt19937::MT19937;

pub struct SkipList<K: Ord, const MAX_HEIGHT: usize = 14, const SEED: u32 = 15445> {
    inner: Arc<RwLock<SkipListInner<K, MAX_HEIGHT, SEED>>>,
}

impl<K: Ord + Debug, const MAX_HEIGHT: usize, const SEED: u32> SkipList<K, MAX_HEIGHT, SEED> {
    pub fn new() -> Self {
        SkipList {
            inner: Arc::new(RwLock::new(SkipListInner::new())),
        }
    }

    pub fn empty(&self) -> bool {
        self.inner.read().unwrap().empty()
    }

    pub fn size(&self) -> usize {
        self.inner.read().unwrap().size()
    }

    pub fn insert(&self, key: K) -> bool {
        let mut inner = self.inner.write().unwrap();
        inner.insert(key)
    }

    pub fn erase<Q>(&self, key: Q) -> bool
    where
        Q: Borrow<K>,
    {
        let mut inner = self.inner.write().unwrap();
        inner.erase(key)
    }

    pub fn contains<Key>(&self, key: Key) -> bool
    where
        Key: Borrow<K>,
    {
        self.inner.read().unwrap().contains(key)
    }

    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.clear();
    }
}

impl<K: Ord + Debug, const MAX_HEIGHT: usize, const SEED: u32> Display
    for SkipList<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner.read().unwrap(), f)
    }
}

impl<K: Ord + Debug, const MAX_HEIGHT: usize, const SEED: u32> Debug
    for SkipList<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner.read().unwrap(), f)
    }
}

struct SkipListInner<K: Ord, const MAX_HEIGHT: usize = 14, const SEED: u32 = 15445> {
    header: Arc<RwLock<Node<K>>>,
    height: usize,
    size: usize,
    rng: Arc<RwLock<MT19937>>,
}

impl<K: Ord + Debug, const MAX_HEIGHT: usize, const SEED: u32> SkipListInner<K, MAX_HEIGHT, SEED> {
    pub fn new() -> Self {
        let header = Arc::new(RwLock::new(Node::new_header(MAX_HEIGHT)));
        let rng = MT19937::new_with_slice_seed(&[SEED]);
        SkipListInner {
            header,
            height: 1,
            size: 0,
            rng: Arc::new(RwLock::new(rng)),
        }
    }

    pub fn empty(&self) -> bool {
        self.size == 0
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn insert(&mut self, key: K) -> bool {
        let (update, _cur, found) = self.trace(&key);

        if found {
            return false;
        }

        let new_height = self.random_height();
        if new_height > self.height {
            self.height = new_height;
        }
        let new_node = Arc::new(RwLock::new(Node::new(key, new_height)));
        for i in 0..new_height {
            let node_to_update = update[MAX_HEIGHT - i - 1].clone();
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
        self.size += 1;

        true
    }

    fn trace<Q>(
        &self,
        key: Q,
    ) -> (
        [Arc<RwLock<Node<K>>>; MAX_HEIGHT],
        Arc<RwLock<Node<K>>>,
        bool,
    )
    where
        Q: Borrow<K>,
    {
        let mut cur = self.header.clone();
        let mut found = false;
        let key = key.borrow();
        let update: [Arc<RwLock<Node<K>>>; MAX_HEIGHT] = array::from_fn(|i| {
            let level = MAX_HEIGHT - i - 1;
            loop {
                let next = {
                    let cur_read_lock = cur.read().unwrap();
                    match cur_read_lock.next(level) {
                        Some(arc) => arc,
                        None => break,
                    }
                };
                let next_key_cmp = next.read().map(|node| node.compare_key(key)).unwrap();
                match next_key_cmp {
                    Some(Ordering::Less) => cur = next.clone(),
                    Some(Ordering::Equal) => {
                        // Key already exists
                        found = true;
                        break;
                    }
                    _ => break,
                }
            }
            cur.clone()
        });
        return (update, cur, found);
    }

    pub fn erase<Q>(&mut self, key: Q) -> bool
    where
        Q: Borrow<K>,
    {
        let (update, cur, found) = self.trace(key);

        if !found {
            return false;
        }

        let node_to_delete = cur
            .read()
            .map(|node| node.next(0))
            .unwrap()
            .expect("Node to erase should be found here");

        for i in (0..MAX_HEIGHT).rev() {
            let node_to_update = update[MAX_HEIGHT - i - 1].clone();
            let next = node_to_update.read().map(|node| node.next(i)).unwrap();
            match next {
                Some(arc) if Arc::ptr_eq(&arc, &node_to_delete) => {
                    let delete_next_i = node_to_delete.read().map(|node| node.next(i)).unwrap();

                    let mut node_to_update_write_lock = node_to_update.write().unwrap();
                    match delete_next_i {
                        Some(arc) => {
                            node_to_update_write_lock.set_next(i, arc);
                        }
                        None => {} // Should not happen, None only if node_to_delete is nil
                    }
                }
                _ => continue,
            }
        }
        self.size -= 1;

        loop {
            match self.header.read().unwrap().next(self.height - 1) {
                Some(arc) if !arc.read().unwrap().is_nil() && self.height > 1 => {
                    self.height -= 1;
                }
                _ => break,
            }
        }
        true
    }

    pub fn contains<Key>(&self, key: Key) -> bool
    where
        Key: Borrow<K>,
    {
        return self.find(key.borrow()).is_some();
    }

    fn find<Key: Borrow<K>>(&self, key: Key) -> Option<Arc<RwLock<Node<K>>>> {
        let mut cur = self.header.clone();
        let height = self.height;
        for level in (0..height).rev() {
            loop {
                let next = {
                    let cur_read_lock = cur.read().unwrap();
                    match cur_read_lock.next(level) {
                        Some(arc) => arc,
                        None => break,
                    }
                };
                let next_read_lock = next.read().unwrap();
                match next_read_lock.compare_key(key.borrow()) {
                    Some(Ordering::Less) => cur = next.clone(),
                    Some(Ordering::Equal) => return Some(next.clone()),
                    _ => break,
                }
            }
        }
        None
    }

    fn random_height(&self) -> usize {
        let mut height: usize = 1;
        let mut rng = self.rng.write().unwrap();
        while height < MAX_HEIGHT && rng.next_u32() % 4 == 0 {
            height += 1;
        }
        height
    }

    pub fn clear(&mut self) {
        {
            let mut header = self.header.write().unwrap();
            header.clear();
        }
        self.height = 1;
        self.size = 0;
    }
}

impl<K: Ord + std::fmt::Debug, const MAX_HEIGHT: usize, const SEED: u32> Display
    for SkipListInner<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Err(e) = f.write_fmt(format_args!(
            "Height: {} | Size: {}\n",
            self.height, self.size
        )) {
            return Err(e);
        };
        let mut cur = self.header.clone();
        f.write_fmt(format_args!("{}, ", cur.read().unwrap()))?;
        loop {
            let next = cur.read().map(|node| node.next(0)).unwrap();
            match next {
                Some(arc) => {
                    let read_lock = arc.read().unwrap();
                    if let Err(e) = f.write_fmt(format_args!("{}, ", read_lock,)) {
                        return Err(e);
                    }
                    cur = arc.clone();
                }
                None => break,
            }
        }
        Ok(())
    }
}

impl<K: Ord + std::fmt::Debug, const MAX_HEIGHT: usize, const SEED: u32> Debug
    for SkipListInner<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

enum Node<K: Ord> {
    Header {
        height: usize,
        links: Vec<Arc<RwLock<Node<K>>>>,
    },
    Inner {
        height: usize,
        key: K,
        links: Vec<Arc<RwLock<Node<K>>>>,
    },
    Nil,
}

impl<K: Ord + Debug> Display for Node<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Header { height, links: _ } => f.write_fmt(format_args!("[H {}]", height)),
            Node::Inner { key, height, links: _ } => {
                f.write_fmt(format_args!("[{:?} {}]", key, height))
            }
            Node::Nil => f.write_str("NIL"),
        }
    }
}

impl<K: Ord + Debug> Debug for Node<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

impl<K: Ord + Debug> Node<K> {
    fn new_header(height: usize) -> Self {
        let mut links = Vec::with_capacity(height);
        let nil = Arc::new(RwLock::new(Self::Nil));
        for _ in 0..height {
            links.push(nil.clone());
        }
        Self::Header { height: 1, links: links }
    }

    fn new(key: K, height: usize) -> Self {
        Self::Inner {
            height: 0,
            key: key,
            links: Vec::with_capacity(height),
        }
    }

    fn height(&self) -> usize {
        match self {
            Node::Header { height, .. } => *height,
            Node::Inner { links, .. } => links.len(),
            Node::Nil => 0,
        }
    }

    fn next(&self, level: usize) -> Option<Arc<RwLock<Node<K>>>> {
        match self {
            Node::Nil => None,
            Node::Header { links, .. } => links.get(level).cloned(),
            Node::Inner { links, .. } => links.get(level).cloned(),
        }
    }

    fn set_next(&mut self, level: usize, next: Arc<RwLock<Node<K>>>) {
        match self {
            Node::Header { height, links } => {
                if *height < level + 1 {
                    *height = level + 1;
                }
                links[level] = next;
            }
            Node::Inner { height, links, .. } => {
                if *height < level + 1 {
                    *height = level + 1;
                }
                if level < links.len() {
                    links[level] = next;
                } else {
                    links.resize(level + 1, next);
                }
            }
            Node::Nil => {}
        }
    }

    fn compare_key(&self, key: impl Borrow<K>) -> Option<Ordering> {
        match self {
            Node::Header { .. } => None,
            Node::Inner {
                key: node_key,
                ..
            } => Some(node_key.cmp(key.borrow())),
            Node::Nil => None,
        }
    }

    fn clear(&mut self) {
        match self {
            Node::Header { links, height, .. } => {
                links.fill(Arc::new(RwLock::new(Node::Nil)));
                *height = 1;
            }
            Node::Inner { links, height, .. } => {
                links.clear();
                *height = 0;
            },
            Node::Nil => {}
        }
    }

    fn is_nil(&self) -> bool {
        matches!(self, Node::Nil)
    }
}

#[cfg(test)]
mod skiplist_test;
