use std::{cell::RefCell, sync::Barrier, thread};

use super::*;

#[test]
fn test1() {
    let mut list = SkipList::<i32>::new();

    // println!("{list}");
    assert_eq!(list.empty(), true);
    debug_assert!(list.empty());

    for i in 0..10 {
        list.insert(i);
    }
    debug_assert_eq!(list.size(), 10);
}

#[test]
fn test2() {
    let n = 100;
    let barrier = Barrier::new(n);

    let list = Arc::new(RwLock::new(SkipList::<usize>::new()));
    thread::scope(|s| {
        let barrier = &barrier;
        for i in 0..n {
            let list = list.clone();
            s.spawn(move || {
                barrier.wait();
                let res = list
                    .write()
                    .map(|mut l| l.insert(i.clone()))
                    .expect("Should insert");
                assert!(res);

                assert!(list.read().map(|l| l.contains(&i)).unwrap());
            });
        }
    });
    debug_assert!(list.read().map(|l| l.size()).unwrap() == 100);
}
