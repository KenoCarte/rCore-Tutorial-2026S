# rCore-Tutorial-2026S

> **KenoCarte** | OS2EDU 2026 春夏季开源操作系统训练营 专业阶段

基于 [rCore-Tutorial v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)，在 RISC-V 平台上用 Rust 从零实现一个类 Unix 内核。

---

## 章节结构

| 章节 | 分支 | 内容 | 评测 |
|------|------|------|:----:|
| ch1 | `ch1` | 裸机启动：linker.ld、entry.asm、SBI 调用、println! 宏 | — |
| ch2 | `ch2` | 批处理系统：特权级切换、系统调用、用户程序加载 | — |
| ch3 | `ch3` | 多道程序与分时调度：任务管理、抢占式调度、时钟中断 | ✅ |
| ch4 | `ch4` | 地址空间：SV39 页表、MemorySet、内核/用户隔离 | ✅ |
| ch5 | `ch5` | 进程管理：fork / exec / waitpid、进程控制块 | ✅ |
| ch6 | `ch6` | 文件系统：inode、目录项、块缓存、简易 FS | ✅ |
| ch7 | `ch7` | 信号处理：sigaction / sigreturn | — |
| ch8 | `ch8` | 并发同步：互斥锁、信号量、条件变量（内核态） | ✅ |

---

## 环境准备

```bash
rustup toolchain install nightly-2024-05-02 --component rust-src,llvm-tools-preview,rustfmt,clippy
rustup target add riscv64gc-unknown-none-elf
sudo apt install qemu-system-misc
cargo install cargo-binutils --vers=0.3.3
```

---

## 快速开始

```bash
git clone https://cnb.cool/KenoCarte/rCore-Tutorial-2026S.git
cd rCore-Tutorial-2026S
git checkout ch$ID
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
cd os && make run
```

---

## 参考资料

| 资料 | 链接 |
|------|------|
| 精简手册 | https://LearningOS.github.io/rCore-Tutorial-Guide/ |
| 详细教程 | https://rcore-os.github.io/rCore-Tutorial-Book-v3/ |
| 训练营主页 | https://opencamp.cn/os2edu/camp/2026spring |
