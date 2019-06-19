use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

static LIST_OF_COUNTER_LISTS: List<List<AtomicU64>> = List {
    head: AtomicPtr::new(null_mut()),
};

struct Node<T: Send + Sync> {
    data: T,
    next: AtomicPtr<Node<T>>,
}

struct List<T: Send + Sync> {
    head: AtomicPtr<Node<T>>,
}

impl<T: Send + Sync> List<T> {
    fn insert(&self, ptr_to_node: *mut Node<T>) {
        unsafe {
            let ref_to_node = &*ptr_to_node;
            let head = &self.head;
            let mut next = head.load(Ordering::Relaxed);
            loop {
                ref_to_node.next.store(next, Ordering::Relaxed);
                match head.compare_exchange_weak(
                    next,
                    ptr_to_node,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(new_next) => next = new_next,
                }
            }
        }
    }
}

pub struct Counter {
    ptr_to_list: *const List<AtomicU64>,
    node: *const Node<AtomicU64>,
}

impl Counter {
    pub fn new() -> Self {
        let counter_node = Box::into_raw(Box::new(Node {
            next: AtomicPtr::default(),
            data: AtomicU64::new(0),
        }));

        let counter_list_node = Box::into_raw(Box::new(Node {
            data: List {
                head: AtomicPtr::new(counter_node),
            },
            next: AtomicPtr::default(),
        }));

        LIST_OF_COUNTER_LISTS.insert(counter_list_node);

        Self {
            ptr_to_list: unsafe { &(*counter_list_node).data as *const _ },
            node: counter_node,
        }
    }

    pub fn get(&self) -> u64 {
        unsafe {
            let mut total = 0;
            let ref_to_list = &*self.ptr_to_list;
            let mut curr = ref_to_list.head.load(Ordering::Acquire);
            while let Some(c) = curr.as_ref() {
                let partial = c.data.load(Ordering::Relaxed);
                total += partial;
                curr = c.next.load(Ordering::Acquire);
            }
            total
        }
    }

    #[inline(never)]
    pub fn inc(&self) {
        unsafe {
            let node_ref = &*self.node;
            node_ref.data.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        let node = Box::into_raw(Box::new(Node {
            next: AtomicPtr::default(),
            data: AtomicU64::new(0),
        }));

        let ptr_to_list = unsafe { &*self.ptr_to_list };
        ptr_to_list.insert(node);

        Self { ptr_to_list, node }
    }
}

unsafe impl Send for Counter {}
unsafe impl Sync for Counter {}


#[cfg(test)]
mod tests {
    use super::Counter;

    #[test]
    fn test_counter_simple() {
        let counter = Counter::new();
        counter.inc();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_counter_clone() {
        let counter = Counter::new();
        let mut handles = Vec::new();

        for _ in 0..10 {
            let counter_clone = counter.clone();
            handles.push(std::thread::spawn(move || counter_clone.inc()));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.get(), 10);
    }
}


