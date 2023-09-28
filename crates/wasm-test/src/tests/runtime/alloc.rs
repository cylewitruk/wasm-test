use crate::runtime::alloc::{WasmAllocator2, WrappedPtr};

#[test]
fn test_alloc_with_size_power_of_two() {
    WasmAllocator2::new(512);
}

#[test]
#[should_panic]
fn test_alloc_with_bad_size_fails() {
    WasmAllocator2::new(123);
}

#[test]
fn test_simple_allocations_round_up_to_next_block_size() {
    let alloc = WasmAllocator2::new(256);

    assert_eq!(Some(8), alloc.allocation_size(0));
    assert_eq!(Some(8), alloc.allocation_size(1));
    assert_eq!(Some(16), alloc.allocation_size(16));
    assert_eq!(Some(32), alloc.allocation_size(17));
    assert_eq!(Some(32), alloc.allocation_size(32));
    assert_eq!(Some(64), alloc.allocation_size(33));
    assert_eq!(Some(256), alloc.allocation_size(256));
}

#[test]
fn test_block_orders() {
    let alloc = WasmAllocator2::new(256);

    assert_eq!(Some(0), alloc.allocation_order(0));
    assert_eq!(Some(0), alloc.allocation_order(1));
    assert_eq!(Some(1), alloc.allocation_order(16));
    assert_eq!(Some(2), alloc.allocation_order(32));
    assert_eq!(Some(3), alloc.allocation_order(64));
    assert_eq!(Some(4), alloc.allocation_order(128));
    assert_eq!(Some(5), alloc.allocation_order(256));
    assert_eq!(None, alloc.allocation_order(512));
}

#[test]
fn test_order_size() {
    let mut alloc = WasmAllocator2::new(256);

    assert_eq!(8, alloc.order_size(0));
    assert_eq!(16, alloc.order_size(1));
    assert_eq!(32, alloc.order_size(2));
    assert_eq!(64, alloc.order_size(3));
    assert_eq!(128, alloc.order_size(4));
    assert_eq!(256, alloc.order_size(5));

    alloc = WasmAllocator2::new(1024);

    assert_eq!(8, alloc.order_size(0));
    assert_eq!(16, alloc.order_size(1));
    assert_eq!(32, alloc.order_size(2));
    assert_eq!(64, alloc.order_size(3));
    assert_eq!(128, alloc.order_size(4));
    assert_eq!(256, alloc.order_size(5));
    assert_eq!(512, alloc.order_size(6));
    assert_eq!(1024, alloc.order_size(7));
}

#[test]
fn test_free_list_remove() {
    let mut alloc = WasmAllocator2::new(256);

    let block1 = WrappedPtr::new(0, 1, 1);
    let block2 = WrappedPtr::new(1, 2, 1);
    let block3 = WrappedPtr::new(2, 3, 1);

    alloc.free_list_push(3, block1);
    alloc.free_list_push(3, block2);
    alloc.free_list_push(3, block3);

    assert_eq!(false, alloc.free_list_remove(2, block1.id));
    assert_eq!(false, alloc.free_list_remove(4, block3.id));
    assert_eq!(true, alloc.free_list_remove(3, block2.id));
}

#[test]
fn test_split_free_block() {
    let mut alloc = WasmAllocator2::new(256);
    let mut ptr = WrappedPtr::new(0, 0, 256);

    alloc.split_free_block(&mut ptr, 5, 1);
}

#[test]
fn test_allocate() {
    let mut alloc = WasmAllocator2::new(256);

    let mut block = alloc.allocate(8);
    println!("{:?}", block);

    let buddy = alloc.find_buddy(0, block).expect("Failed to find buddy");
    assert_eq!(5, buddy.id);
}

#[test]
fn test_find_buddy() {
    let mut alloc = WasmAllocator2::new(256);

    let mut block = alloc.allocate(8);
    println!("{:?}", block);

    let buddy = alloc.find_buddy(0, block).expect("Failed to find buddy");
    println!("buddy: {:?}", buddy);
    alloc.deallocate(block);
}
