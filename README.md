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
| **ch5** | **进程模型** | fork/exec/waitpid，spawn，PID 分配，进程调度 | ✅ |
| ch6 | 文件系统 | inode，块缓存，EasyFS | ⬜ |
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
cargo install cargo-binutils --vers=0.3.3
```

---

## 快速开始

```bash
git clone https://cnb.cool/KenoCarte/rCore-Tutorial-2026S.git
cd rCore-Tutorial-2026S
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
git checkout ch5
cd os
make run
```

---

## 关键设计

### ch5 进程模型

```
┌─────────────────────────────────┐
│         TaskManager              │
│  ready_queue: VecDeque<Arc<TCB>> │  FIFO 就绪队列
└─────────────────────────────────┘
             ↓ fetch / add
┌─────────────────────────────────┐
│          Processor               │
│  current: Option<Arc<TCB>>       │  当前运行进程
│  idle_task_cx: TaskContext       │  调度锚点
└─────────────────────────────────┘

TaskControlBlock (Arc 共享)
├── pid: PidHandle (RAII 回收)
├── kernel_stack: KernelStack (RAII 回收)
└── inner: UPSafeCell<TaskControlBlockInner>
    ├── memory_set: MemorySet     (地址空间)
    ├── parent: Option<Weak<TCB>> (父进程弱引用)
    ├── children: Vec<Arc<TCB>>   (子进程列表)
    ├── exit_code: i32            (退出码)
    ├── priority: isize           (调度优先级)
    └── heap_bottom / program_brk (堆管理)
```

- **fork**：复制父进程地址空间（`MemorySet::from_existed_user`），子进程 trap_cx.x[10] = 0
- **exec**：替换当前进程地址空间，复用 PID 和内核栈
- **spawn**：直接从 ELF 创建新进程（跳过 fork 的复制阶段）
- **waitpid**：查找 Zombie 子进程，读取 exit_code，移除并回收

---

## 评测提交

在 CNB 上提 Pull Request：

1. Fork 到 [KenoCarte/rCore-Tutorial-2026S](https://cnb.cool/KenoCarte/rCore-Tutorial-2026S)
2. 切到目标章节分支，完成代码
3. `git push origin ch$ID`
4. CNB 创建 PR：源分支 → `LearningOS/OSCamp-2026S/rCore-Tutorial-2026S` 对应 `ch$ID`

---

## 参考

- [rCore-Tutorial-Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [RISC-V Privileged Spec](https://github.com/riscv/riscv-isa-manual/releases)
