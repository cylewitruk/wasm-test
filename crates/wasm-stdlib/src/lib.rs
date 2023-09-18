#![no_std]
#![feature(alloc_error_handler)]

use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::cell::UnsafeCell;

#[panic_handler]
fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    core::arch::wasm32::unreachable()
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator::new();

#[no_mangle]
#[export_name = "add-int128"]
pub extern "C" fn add_int128(a_lo: i64, a_hi: i64, b_lo: i64, b_hi: i64) -> (i64, i64) {
    let a = ((a_lo as u64) as u128) | ((a_hi as u64) as u128) << 64;
    let b = ((b_lo as u64) as u128) | ((b_hi as u64) as u128) << 64;

    match a.checked_add(b) {
        Some(result) => {
            let result: i128 = result.try_into()
                .unwrap_or_else(|_| core::arch::wasm32::unreachable());
            (
                (result & 0xFFFFFFFFFFFFFFFF) as i64,
                ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
            )
        },
        _ => core::arch::wasm32::unreachable()
    }
}

#[no_mangle]
#[export_name = "add-unt128"]
pub extern "C" fn add_uint128(a_lo: i64, a_hi: i64, b_lo: i64, b_hi: i64) -> (i64, i64) {
    let a = ((a_lo as u64) as u128) | ((a_hi as u64) as u128) << 64;
    let b = ((b_lo as u64) as u128) | ((b_hi as u64) as u128) << 64;

    match a.checked_add(b) {
        Some(result) => {
            (
                (result & 0xFFFFFFFFFFFFFFFF) as i64,
                ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
            )
        },
        _ => core::arch::wasm32::unreachable()
    }
}

#[no_mangle]
#[export_name = "mul-uint128"]
pub extern "C" fn mul_uint128(a_lo: i64, a_hi: i64, b_lo: i64, b_hi: i64) -> (i64, i64) {
    let a = ((a_lo as u64) as u128) | ((a_hi as u64) as u128) << 64;
    let b = ((b_lo as u64) as u128) | ((b_hi as u64) as u128) << 64;

    match a.checked_mul(b) {
        Some(result) => {
            (
                (result & 0xFFFFFFFFFFFFFFFF) as i64,
                ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
            )
        },
        _ => core::arch::wasm32::unreachable()
    }
}

const ARENA_SIZE: usize = 128 * 1024;
#[repr(C, align(32))]
struct SimpleAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    head: UnsafeCell<usize>,
}

impl SimpleAllocator {
    const fn new() -> Self {
        SimpleAllocator {
            arena: UnsafeCell::new([0; ARENA_SIZE]),
            head: UnsafeCell::new(0),
        }
    }
}

unsafe impl Sync for SimpleAllocator {}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Find the next address that has the right alignment.
        let idx = (*self.head.get()).next_multiple_of(align);
        // Bump the head to the next free byte
        *self.head.get() = idx + size;
        let arena: &mut [u8; ARENA_SIZE] = &mut (*self.arena.get());
        // If we ran out of arena space, we return a null pointer, which
        // signals a failed allocation.
        match arena.get_mut(idx) {
            Some(item) => item as *mut u8,
            _ => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* lol */
    }
}
