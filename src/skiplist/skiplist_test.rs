use std::{sync::Barrier, thread};
use std::sync::Arc;

use super::*;

trait Check {
    fn check_integrity(&self, keys: &Vec<i32>, heights: &Vec<usize>);
}

impl<const MAX_HEIGHT: usize, const SEED: u32> Check for SkipList<i32, MAX_HEIGHT, SEED> {
    fn check_integrity(&self, keys: &Vec<i32>, heights: &Vec<usize>) {
        assert_eq!(self.size(), keys.len());

        let mut pos = 0;
        let mut curr = self.header.clone();
        loop {
            let next = curr.read().map(|n| n.next(0)).unwrap();

            match next {
                Some(node) => {
                    curr = node;
                    let node = curr.read().unwrap();
                    let height = node.height();
                    assert_eq!(height, heights[pos]);
                    assert_eq!(node.key, Some(keys[pos]));

                    // Check link at each node level
                    for level in 0..height {
                        for key in &keys[(pos + 1)..] {
                            if height > level {
                                assert_eq!(
                                    node.next(level)
                                        .and_then(|n| n.read().map(|n| n.key).unwrap()),
                                    Some(*key)
                                );
                                break;
                            }
                        }
                    }
                    pos += 1;
                }
                None => break,
            }
        }

        assert_eq!(pos, keys.len());
    }
}

#[test]
fn integrity_check_test() {
    let mut list = SkipList::<i32>::new();

    let mut keys = vec![12, 16, 2, 6, 15, 8, 13, 1, 11, 14, 0, 4, 19, 10, 9, 5, 7, 3, 17, 18];
    let heights = vec![2, 1, 1, 1, 2, 1, 1, 1, 2, 1, 2, 1, 3, 1, 1, 2, 1, 1, 2, 3];

    for key in &keys {
        list.insert(*key);
    }

    keys.sort();
    println!("{list}");
    list.check_integrity(&keys, &heights);
}

#[test]
fn insert_contain_test_1() {
    let mut list = SkipList::<i32>::new();

    assert_eq!(list.size(), 0);
    assert!(list.empty());

    for i in 0..10 {
        assert!(list.insert(i));
    }

    for i in 0..10 {
        assert!(list.contains(&i));
    }

    for i in 10..20 {
        assert!(!list.contains(&i));
    }

    for i in 0..10 {
        assert!(!list.insert(i));
    }

    assert_eq!(list.size(), 10);

    for i in 10..20 {
        assert!(list.insert(i));
    }

    assert_eq!(list.size(), 20);

    assert!(!list.empty());

    list.clear();

    assert_eq!(list.size(), 0);
    assert!(list.empty());

    for i in 0..30 {
        assert!(!list.contains(&i));
    }
}

#[test]
fn insert_contain_test_2() {
    let mut list = SkipList::<i32>::new();

    assert_eq!(list.size(), 0);
    assert!(list.empty());

    assert!(list.insert(1));
    assert_eq!(list.size(), 1);

    assert!(list.insert(2));
    assert_eq!(list.size(), 2);

    assert!(list.contains(1));
    assert!(list.contains(2));

    assert!(!list.contains(3));
}

#[test]
fn insert_and_erase() {
    let mut list = SkipList::<i32>::new();

    for i in 0..5 {
        assert!(list.insert(i));
    }

    assert_eq!(list.size(), 5);

    for i in 0..5 {
        assert!(list.contains(i));
        assert!(list.erase(i));

        assert_eq!(list.size(), (5 - i - 1).try_into().unwrap());
    }

    assert!(list.empty());
}

#[test]
fn erase_non_existing_test() {
    let mut list = SkipList::<i32>::new();

    for i in 0..5 {
        assert!(list.insert(i));
    }

    assert!(!list.erase(10));
    assert_eq!(list.size(), 5);
}

#[test]
fn concurrent_insert_test() {
    let mut list = SkipList::<i32>::new();

    let num_threads = 10;
    let num_insertions_per_thread = 100;

    let successful_insertion = Arc::new(0);

    let list = Arc::new(list);
    let barrier = Arc::new(Barrier::new(num_threads));
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let list = Arc::clone(&list);
            let barrier = Arc::clone(&barrier);
            handles.push(s.spawn(move || {
                barrier.wait();
                for j in 0..num_insertions_per_thread {
                    let key = i * num_insertions_per_thread + j;
                    list.insert(key);
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
