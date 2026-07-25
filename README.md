# rCore-Tutorial-2026S — KenoCarte's Fork

基于 [rCore-Tutorial-2026S](https://github.com/LearningOS/rCore-Tutorial-2026S) 的 RISC-V 操作系统实验，从零构建 Unix-like 内核。

**Fork 仓库**：[KenoCarte/rCore-Tutorial-2026S](https://cnb.cool/KenoCarte/rCore-Tutorial-2026S)

---

## 章节进度

| 章节 | 主题 | 核心概念 | 状态 |
|---|---|---|---|
| ch1 | 裸机程序 | SBI ecall，linker.ld，no_std Rust | ✅ |
| ch2 | 批处理系统 | TrapContext，`__alltraps`/`__restore`/`sret`，U→S 切换 | ✅ |
| ch3 | 分时多任务 | TaskContext，`__switch`，Round-Robin，时钟中断 | ✅ |
| ch4 | 地址空间 | SV39 页表，MemorySet，跳板页，mmap/munmap | ✅ |
| ch5 | 进程模型 | fork/exec/waitpid，spawn，PID 分配，进程调度 | ✅ |
| ch6 | 文件系统 | inode，块缓存，EasyFS，硬链接，virtio-blk | ✅ |
| **ch7** | **进程间通信** | **管道，信号，sigaction，kill，exec 传参** | ✅ |
| ch8 | 线程同步 | 线程创建，互斥锁，信号量，条件变量 | ⬜ |

> 评测章节：**ch3、ch4、ch5、ch6、ch7、ch8**

---

## 环境搭建

```bash
# Rust 工具链
rustup install nightly-2024-05-02
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src llvm-tools-preview

# QEMU
sudo apt install qemu-system-riscv64

# cargo-binutils
cargo install cargo-binutils --version=0.3.6
```

---

## 快速开始

```bash
git clone https://cnb.cool/KenoCarte/rCore-Tutorial-2026S.git
cd rCore-Tutorial-2026S
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
git checkout ch7
cd os
make run
```

---

## ch7 进程间通信架构

```
User App
  │ pipe/read/write/close (fork+pipe for IPC)
  │ kill/sigaction/sigprocmask/sigreturn (signals)
  │ exec(path, argv) — 带参数的程序执行
  ▼
syscall (os/src/syscall/)
  ├── fs.rs:       sys_pipe / sys_dup          ← 本章新增
  └── process.rs:  sys_kill / sys_sigaction /  ← 本章新增
                   sys_sigprocmask / sys_sigreturn
                   sys_exec(path, argv)         ← 签名变更
  ▼
Pipe (os/src/fs/pipe.rs)
  │ Ring Buffer (32 bytes), head/tail, Full/Empty/Normal
  │ read: 空时 yield 等待 write end 关闭则 EOF
  │ write: 满时 yield 等待
  │ make_pipe() → (read_end, write_end) 两个 Arc<Pipe>
  ▼
Signal (os/src/task/signal.rs + action.rs + mod.rs)
  │ SignalFlags: 31 种信号 (SIGINT/SIGKILL/SIGSEGV/...)
  │ SignalAction: handler 函数地址 + mask 屏蔽集
  │ SignalActions: 每种信号的默认动作表
  │ check_pending_signals() → call_user/kernel_signal_handler()
  ▼
TaskControlBlock (os/src/task/task.rs)
  │ signals / signal_mask / handling_sig / signal_actions
  │ killed / frozen / trap_ctx_backup
  │ exec 传参: args 推入用户栈 → a0=argc, a1=argv_base
  ▼
Trap (os/src/trap/mod.rs)
  │ PageFault/IllegalInst → current_add_signal(SIGSEGV/SIGILL)
  │ handle_signals() → check_pending_signals → handler
  │ check_signals_error_of_current() → exit if fatal
```

### Pipe 管道

```
make_pipe():
  buffer = Arc<UPSafeCell<PipeRingBuffer>>  (32 bytes)
  read_end  = Arc<Pipe>  (readable=true,  writable=false)
  write_end = Arc<Pipe>  (readable=false, writable=true)

sys_pipe(pipe[2]):
  read_fd  = alloc_fd() → fd_table[read_fd]  = pipe_read
  write_fd = alloc_fd() → fd_table[write_fd] = pipe_write
  pipe[0] = read_fd, pipe[1] = write_fd

fork 后父子进程共享 pipe fds:
  父 close(write_fd), 读数据 → 子 close(read_fd), 写数据

读端检测写端关闭: PipeRingBuffer.write_end (Weak<Pipe>)
  → all_write_ends_closed() → 返回 EOF (already_read)
```

### Signal 信号

```
信号生命周期:
  1. 产生: sys_kill(pid, sig)     → task.signals.insert(flag)
           trap (pagefault)       → current_add_signal(SIGSEGV)
  2. 投递: handle_signals()       → check_pending_signals()
  3. 处理:
     a) Kernel 信号 (SIGKILL/SIGSTOP/SIGCONT/SIGDEF):
        call_kernel_signal_handler()
        - SIGSTOP → frozen = true
        - SIGCONT → frozen = false
        - SIGKILL → killed = true
     b) User 信号:
        call_user_signal_handler(sig, signal)
        - 备份 trap_ctx → trap_ctx_backup
        - 修改 sepc = handler, a0 = sig
        - sigreturn 时恢复 trap_ctx
  4. 检查: check_signals_error_of_current()
        - SIGSEGV → exit(-11)
        - SIGILL  → exit(-4)
        - SIGKILL → exit(-9)

信号屏蔽:
  - signal_mask: 当前进程屏蔽的信号集
  - handling_sig: 正在处理的信号 (-1 表示无)
  - 处理某信号时, 其 action.mask 中标记的信号也被屏蔽
  - sys_sigprocmask(mask) 设置 signal_mask
```

### exec 传参

```
旧: exec(path)
新: exec(path, argv)

sys_exec(path, args):
  1. 解析 argv 数组 (以 NULL 结尾)
  2. 将 args 推入用户栈:
     [arg0_str\0][arg1_str\0]...[argv_array][argc]
  3. 设置 trap_cx:
     a0 (x10) = argc
     a1 (x11) = argv_base
  4. 返回 argc
```

### PID2TCB 映射

```
PID2TCB: BTreeMap<usize, Arc<TaskControlBlock>>
  - add_task()     → PID2TCB.insert(pid, task)
  - pid2task(pid)  → PID2TCB.get(&pid)
  - exit()         → remove_from_pid2task(pid)
  - sys_kill(pid)  → pid2task(pid).signals.insert(flag)
```

---

## ch6 文件系统架构

```
User App
  │ open/read/write/close/fstat/link/unlink (lib.rs → ecall)
  ▼
syscall (os/src/syscall/)
  ├── fs.rs:       sys_open / sys_read / sys_write / sys_close
  │                sys_fstat / sys_linkat / sys_unlinkat   ← 本章实现
  └── process.rs:  fork/exec/spawn/mmap/munmap/get_time/set_priority
  ▼
OSInode (os/src/fs/inode.rs)
  │ Arc<Inode> 封装，实现 File trait (read/write/ino/nlink/is_dir)
  │ link_file() / unlink_file() → ROOT_INODE
  ▼
easy-fs Inode (easy-fs/src/vfs.rs)
  │ VFS 层：find/create/link/unlink/ls/read_at/write_at/clear
  │ nlink 归零时自动回收 inode + data blocks
  ▼
DiskInode (easy-fs/src/layout.rs)
  │ 磁盘数据结构：size/direct/indirect1/indirect2/type_/nlink
  ▼
BlockCache → BLOCK_DEVICE → VirtIOBlock → virtio-drivers → QEMU virtio-blk
```

### 进程文件描述符表

```
TaskControlBlockInner
├── fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>
│   ├── [0] Stdin   (fd 0)
│   ├── [1] Stdout  (fd 1)
│   ├── [2] Stdout  (fd 2, stderr)
│   ├── [3] OSInode → easy-fs Inode (文件)
│   ├── [4] OSInode → easy-fs Inode (文件)
│   └── ...
└── alloc_fd(): 自动分配最小可用 fd
```

### 硬链接 (Hard Link)

```
ROOT_INODE (目录)
├── dirent[0]: "fname2"   → ino=5 (nlink=4)
├── dirent[1]: "linkname0" → ino=5
├── dirent[2]: "linkname1" → ino=5
└── dirent[3]: "linkname2" → ino=5

Inode (ino=5)
├── nlink = 4
├── size = ...
└── data blocks [...]

link("fname2", "newname"):
  1. find_inode_id("fname2") → target_ino
  2. 检查 "newname" 不存在，同名返回 -1
  3. target_inode.nlink += 1
  4. ROOT_INODE 添加 dirent("newname" → target_ino)

unlink("fname2"):
  1. find_inode_id("fname2") → target_ino
  2. 检查文件存在，否则返回 -1
  3. target_inode.nlink -= 1
  4. 如果 nlink == 0：clear_size() → dealloc_data() → dealloc_inode()
  5. ROOT_INODE 移除对应 dirent，目录 shrink
```

### fstat

```
fstat(fd, &stat):
  struct Stat {
      dev: u64,        // 固定 0（同一设备）
      ino: u64,        // inode 编号
      mode: StatMode,  // DIR(0o040000) 或 FILE(0o100000)
      nlink: u32,      // 硬链接计数
      pad: [u64; 7],   // 对齐填充
  }
```

---

## 关键修复 (ch7)

### 信号支持的 trap 重构

- **旧行为**：PageFault/IllegalInstruction 直接 `exit_current_and_run_next(-2/-3)` 杀死进程
- **新行为**：调用 `current_add_signal(SIGSEGV/SIGILL)`，由 `handle_signals()` 统一处理
  - 如果进程注册了 handler → 备份 trap_ctx，跳转到 handler 执行
  - 如果未注册 → killed=true，最终 `check_signals_error_of_current()` 触发 exit
  - 退出码从 -2 变为 -11 (SIGSEGV) / -4 (SIGILL)，符合 POSIX 规范

### TrapContext Clone+Copy

- `TrapContext` 新增 `Clone + Copy` derive
- `call_user_signal_handler` 中通过 `*trap_ctx` 按位复制备份到 `trap_ctx_backup`
- `sigreturn` 时通过 `*trap_ctx = inner.trap_ctx_backup.unwrap()` 整体恢复

### easy-fs BlockCache Vec\<u8\>

- `BlockCache.cache` 从 `[u8; BLOCK_SZ]` 改为 `Vec<u8>`，改善对齐和移动效率
- `easy-fs/src/lib.rs` 移除 `#![deny(missing_docs)]`
- `create()` 中 name conflict 检查从 `read_disk_inode` 改为 `modify_disk_inode`

### exec 传参兼容

- `sys_exec` 签名从 `fn(path)` 改为 `fn(path, args: *const usize)`
- `TaskControlBlock::exec` 接受 `args: Vec<String>`
- 参数推入新的用户栈，`a0=argc, a1=argv`

### ch5/ch6 实现移植

- `sys_get_time`、`sys_mmap`、`sys_munmap`、`sys_spawn`、`sys_set_priority` 从 ch6 移植
- `sys_fstat`、`sys_linkat`、`sys_unlinkat` 从 ch6 移植
- `sys_spawn` 适配 ch6 文件系统加载 (`open_file + read_all`)

### 测试结果

```
ch7b_sig_simple       ✓ (exit 0)
ch7b_pipetest         ✓ (exit 0)
ch7b_pipe_large_test  ✓ (exit 0)
ch6b_filetest_simple  ✓ (exit 0)
ch5b_forktest         ✓ (exit 0)
...
Basic usertests passed!
```

---

## 关键修复 (ch6)

### QEMU 6.2 兼容性

- **问题**：QEMU 6.2 的 `virtio-blk-device` 需显式挂载到 `virtio-mmio-bus.0`，否则设备不会出现在 MMIO 槽位
- **修复**：Makefile 中 `qemu-system-riscv64` 命令加 `-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0`
- **块设备发现**：`VirtIOBlock::new()` 扫描全部 8 个 MMIO 槽位（0x10001000 → 0x10008000）匹配 `magic==0x74726976 && device_id==2` 的块设备
- **MMIO 映射**：`config.rs` 中 MMIO 区域扩展为 `(0x10001000, 0x8000)` 覆盖全部槽位
- **依赖源**：`riscv` 和 `virtio-drivers` crate 改用本地 patch + gitee 镜像

### unlink 目录收缩 bug

- **问题**：`root_inode.size = new_size` 在 dirent shift 循环**之前**执行，导致循环内 `read_at` 因超出 size 边界返回 0
- **修复**：将 `size` 更新移至 shift 循环**之后**

### ch5 实现移植

- `sys_spawn` 从 `get_app_data_by_name` 改为 `open_file + read_all`（适配 ch6 文件系统加载 ELF）
- 其余 ch5 实现（`sys_mmap`、`sys_munmap`、`sys_get_time`、`sys_set_priority`、`priority` 字段）从 ch5 分支移植

---

## 评测提交

在 CNB 上提 Pull Request：

1. Fork 到 [KenoCarte/rCore-Tutorial-2026S](https://cnb.cool/KenoCarte/rCore-Tutorial-2026S)
2. 切到目标章节分支，完成代码
3. `git push origin ch6`
4. CNB 创建 PR：源分支 → `LearningOS/OSCamp-2026S/rCore-Tutorial-2026S` 对应 `ch6`

---

## 参考

- [rCore-Tutorial-Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [RISC-V Privileged Spec](https://github.com/riscv/riscv-isa-manual/releases)
- [Virtual I/O Device (VIRTIO) Version 1.2](https://docs.oasis-open.org/virtio/virtio/v1.2/virtio-v1.2.html)
