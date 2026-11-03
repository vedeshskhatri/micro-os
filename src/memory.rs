#[derive(Debug, Eq, PartialEq)]
pub struct Allocation {
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug)]
pub struct BumpAllocator {
    heap: Vec<u8>,
    next: usize,
}

impl BumpAllocator {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "allocator capacity must be positive");
        Self {
            heap: vec![0; capacity],
            next: 0,
        }
    }

    pub fn allocate(&mut self, size: usize, alignment: usize) -> Option<Allocation> {
        if size == 0 || !alignment.is_power_of_two() {
            return None;
        }
        let aligned = (self.next + alignment - 1) & !(alignment - 1);
        let end = aligned.checked_add(size)?;
        if end > self.heap.len() {
            return None;
        }
        self.next = end;
        Some(Allocation {
            offset: aligned,
            size,
        })
    }

    pub fn used(&self) -> usize {
        self.next
    }

    pub fn capacity(&self) -> usize {
        self.heap.len()
    }
}
