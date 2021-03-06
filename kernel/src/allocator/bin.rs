use alloc::heap::{AllocErr, Layout};
use std::cmp::max;
use std::fmt;
use std::iter::Iterator;
use std::mem;
use std::ptr;

use allocator::linked_list::{LinkedList, Node};
use allocator::util::*;
use console::_print;

const BLOCK_LEN: usize = mem::size_of::<BlockHeader>();

macro_rules! exhausted {
    ($size:expr, $align: expr) => {
        AllocErr::Exhausted {
            request: unsafe { Layout::from_size_align_unchecked($size, $align) },
        }
    };
}

#[repr(C)]
struct BlockHeader {
    head: LinkedList,
    size: usize,
}

impl fmt::Debug for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BlockHeader {{ head: 0x{:x}, size: {}) }}",
            &self.head as *const LinkedList as usize, self.size
        )
    }
}

impl BlockHeader {
    #[inline]
    unsafe fn new_from_ptr(ptr: *mut usize) -> &'static mut BlockHeader {
        let block_header = &mut *(ptr as *mut BlockHeader);
        let mut head_ptr = &mut block_header.head as *mut LinkedList as *mut usize;
        // this might require volatile
        head_ptr = ptr::null_mut();
        block_header.size = 0;
        block_header
    }

    #[inline]
    unsafe fn from_ptr(ptr: *mut usize) -> &'static mut BlockHeader {
        &mut *(ptr as *mut BlockHeader)
    }

    #[inline]
    fn addr(&self) -> usize {
        self as *const BlockHeader as usize
    }

    #[inline]
    fn matches_exact(&self, layout: &Layout) -> bool {
        self.size == layout.size() && self.addr() % layout.align() == 0
    }

    #[inline]
    fn matches_contains(&self, layout: &Layout) -> bool {
        self.size > layout.size() && self.addr() % layout.align() == 0
    }

    #[inline]
    fn is_adjacent(&self, other: &BlockHeader) -> bool {
        self.addr().saturating_add(self.size) == other.addr()
    }

    #[inline]
    fn equal_size(&self, other: &BlockHeader) -> bool {
        self.size == other.size
    }

    #[inline]
    fn aligned_on(&self, align: usize) -> bool {
        self.addr() % align == 0
    }

    fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(head: {:x}, size: {})",
            &self.head as *const LinkedList as usize, self.size
        )
    }
}

struct BlockHeaderList<'a> {
    head: &'a LinkedList,
}

impl<'a> fmt::Debug for BlockHeaderList<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(
                self.head
                    .iter()
                    .map(|ptr| unsafe { BlockHeader::from_ptr(ptr) }),
            ).finish()
    }
}

/// A simple allocator that allocates based on size classes.
#[derive(Debug)]
pub struct Allocator {
    start: usize,
    end: usize,
    head: LinkedList,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        // TODO, where does the stack start and end???
        let start = align_up(start, BLOCK_LEN);
        let end = align_down(end, BLOCK_LEN);

        let mut head = LinkedList::new();
        let block_header = unsafe { BlockHeader::new_from_ptr(start as *mut usize) };
        block_header.size = end - start;
        unsafe { head.push(block_header.addr() as *mut usize) };

        let mut allocator = Allocator { start, end, head };
        allocator.divide_maximally();
        allocator
    }

    fn print_block_header_list(&self, message: &'static str) {
        kprintln!("{}: {:?}", message, BlockHeaderList { head: &self.head })
    }

    #[inline]
    fn next_power_of_two_below(&self, n: usize) -> usize {
        // TODO figure out a way to avoid all of these awful unwraps
        n.checked_next_power_of_two().unwrap() >> 1
    }

    fn inner_layout(&self, layout: &Layout) -> Option<Layout> {
        let mut size = max(layout.size(), BLOCK_LEN);
        if !size.is_power_of_two() {
            size = size.checked_next_power_of_two().unwrap();
        }

        Layout::from_size_align(size, max(layout.align(), BLOCK_LEN))
    }

    fn divide_maximally(&mut self) {
        let mut iter = self.head.iter();
        while let Some(ptr) = iter.peek() {
            let ptr = ptr as usize;
            let cbh = unsafe { BlockHeader::from_ptr(ptr as *mut usize) };
            let alignment = self.next_power_of_two_below(cbh.size);
            let aligned_ptr = align_up(ptr, alignment);
            let next_ptr = if ptr != aligned_ptr {
                aligned_ptr
            } else if !cbh.size.is_power_of_two() {
                ptr + self.next_power_of_two_below(cbh.size)
            } else {
                ptr
            };

            if ptr != next_ptr {
                let nbh = unsafe { BlockHeader::new_from_ptr(next_ptr as *mut usize) };
                let diff = next_ptr.saturating_sub(ptr);
                nbh.size = cbh.size - diff;
                cbh.size = diff;
                unsafe { cbh.head.push(next_ptr as *mut usize) };
                continue;
            }
            iter.next();
        }
    }

    // Iterate pointers, and if two blocks are adjacent, same sized, and the
    // first block is aligned on the sum of their sizes, they can be joined.
    // Note that this is a single pass, so optimal defragmentation would take
    // multiple passes until the freelist no longer changed.
    fn defragment(&mut self) {
        self.head
            .iter()
            .fold(None, |prev_ptr, cur_ptr| -> Option<*mut usize> {
                if let Some(prev_ptr) = prev_ptr {
                    let pbh = unsafe { BlockHeader::from_ptr(prev_ptr) };
                    let cbh = unsafe { BlockHeader::from_ptr(cur_ptr) };
                    if pbh.is_adjacent(cbh) && pbh.equal_size(cbh) && pbh.aligned_on(2 * pbh.size) {
                        pbh.head.pop();
                        pbh.size <<= 1;
                    }
                }
                Some(cur_ptr)
            });
    }

    fn find_exact_free_block(
        &mut self,
        layout: &Layout,
    ) -> Option<(Node, &'static mut BlockHeader)> {
        self.head
            .iter_mut()
            .map(|node| unsafe {
                let value = node.value();
                (node, BlockHeader::from_ptr(value))
            }).find(|&(_, ref block_header)| -> bool { block_header.matches_exact(&layout) })
    }

    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning `Err` indicates that either memory is exhausted
    /// (`AllocError::Exhausted`) or `layout` does not meet this allocator's
    /// size or alignment constraints (`AllocError::Unsupported`).
    pub fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        let inner_layout = self.inner_layout(&layout).unwrap();
        let addr = self.alloc_inner(&inner_layout).map_err(|e| match e {
            AllocErr::Exhausted { .. } => exhausted!(layout.size(), layout.align()),
            _ => e,
        })?;
        self.divide_maximally();
        Ok(addr)
    }

    fn alloc_inner(&mut self, layout: &Layout) -> Result<(*mut u8), AllocErr> {
        if let Some((node, _)) = self.find_exact_free_block(&layout) {
            return Ok(node.pop() as *mut u8);
        }

        let head_ptr = &mut self.head as *mut LinkedList as *mut usize;
        let res = self
            .head
            .iter()
            .fold((Some(head_ptr), None), |accum, cur_ptr| match accum {
                (Some(prev_ptr), Some(cur_ptr)) => (Some(prev_ptr), Some(cur_ptr)),
                (Some(prev_ptr), None) => {
                    let block_header = unsafe { BlockHeader::from_ptr(cur_ptr) };
                    if block_header.matches_contains(&layout) {
                        (Some(prev_ptr), Some(cur_ptr))
                    } else {
                        (
                            Some(&mut block_header.head as *mut LinkedList as *mut usize),
                            None,
                        )
                    }
                }
                _ => unreachable!(),
            });

        if let (Some(list_ptr), Some(cbh_ptr)) = res {
            let list = unsafe { &mut *(list_ptr as *mut LinkedList) };
            let cbh = unsafe { BlockHeader::from_ptr(cbh_ptr) };
            let mbh = unsafe {
                BlockHeader::new_from_ptr(cbh.addr().saturating_add(layout.size()) as *mut usize)
            };
            let diff = mbh.addr().saturating_sub(cbh.addr());
            mbh.size = cbh.size.saturating_sub(diff);
            // TODO just for during development

            assert!(list.pop().unwrap() as usize == cbh.addr());
            assert!(cbh.addr() != mbh.addr());
            unsafe { list.push(mbh.addr() as *mut usize) };
            return Ok(cbh.addr() as *mut u8);
        } else {
            Err(exhausted!(layout.size(), layout.align()))
        }
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if (ptr as usize) < self.start || (ptr as usize) >= self.end {
            panic!("deallocated pointer is not owned by allocator");
        }

        let inner_layout = self.inner_layout(&layout).unwrap();
        self.dealloc_inner(ptr, &inner_layout);

        self.defragment();
    }

    pub fn dealloc_inner(&mut self, ptr: *mut u8, layout: &Layout) {
        let freed_ptr = ptr as *mut usize;
        let head_ptr = &mut self.head as *mut LinkedList as *mut usize;
        let res = self
            .head
            .iter()
            .fold((Some(head_ptr), None), |accum, cur_ptr| match accum {
                (Some(prev_ptr), Some(cur_ptr)) => (Some(prev_ptr), Some(cur_ptr)),
                (Some(prev_ptr), None) => {
                    if freed_ptr < cur_ptr {
                        (Some(prev_ptr), Some(cur_ptr))
                    } else {
                        (Some(cur_ptr), None)
                    }
                }
                _ => unreachable!(),
            });

        if let (Some(prev_ptr), Some(_)) = res {
            let list = unsafe { &mut *(prev_ptr as *mut LinkedList) };
            let block_header = unsafe { BlockHeader::new_from_ptr(freed_ptr) };
            block_header.size = layout.size();
            assert!(block_header.addr() % layout.align() == 0);
            unsafe { list.push(block_header.addr() as *mut usize) };
        } else {
            panic!("invalid pointer provided")
        }
    }
}
