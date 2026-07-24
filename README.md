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
| **ch6** | **文件系统** | **inode，块缓存，EasyFS，硬链接，virtio-blk** | ✅ |
| ch8 | 线程同步 | 线程创建，互斥锁，信号量，条件变量 | ⬜ |

> 评测章节：**ch3、ch4、ch5、ch6、ch8**

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
git checkout ch6
cd os
make run
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

## 关键修复

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
