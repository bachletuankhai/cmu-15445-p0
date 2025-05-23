use std::sync::Arc;

pub struct SkipList<K: Ord, const MAX_HEIGHT: usize> {
    header: Arc<Node<K>>,
}

struct Node<K: Ord> {
    key: K,
    links: Vec<Arc<Node<K>>>,
}

impl<K: Ord> Node<K> {
    fn new(key: K, height: usize) -> Self {
        Node { key, links: Vec::with_capacity(height) }
    }

    fn height(&self) -> usize {
        self.links.len()
    }

    fn next(&self, level: usize) -> Option<Arc<Node<K>>> {
        
    }
}