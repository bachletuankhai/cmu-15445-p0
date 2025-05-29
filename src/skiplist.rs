use rand_core::RngCore;
use std::{
    array, borrow::Borrow, cmp::Ordering, fmt::{Debug, Display}, sync::{Arc, RwLock}
};

use mt19937::MT19937;

pub struct SkipList<K: Ord, const MAX_HEIGHT: usize = 14, const SEED: u32 = 15445> {
    header: Arc<RwLock<Node<K>>>,
    height: RwLock<usize>,
    size: RwLock<usize>,
    rng: Arc<RwLock<MT19937>>,
}

impl<K: Ord + Debug, const MAX_HEIGHT: usize, const SEED: u32> SkipList<K, MAX_HEIGHT, SEED> {
    pub fn new() -> Self {
        let header = Arc::new(RwLock::new(Node::new_header(MAX_HEIGHT)));
        let rng = MT19937::new_with_slice_seed(&[SEED]);
        SkipList {
            header,
            height: RwLock::new(1),
            size: RwLock::new(0),
            rng: Arc::new(RwLock::new(rng)),
        }
    }

    pub fn empty(&self) -> bool {
        self.size.read().map(|h| *h == 0).unwrap()
    }

    pub fn size(&self) -> usize {
        self.size.read().map(|s| *s).unwrap()
    }

    pub fn insert(&self, key: K) -> bool {
        let (update, _cur, found) = self.trace(&key);

        if found {
            return false;
        }

        let new_height = self.random_height();
        let is_higher = self.height.read().map(|h| new_height > *h).unwrap();
        if is_higher {
            let mut height = self.height.write().unwrap();
            *height += 1;
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
        {
            let mut size = self.size.write().unwrap();
            *size += 1;
        }

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

    pub fn erase<Q>(&self, key: Q) -> bool
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
                        None => {
                            node_to_update_write_lock.remove_next(i);
                        }
                    }
                }
                _ => continue,
            }
        }
        {
            let mut size_write_lock = self.size.write().unwrap();
            *size_write_lock -= 1;
        }
        {
            let mut height_write_lock = self.height.write().unwrap();
            *height_write_lock = self.header.read().map(|n| n.height()).unwrap();
            println!("{}", *height_write_lock);
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
        let height = self.height.read().unwrap();
        for level in (0..*height).rev() {
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
        {
            let mut height = self.height.write().unwrap();
            *height = 1;
        }
        {
            let mut size = self.size.write().unwrap();
            *size = 0;
        }
    }
}

impl<K: Ord + std::fmt::Debug, const MAX_HEIGHT: usize, const SEED: u32> Display
    for SkipList<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Err(e) = f.write_fmt(format_args!(
            "Height: {} | Size: {}\n",
            self.height.read().unwrap(),
            self.size.read().unwrap()
        )) {
            return Err(e);
        };
        let mut cur = self.header.clone();
        loop {
            let next = cur.read().map(|node| node.next(0)).unwrap();
            match next {
                Some(arc) => {
                    let read_lock = arc.read().unwrap();
                    if let Err(e) = f.write_fmt(format_args!(
                        "{}\n",
                        read_lock,
                    )) {
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
    for SkipList<K, MAX_HEIGHT, SEED>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

enum Node<K: Ord> {
    Header {
        links: Vec<Arc<RwLock<Node<K>>>>,
    },
    Inner {
        key: K,
        links: Vec<Arc<RwLock<Node<K>>>>,
    },
    Nil
}

impl<K: Ord + Debug> Display for Node<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Header { links: _ } => f.write_fmt(format_args!("Header | Height: {}", self.height())),
            Node::Inner { key, links: _ } => f.write_fmt(format_args!("Key: {:?} | Height: {}", key, self.height())),
            Node::Nil => f.write_str("NIL"),
        }
    }
}

impl<K: Ord> Node<K> {
    fn new_header(height: usize) -> Self {
        let mut links = Vec::with_capacity(height);
        links.push(Arc::new(RwLock::new(Self::Nil)));
        Self::Header {
            links: links,
        }
    }

    fn new(key: K, height: usize) -> Self {
        Self::Inner {
            key: key,
            links: Vec::with_capacity(height),
        }
    }

    fn height(&self) -> usize {
        match self {
            Node::Header { links } => links.len(),
            Node::Inner { key: _, links } => links.len(),
            Node::Nil => 0,
        }
    }

    fn next(&self, level: usize) -> Option<Arc<RwLock<Node<K>>>> {
        match self {
            Node::Nil => None,
            Node::Header { links } => links.get(level).cloned(),
            Node::Inner { key: _, links } => links.get(level).cloned()
        }
    }

    fn set_next(&mut self, level: usize, next: Arc<RwLock<Node<K>>>) {
        let link_vec = match self {
            Node::Header { links } => links,
            Node::Inner { key: _, links } => links,
            Node::Nil => return,
        };

        assert!(level < link_vec.capacity());
        if level < link_vec.len() {
            link_vec[level] = next;
        } else {
            link_vec.resize(level + 1, next);
        }
    }

    fn remove_next(&mut self, level: usize) {
        let links = match self {
            Node::Header { links } => links,
            Node::Inner { key: _, links } => links,
            Node::Nil => return,
        };

        links.truncate(level);
    }

    fn compare_key(&self, key: impl Borrow<K>) -> Option<Ordering> {
        match self {
            Node::Header { links: _ } => None,
            Node::Inner { key: node_key, links: _ } => Some(node_key.cmp(key.borrow())),
            Node::Nil => None
        }
    }

    fn clear(&mut self) {
        match self {
            Node::Header { links } => links.clear(),
            Node::Inner { key: _, links } => links.clear(),
            Node::Nil => {},
        }
    }
}



#[cfg(test)]
mod skiplist_test;
