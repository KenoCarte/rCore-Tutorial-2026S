//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    /// tids of threads currently holding units of this semaphore
    pub holders: Vec<usize>,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    holders: Vec::new(),
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        // Remove this thread from holders
        if let Some(pos) = inner.holders.iter().position(|t| *t == tid) {
            inner.holders.remove(pos);
        }
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
            // After waking up, we now hold a unit
            let mut inner = self.inner.exclusive_access();
            inner.holders.push(tid);
        } else {
            inner.holders.push(tid);
        }
    }

    /// Get the current count for deadlock detection
    pub fn get_count(&self) -> isize {
        self.inner.exclusive_access().count
    }

    /// Get the tids of current holders for deadlock detection
    pub fn get_holders(&self) -> Vec<usize> {
        self.inner.exclusive_access().holders.clone()
    }

    /// Get the tids of waiting threads for deadlock detection
    pub fn get_wait_queue_tids(&self) -> Vec<usize> {
        self.inner
            .exclusive_access()
            .wait_queue
            .iter()
            .map(|t| {
                t.inner_exclusive_access()
                    .res
                    .as_ref()
                    .unwrap()
                    .tid
            })
            .collect()
    }
}
