# rCore-Tutorial-2026S — KenoCarte's Fork

基于 [rCore-Tutorial-2026S](https://github.com/LearningOS/rCore-Tutorial-2026S) 的 RISC-V 操作系统实验，从零构建一个 Unix-like 内核。

**Fork 仓库**：[KenoCarte/rCore-Tutorial-2026S](https://cnb.cool/KenoCarte/rCore-Tutorial-2026S)

---

## 章节进度

| 章节 | 主题 | 状态 |
|---|---|---|
| ch1 | 独立可执行程序：裸机 Rust，SBI 调用，linker.ld | ✅ 完成 |
| ch2 | 批处理系统：用户态切换，TrapContext，`__alltraps`/`__restore`/`sret` | ✅ 完成 |
| ch3 | 分时多任务：多道程序，TaskContext，`__switch`，Round-Robin，时钟中断 | ✅ 完成 |
| **ch4** | **地址空间：SV39 页表，MemorySet，跳板页，mmap/munmap** | 🔨 进行中 |
| ch5 | 进程：fork，exec，进程调度 | ⬜ 待开始 |
| ch6 | 文件系统：inode，块缓存，EasyFS | ⬜ 待开始 |
| ch7 | IPC & 信号：管道，信号处理 | ⬜ 待开始 |
| ch8 | 线程 & 同步：线程创建，互斥锁，信号量，条件变量 | ⬜ 待开始 |

> 评测章节：**ch3、ch4、ch5、ch6、ch8**（提交 PR 自动触发 CI 评分）

---

## 环境搭建

```bash
# 系统：WSL2 (Ubuntu) / Ubuntu 20.04+
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
# 克隆仓库
git clone https://cnb.cool/KenoCarte/rCore-Tutorial-2026S.git
cd rCore-Tutorial-2026S

# 克隆用户态测试用例
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user

# 切换到目标章节
git checkout ch4

# 编译 & 运行
cd os
make run
```

---

## 关键设计 (ch4)

### SV39 虚拟内存架构

```
用户虚拟地址空间:
  0xFFFFFFFFFFFFF000  TRAMPOLINE    (跳板页，__alltraps/__restore)
  0xFFFFFFFFFFFFE000  TRAP_CONTEXT  (陷入上下文)
         ...
  0x00000000????????  用户栈 / 堆 / 数据段 / 代码段

内核地址空间:
  0xFFFFFFFF????????  内核代码/数据/栈 (恒等映射物理内存)
  0x80200000          内核物理起始地址
```

### 跳板页机制

`__alltraps` 和 `__restore` 放在独立的 `.text.trampoline` 段，同时映射到内核和用户地址空间的最高的同一虚拟地址。这意味着切换页表（satp）时，当前执行的代码所在的虚拟页在两张页表中都存在，不会出现"切页表后 pc 指向无效地址"的问题。

- **U→S**：用户态 `ecall` → 硬件设 sepc/stval/scause → 切到 S 态 → `stvec` 指向 TRAMPOLINE → `__alltraps` 保存寄存器到 TRAP_CONTEXT → 读 `kernel_satp`/`kernel_sp` → 切换内核页表 → 跳转 `trap_handler`
- **S→U**：`trap_return()` 计算 `__restore` 在跳板页的 VA → 设 a0=TrapContext 指针, a1=用户 satp → `jr` 跳板页上的 `__restore` → 切用户页表 → 恢复寄存器 → `sret`

---

## 评测提交

在 CNB 上提 Pull Request：

1. Fork 本仓库到你的 CNB 账号
2. 切到目标章节分支，完成代码
3. `git push origin ch$ID`
4. 在 CNB 上创建 PR：源分支 → 目标仓库 `LearningOS/OSCamp-2026S/rCore-Tutorial-2026S` 的对应 `ch$ID` 分支
5. CI 自动运行测试，满分自动上传分数

---

## 参考资源

- [rCore-Tutorial-Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)
- [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [RISC-V Privileged Spec](https://github.com/riscv/riscv-isa-manual/releases)
