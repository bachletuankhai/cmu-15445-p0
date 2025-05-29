use std::sync::Barrier;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::scope;

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
                    assert_eq!(node.compare_key(keys[pos]), Some(Ordering::Equal));

                    // Check link at each node level
                    for level in 0..height {
                        for key in &keys[(pos + 1)..] {
                            if height > level {
                                assert_eq!(
                                    node.next(level)
                                        .and_then(|n| n.read().map(|n| n.compare_key(key)).unwrap()),
                                    Some(Ordering::Equal)
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
    let list = SkipList::<i32>::new();

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
    let list = SkipList::<i32>::new();

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
    let list = SkipList::<i32>::new();

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
    let list = SkipList::<i32>::new();

    for i in 0..5 {
        assert!(list.insert(i));
    }

    assert!(!list.erase(10));
    assert_eq!(list.size(), 5);
}

#[test]
fn concurrent_insert_test() {
    let list = SkipList::<i32>::new();

    let num_threads = 10;
    let num_insertions_per_thread = 100;

    let successful_insertion = Arc::new(Mutex::new(0));

    let list = Arc::new(list);
    let barrier = Arc::new(Barrier::new(num_threads));
    std::thread::scope(|s| {
        for i in 0..num_threads {
            let list = Arc::clone(&list);
            let barrier = Arc::clone(&barrier);
            let successful_insertion = Arc::clone(&successful_insertion);
            s.spawn(move || {
                barrier.wait();
                let k = i * num_insertions_per_thread;
                for j in 0..num_insertions_per_thread {
                    let key = k + j;
                    if list.insert(key.try_into().unwrap()) {
                        let mut successful_insertion = successful_insertion.lock().unwrap();
                        *successful_insertion += 1;
                    }
                }
            });
        }
    });

    assert_eq!(successful_insertion.lock().unwrap().to_owned(), num_threads * num_insertions_per_thread);

    for i in 0..(num_threads * num_insertions_per_thread) {
        assert!(list.contains::<i32>(i.try_into().unwrap()));
    }
}

#[test]
fn concurrent_erase_test() {
    let list = Arc::new(SkipList::<i32>::new());

    for i in 0..100 {
        list.insert(i);
    }

    let num_threads = 10;
    let num_erasures_per_thread = 10;
    let successful_erasures = Arc::new(Mutex::new(0));
    let barrier = Arc::new(Barrier::new(num_threads));
    scope(|s| {
        for i in 0..num_threads {
            let list = list.clone();
            let barrier = barrier.clone();
            let k = i * num_erasures_per_thread;
            let successful_erasures = successful_erasures.clone();
            s.spawn(move || {
                barrier.wait();
                for j in 0..num_erasures_per_thread {
                    if list.erase::<i32>((k + j).try_into().unwrap()) {
                        let mut success_erase = successful_erasures.lock().unwrap();
                        *success_erase += 1;
                    }
                }

            });
        }
    });

    assert!(successful_erasures.lock().map(|n| *n == num_erasures_per_thread * num_threads).unwrap());

    for i in 0..100 {
        if i < num_threads * num_erasures_per_thread {
            assert!(!list.contains::<i32>(i.try_into().unwrap()));
        } else {
            assert!(list.contains::<i32>(i.try_into().unwrap()));
        }
    }
}

#[test]
fn concurrent_insert_and_erase_test() {
    let list = Arc::new(SkipList::<i32>::new());

    for i in 0..100 {
        list.insert(i);
    }

    const NUM_THREADS: i32 = 10;
    const NUM_OPERATIONS_PER_THREAD: i32 = 10;

    let succ_inserts = Arc::new(Mutex::new(0));
    let succ_erases = Arc::new(Mutex::new(0));
    let barrier = Arc::new(Barrier::new(NUM_THREADS as usize));

    scope(|s| {
        for i in 0..NUM_THREADS {
            let barrier = Arc::clone(&barrier);
            let list = Arc::clone(&list);
            let succ_inserts = Arc::clone(&succ_inserts);
            let succ_erases = Arc::clone(&succ_erases);
            let start = i * NUM_OPERATIONS_PER_THREAD;
            s.spawn(move || {
                barrier.wait();
                for j in start..(NUM_OPERATIONS_PER_THREAD + start) {
                    if !list.contains(j) {
                        list.insert(j);
                    }

                    if list.insert(j + 100) {
                        let mut inserts = succ_inserts.lock().unwrap();
                        *inserts += 1;
                    }

                    if list.erase(j) {
                        let mut erases = succ_erases.lock().unwrap();
                        *erases += 1;
                    }
                }
            });
        }
    });

    assert!(succ_inserts.lock().map(|n| *n == NUM_THREADS * NUM_OPERATIONS_PER_THREAD).unwrap());
    assert!(succ_erases.lock().map(|n| *n == NUM_THREADS * NUM_OPERATIONS_PER_THREAD).unwrap());

    for i in 100..(100 + NUM_THREADS * NUM_OPERATIONS_PER_THREAD) {
        assert!(list.contains(i));
    }

    for i in 0..(NUM_THREADS * NUM_OPERATIONS_PER_THREAD) {
        assert!(!list.contains(i));
    }
}

#[test]
fn concurrent_read_test() {
    const NUM_THREADS: usize = 8;

    const TOTAL_NUM_ELEMENTS: usize = NUM_THREADS * 100000;

    let list = Arc::new(SkipList::<i32>::new());
    for i in 0..TOTAL_NUM_ELEMENTS {
        list.insert(i as i32);
    }
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    scope(|s| {
        for _ in 0..NUM_THREADS {
            let list = Arc::clone(&list);
            let barrier = Arc::clone(&barrier);
            s.spawn(move || {
                barrier.wait();
                for i in 0..TOTAL_NUM_ELEMENTS {
                    list.contains(&(i as i32));
                }
            });
        }
    });
}
