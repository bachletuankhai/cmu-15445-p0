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