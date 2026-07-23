//! Process management syscalls

use crate::mm::translated_byte_buffer;
use crate::task::{
    current_user_token, exit_current_and_run_next, get_syscall_count, mmap_current,
    munmap_current, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let token = current_user_token();
    let buffers =
        translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    let src = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const _ as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut copied = 0;
    for buffer in buffers {
        let to_copy = buffer.len();
        buffer.copy_from_slice(&src[copied..copied + to_copy]);
        copied += to_copy;
    }
    0
}

/// trace syscall: three modes based on trace_request
///
/// - 0 (Read):  read a byte from user address `id`, return the byte value, -1 if invalid
/// - 1 (Write): write `data` (as u8) to user address `id`, return 0, -1 if invalid
/// - 2 (Syscall): query how many times syscall `id` has been called by current task
/// - otherwise: return -1
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            let token = current_user_token();
            let page_table = crate::mm::PageTable::from_token(token);
            let addr = crate::mm::VirtAddr::from(id);
            match page_table.translate(addr.floor()) {
                Some(pte) if pte.is_valid() => {
                    pte.ppn().get_bytes_array()[addr.page_offset()] as isize
                }
                _ => -1,
            }
        }
        1 => {
            let token = current_user_token();
            let page_table = crate::mm::PageTable::from_token(token);
            let addr = crate::mm::VirtAddr::from(id);
            match page_table.translate(addr.floor()) {
                Some(pte) if pte.is_valid() && pte.writable() => {
                    pte.ppn().get_bytes_array()[addr.page_offset()] = data as u8;
                    0
                }
                _ => -1,
            }
        }
        2 => get_syscall_count(id) as isize,
        _ => -1,
    }
}

/// mmap: map a user memory region
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    mmap_current(start, len, prot)
}

/// munmap: unmap a user memory region
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    munmap_current(start, len)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = crate::task::change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
