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
    unsafe fn new_from_ptr(ptr: usize) -> &'static mut BlockHeader {
        let block_header = &mut *(ptr as *mut usize as *mut BlockHeader);
        let mut head_ptr = &mut block_header.head as *mut LinkedList as *mut usize;
        head_ptr = ptr::null_mut();
        block_header.size = 0;
        block_header
    }

    #[inline]
    unsafe fn from_ptr(ptr: usize) -> &'static mut BlockHeader {
        &mut *(ptr as *mut usize as *mut BlockHeader)
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
                    .map(|ptr| unsafe { BlockHeader::from_ptr(ptr as usize) }),
            ).finish()
    }
}

/// A simple allocator that allocates based on size classes.
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
        let block_header = unsafe { BlockHeader::new_from_ptr(start) };
        block_header.size = end - start;
        unsafe { head.push(block_header.addr() as *mut usize) };

        let mut allocator = Allocator { start, end, head };
        kprintln!("before divide: {:?}", allocator.head);
        allocator.divide_maximally();
        kprintln!("after divide: {:?}", allocator.head);
        allocator
    }

    #[inline]
    fn next_power_of_two_below(&self, n: usize) -> usize {
        // TODO figure out a way to avoid all of these awful unwraps
        n.checked_next_power_of_two().unwrap() >> 1
    }

    fn divide_maximally(&mut self) {
        let list = self.head;
        // unsafe { list.push(head as *mut usize) };
        kprintln!("list before: {:?}", list);
        {
            kprintln!(
                "block header list before {:?}",
                BlockHeaderList { head: &self.head }
            )
        }

        let mut iter = list.iter();
        loop {
            if let Some(ptr) = iter.peek() {
                let ptr = ptr as usize;
                println!("next pointer to divide: 0x{:x}", ptr);
                let block_header = unsafe { BlockHeader::from_ptr(ptr) };
                let alignment = self.next_power_of_two_below(block_header.size);
                let aligned_ptr = align_up(ptr, alignment);
                let next_ptr = if ptr != aligned_ptr {
                    aligned_ptr
                } else if !block_header.size.is_power_of_two() {
                    ptr + self.next_power_of_two_below(block_header.size)
                } else {
                    ptr
                };

                if ptr != next_ptr {
                    let next_block_header = unsafe { BlockHeader::new_from_ptr(next_ptr) };
                    let diff = next_ptr.saturating_sub(ptr);
                    next_block_header.size = block_header.size - diff;
                    block_header.size = diff;
                    kprintln!(
                        "pushing: 0x{:x}, size: 0x{:x}, alignment: 0x{:x}",
                        next_ptr,
                        next_block_header.size,
                        alignment
                    );
                    unsafe { block_header.head.push(next_ptr as *mut usize) };
                    continue;
                }
            } else {
                break;
            }
            iter.next();
        }
        kprintln!("list after: {:?}", list);
        {
            kprintln!(
                "block header list after {:?}",
                BlockHeaderList { head: &self.head }
            )
        }
    }

    fn find_exact_free_block(
        &mut self,
        layout: &Layout,
    ) -> Option<(Node, &'static mut BlockHeader)> {
        self.head
            .iter_mut()
            .map(|node| unsafe {
                let value = node.value();
                (node, BlockHeader::from_ptr(value as usize))
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
        kprintln!("[alloc] list before: {:?}", self.head);
        {
            kprintln!(
                "[alloc] block header list before {:?}",
                BlockHeaderList { head: &self.head }
            )
        }
        let mut size = max(layout.size(), BLOCK_LEN);
        if !size.is_power_of_two() {
            size = size.checked_next_power_of_two().unwrap();
        }

        let inner_layout = Layout::from_size_align(size, max(layout.align(), BLOCK_LEN)).unwrap();
        let (addr, new_block_addr) = self.alloc_inner(&inner_layout).map_err(|e| match e {
            AllocErr::Exhausted { .. } => exhausted!(layout.size(), layout.align()),
            _ => e,
        })?;
        if let Some(addr) = new_block_addr {
            self.divide_maximally();
        }
        kprintln!("[alloc] list after: {:?}", self.head);
        {
            kprintln!(
                "[alloc] block after list before {:?}",
                BlockHeaderList { head: &self.head }
            )
        }

        Ok(addr)
    }

    fn alloc_inner(&mut self, layout: &Layout) -> Result<(*mut u8, Option<usize>), AllocErr> {
        if let Some((node, _)) = self.find_exact_free_block(&layout) {
            kprintln!("exact match: {:?}", node.value());
            return Ok((node.pop() as *mut u8, None));
        }

        let head_ptr = &mut self.head as *mut LinkedList as *mut usize;
        let res = self
            .head
            .iter()
            .fold((Some(head_ptr), None), |accum, cur_ptr| match accum {
                (Some(prev_ptr), Some(cur_ptr)) => (Some(prev_ptr), Some(cur_ptr)),
                (Some(prev_ptr), None) => {
                    let block_header = unsafe { BlockHeader::from_ptr(cur_ptr as usize) };
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
            let cbh = unsafe { BlockHeader::from_ptr(cbh_ptr as usize) };
            let mbh =
                unsafe { BlockHeader::new_from_ptr(cbh.addr().saturating_add(layout.size())) };
            let diff = mbh.addr().saturating_sub(cbh.addr());
            mbh.size = cbh.size.saturating_sub(diff);
            // TODO just for during development

            kprintln!(
                "[move block] list 0x{:x} before: {:?}",
                list as *const LinkedList as usize,
                list
            );
            assert!(list.pop().unwrap() as usize == cbh.addr());
            assert!(cbh.addr() != mbh.addr());
            unsafe { list.push(mbh.addr() as *mut usize) };
            kprintln!(
                "[move block] list 0x{:x} after: {:?}",
                list as *const LinkedList as usize,
                list
            );
            kprintln!("requested layout: {:?}", layout);
            kprintln!("cbh: {:#?}, addr: 0x{:x}", cbh, cbh.addr());
            kprintln!("mbh: {:#?}, addr: 0x{:x}", mbh, mbh.addr());
            return Ok((cbh.addr() as *mut u8, Some(mbh.addr())));
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
    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {}
}
//
// FIXME: Implement `Debug` for `Allocator`.
