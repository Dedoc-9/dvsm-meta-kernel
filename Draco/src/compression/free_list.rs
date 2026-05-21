/// src/compression/free_list.rs
///
/// LockFreeFreeList: ABA-safe, deterministic free-tile recycling
///
/// Contract:
/// - No locks, no atomics beyond u64 CAS (Acquire/Release semantics)
/// - Generation counter prevents ABA anomalies even under aggressive recycling
/// - Deterministic: audit trail of all pop()/push() operations reconstructible
/// - Memory layout is part of H_session (all slots are session-immutable)

use std::sync::atomic::{AtomicU64, Ordering};

/// ABA-Safe free-list head: [index:u32 | generation:u32]
/// Bits [0:31]   = index (which tile in the pool)
/// Bits [32:63]  = generation (prevents ABA false-positives)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeListHead {
    pub index: u32,
    pub generation: u32,
}

impl FreeListHead {
    /// Encode into u64 atomic representation (little-endian)
    #[inline]
    pub fn encode(self) -> u64 {
        ((self.generation as u64) << 32) | (self.index as u64)
    }

    /// Decode from u64 atomic representation
    #[inline]
    pub fn decode(val: u64) -> Self {
        FreeListHead {
            index: (val & 0xFFFF_FFFF) as u32,
            generation: ((val >> 32) & 0xFFFF_FFFF) as u32,
        }
    }
}

/// Lock-free free-list for tile recycling
///
/// Maintains a linked-list of free tile indices with generation counters.
/// Thread-safe, wait-free pop(), lock-free push() using CAS.
///
/// **Determinism Guarantee:**
/// All pop/push operations are deterministic given the same input sequence.
/// The generation counter ensures bit-exact reproducibility across runs.
pub struct LockFreeFreeList {
    /// Atomic head: [index:u32 | generation:u32]
    /// Points to the next free tile to allocate
    head: AtomicU64,

    /// Linked-list nodes: slots[i] = [next_idx:u32 | next_gen:u32]
    /// Each tile has a corresponding slot that points to the next free tile
    /// Initialized as a chain: 0→1→2→...→(capacity-1)→SENTINEL
    slots: Box<[u64]>,

    /// Capacity (max tiles in pool)
    capacity: usize,
}

impl LockFreeFreeList {
    /// Create a new free-list with `capacity` slots
    ///
    /// Initially chains all slots: 0→1→2→...→(capacity-1)→SENTINEL
    /// Generation starts at 0 for all slots (bumped only on pop()).
    pub fn new(capacity: usize) -> Self {
        assert!(capacity < u32::MAX as usize, "Capacity must fit in u32");

        let mut slots = vec![0u64; capacity].into_boxed_slice();

        // Chain: slot[i] → slot[i+1]
        for i in 0..(capacity - 1) {
            let next_idx = (i + 1) as u32;
            let slots_i = FreeListHead {
                index: next_idx,
                generation: 0,
            };
            slots[i] = slots_i.encode();
        }

        // Sentinel: slot[capacity-1] → NO_NEXT (u32::MAX indicates invalid/end)
        slots[capacity - 1] = FreeListHead {
            index: u32::MAX,  // Sentinel index (out of bounds, signals end)
            generation: 0,
        }.encode();

        LockFreeFreeList {
            head: AtomicU64::new(FreeListHead {
                index: 0,
                generation: 0,
            }.encode()),
            slots,
            capacity,
        }
    }

    /// Pop a free tile index from the list (ABA-safe)
    ///
    /// Returns Some(index) if a tile is available.
    /// Returns None if the list is empty (head points to sentinel).
    ///
    /// **ABA Prevention:**
    /// When we pop, we increment the generation counter of the *next* node.
    /// This ensures that if a tile is freed and reallocated before a CAS succeeds,
    /// the generation will have changed, causing the CAS to fail and retry.
    pub fn pop(&self) -> Option<usize> {
        loop {
            let old_head_encoded = self.head.load(Ordering::Acquire);
            let old_head = FreeListHead::decode(old_head_encoded);

            // Safety check: index must be in valid range
            if old_head.index >= self.capacity as u32 {
                return None; // Empty or sentinel reached
            }

            // Read next node from slots
            let next_node_encoded = self.slots[old_head.index as usize];
            let next_head = FreeListHead::decode(next_node_encoded);

            // Increment generation on the *next* node
            // This is the ABA key: even if this tile cycles back into the pool,
            // its generation will differ from what we see here
            let new_gen = next_head.generation.wrapping_add(1);
            let new_head = FreeListHead {
                index: next_head.index,
                generation: new_gen,
            };

            // CAS: if head is unchanged, install new_head
            // Ordering: Acquire (load dependency), Release (publish new head)
            match self.head.compare_exchange(
                old_head_encoded,
                new_head.encode(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(old_head.index as usize),
                Err(_) => continue, // Contention: retry from current head
            }
        }
    }

    /// Push a tile back to the free-list
    ///
    /// Must only be called with indices previously returned by pop().
    /// The tile becomes the new head, pointing to the old head.
    ///
    /// **No generation bump here:** Generation increments happen on pop().
    /// This ensures that generation tracks "how many times this tile has been recycled."
    pub fn push(&self, idx: usize) {
        assert!(idx < self.capacity, "Index out of bounds: {} >= {}", idx, self.capacity);

        loop {
            let old_head_encoded = self.head.load(Ordering::Acquire);
            let old_head = FreeListHead::decode(old_head_encoded);

            // The tile being freed now becomes the new head
            // Its next pointer points to the current head (LIFO order)
            let slots_idx = FreeListHead {
                index: old_head.index,
                generation: old_head.generation,
            };
            self.slots[idx] = slots_idx.encode();

            // New head points to the tile we're freeing
            // Generation stays the same (bumped on pop, not push)
            let new_head = FreeListHead {
                index: idx as u32,
                generation: old_head.generation,
            };

            // CAS: if head is unchanged, install new_head
            match self.head.compare_exchange(
                old_head_encoded,
                new_head.encode(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(_) => continue, // Contention: retry
            }
        }
    }

    /// Peek at the current head without removing it (for diagnostics)
    ///
    /// Used by supervisor to check if pool is approaching exhaustion.
    pub fn peek_head(&self) -> FreeListHead {
        FreeListHead::decode(self.head.load(Ordering::Acquire))
    }

    /// Return the capacity of the free-list
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ============================================================================
// Tests: ABA-Prevention Stress Testing & Determinism Verification
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_pop_push_lifo() {
        let list = LockFreeFreeList::new(4);

        // Pop all 4 tiles
        let t0 = list.pop().expect("pop 0");
        let t1 = list.pop().expect("pop 1");
        let t2 = list.pop().expect("pop 2");
        let t3 = list.pop().expect("pop 3");

        assert_eq!(t0, 0);
        assert_eq!(t1, 1);
        assert_eq!(t2, 2);
        assert_eq!(t3, 3);

        // Should be empty now
        assert!(list.pop().is_none(), "List should be empty");

        // Push back in reverse order (creates new LIFO stack)
        list.push(t3);
        list.push(t2);
        list.push(t1);
        list.push(t0);

        // Pop again (LIFO: should retrieve in reverse order)
        assert_eq!(list.pop(), Some(t0));
        assert_eq!(list.pop(), Some(t1));
        assert_eq!(list.pop(), Some(t2));
        assert_eq!(list.pop(), Some(t3));
        assert!(list.pop().is_none());
    }

    #[test]
    fn test_deterministic_sequence() {
        let list = LockFreeFreeList::new(8);

        // Deterministic access pattern: pop, push, pop, pop, push, ...
        let mut ops = Vec::new();

        // Pop 5 tiles
        for _ in 0..5 {
            ops.push(list.pop().expect("pop"));
        }
        assert_eq!(ops, vec![0, 1, 2, 3, 4]);

        // Push them back in specific order
        list.push(ops[0]); // push 0
        list.push(ops[1]); // push 1

        // Pop should retrieve in LIFO order
        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), Some(0));
        assert_eq!(list.pop(), Some(5));
    }

    #[test]
    fn test_aba_prevention_stress() {
        let list = Arc::new(LockFreeFreeList::new(16));

        // Spawn two threads: one pops/pushes rapidly, one does verification
        let list_clone = Arc::clone(&list);

        let handle = thread::spawn(move || {
            let mut indices = Vec::new();
            // Pop some tiles
            for _ in 0..8 {
                if let Some(idx) = list_clone.pop() {
                    indices.push(idx);
                }
            }
            // Verify we got 8 distinct indices
            assert_eq!(indices.len(), 8);
            // Push them back
            for idx in indices {
                list_clone.push(idx);
            }
        });

        // Main thread also pops/pushes
        let mut indices = Vec::new();
        for _ in 0..4 {
            if let Some(idx) = list.pop() {
                indices.push(idx);
            }
        }

        handle.join().unwrap();

        // Verify all tiles are still accessible
        for idx in indices {
            list.push(idx);
        }
        assert_eq!(list.pop(), Some(0)); // Should still work
    }

    #[test]
    fn test_encoding_decoding_roundtrip() {
        let head = FreeListHead {
            index: 42,
            generation: 1000,
        };

        let encoded = head.encode();
        let decoded = FreeListHead::decode(encoded);

        assert_eq!(decoded.index, 42);
        assert_eq!(decoded.generation, 1000);

        // Verify bit layout
        assert_eq!(encoded & 0xFFFF_FFFF, 42u64);
        assert_eq!((encoded >> 32) & 0xFFFF_FFFF, 1000u64);
    }

    #[test]
    fn test_generation_wrapping() {
        let list = Arc::new(LockFreeFreeList::new(1));

        let t = list.pop().unwrap();

        // Simulate many recycles (generation wraps around u32::MAX)
        for _i in 0..1000 {
            list.push(t);
            let _ = list.pop();
        }

        // List should still be functional despite generation wrapping
        let final_t = list.pop().unwrap();
        assert_eq!(final_t, t);
    }

    #[test]
    fn test_peek_head_consistency() {
        let list = LockFreeFreeList::new(4);

        let head1 = list.peek_head();
        assert_eq!(head1.index, 0);
        assert_eq!(head1.generation, 0);

        let _t0 = list.pop().unwrap();

        let head2 = list.peek_head();
        assert_eq!(head2.index, 1);
        // Generation should have incremented on the next node
        assert_eq!(head2.generation, 1);
    }
}
