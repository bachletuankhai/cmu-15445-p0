use std::sync::Arc;

pub struct SkipList<K: Ord> {
    header: Arc<Node<K>>,
}

struct Node<K: Ord> {
    height: usize,
    key: K,
    links: Vec<Arc<Node<K>>>,
}

impl<K: Ord> Node<K> {
    fn new(key: K, height: usize) -> Self {
        Node { key, height, links: Vec::new() }
    }

    fn height(&self) -> usize {
        self.height
    }

    fn next(&self, level: usize) -> Option<Arc<Node<K>>> {
        
    }
}