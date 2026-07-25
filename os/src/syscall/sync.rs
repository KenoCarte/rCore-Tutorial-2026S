use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    // Check deadlock before locking (release PCB lock before check)
    let should_check = {
        let process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detect_enabled
    };
    if should_check {
        let locked = {
            let process_inner = process.inner_exclusive_access();
            let m = process_inner.mutex_list[mutex_id].as_ref().unwrap();
            m.get_holder().is_some()
        };
        if locked && mutex_deadlock_detect(mutex_id) {
            return -0xdead;
        }
    }
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}

/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}

/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}

/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}

/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    // Check deadlock before down (release PCB lock before check to avoid double-borrow)
    let should_check = {
        let process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detect_enabled
    };
    if should_check {
        let (would_block, target_ptr) = {
            let process_inner = process.inner_exclusive_access();
            let sem = process_inner.semaphore_list[sem_id].as_ref().unwrap();
            (sem.get_count() <= 0, Arc::as_ptr(sem) as *const Semaphore)
        };
        if would_block && semaphore_deadlock_detect(sem_id, target_ptr) {
            return -0xdead;
        }
    }
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    0
}

/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}

/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}

/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}

/// enable deadlock detection syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_enable_deadlock_detect {}",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        enabled
    );
    if enabled != 0 && enabled != 1 {
        return -1;
    }
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.deadlock_detect_enabled = enabled == 1;
    0
}

/// Check if locking `mutex_id` would cause a deadlock.
/// Uses simple cycle detection: follow holder chain to see if we reach ourselves.
fn mutex_deadlock_detect(mutex_id: usize) -> bool {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let my_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    // Get holder of target mutex
    let target_mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap();
    let mut current_holder = match target_mutex.get_holder() {
        Some(tid) => tid,
        None => return false, // mutex is not locked, no deadlock
    };

    // Self-deadlock: same thread trying to lock a mutex it already holds
    if current_holder == my_tid {
        return true;
    }

    // Follow the wait-for chain: check if the holder is waiting for another mutex,
    // and follow that mutex's holder, etc. If we reach back to ourself, deadlock.
    let mut visited = alloc::vec![current_holder];
    loop {
        // Check all mutexes: is `current_holder` waiting for any of them?
        let mut found = false;
        for m in process_inner
            .mutex_list
            .iter()
            .filter_map(|m| m.as_ref())
        {
            let waiters = m.get_wait_queue_tids();
            if waiters.contains(&current_holder) {
                // This thread is waiting on this mutex — who holds it?
                if let Some(next_holder) = m.get_holder() {
                    if next_holder == my_tid {
                        return true; // cycle: we're in the chain
                    }
                    if !visited.contains(&next_holder) {
                        visited.push(next_holder);
                        current_holder = next_holder;
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            break; // holder is not waiting for any mutex, no cycle
        }
    }
    false
}

/// Banker's algorithm for semaphore deadlock detection.
/// `target_sem_ptr` is the raw Arc pointer of the semaphore being requested.
/// Returns true if granting the current request would lead to deadlock.
fn semaphore_deadlock_detect(_sem_id: usize, target_sem_ptr: *const Semaphore) -> bool {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let my_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    // Collect all semaphores
    let sems: Vec<&Arc<Semaphore>> = process_inner
        .semaphore_list
        .iter()
        .filter_map(|s| s.as_ref())
        .collect();
    let m = sems.len();
    if m == 0 {
        return false;
    }

    // Collect all thread tids that participate in this process
    // Skip threads that have already exited (res = None)
    let all_tids: Vec<usize> = process_inner
        .tasks
        .iter()
        .filter_map(|t| {
            t.as_ref().and_then(|t| {
                t.inner_exclusive_access()
                    .res
                    .as_ref()
                    .map(|r| r.tid)
            })
        })
        .collect();
    let n = all_tids.len();

    // Build Available, Allocation, Need
    let mut available = vec![0isize; m];
    let mut allocation = vec![vec![0isize; m]; n];
    let mut need = vec![vec![0isize; m]; n];

    for (j, sem) in sems.iter().enumerate() {
        available[j] = sem.get_count().max(0);
        let holders = sem.get_holders();
        for holder_tid in &holders {
            if let Some(i) = all_tids.iter().position(|t| t == holder_tid) {
                allocation[i][j] += 1;
            }
        }
        let waiters = sem.get_wait_queue_tids();
        for waiter_tid in &waiters {
            if let Some(i) = all_tids.iter().position(|t| t == waiter_tid) {
                need[i][j] += 1;
            }
        }
    }

    // Add the current request to Need
    if let Some(i) = all_tids.iter().position(|t| t == &my_tid) {
        // Find the index in sems corresponding to the target semaphore
        let j = sems
            .iter()
            .position(|s| Arc::as_ptr(s) as *const Semaphore == target_sem_ptr)
            .unwrap_or(0);
        need[i][j] += 1;
    }

    // Banker's algorithm
    let mut work = available.clone();
    let mut finish = vec![false; n];

    loop {
        let mut progress = false;
        for i in 0..n {
            if finish[i] {
                continue;
            }
            // Check if Need[i] <= Work
            let can_satisfy = (0..m).all(|j| need[i][j] as isize <= work[j]);
            if can_satisfy {
                // Thread i can finish — release its allocation
                for j in 0..m {
                    work[j] += allocation[i][j];
                }
                finish[i] = true;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    // If any thread cannot finish, deadlock
    finish.iter().any(|f| !*f)
}