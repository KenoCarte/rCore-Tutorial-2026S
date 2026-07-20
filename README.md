# rCore-Tutorial-2026S

> **KenoCarte** 的 rCore-Tutorial 实验仓库 | OS2EDU 2026 春夏季开源操作系统训练营 专业阶段

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

### 必需工具

```bash
# Rust nightly 工具链（rust-toolchain.toml 已锁定版本）
rustup toolchain install nightly-2024-05-02 --component rust-src,llvm-tools-preview,rustfmt,clippy

# RISC-V 编译目标
rustup target add riscv64gc-unknown-none-elf

# QEMU（≥6.0）
sudo apt install qemu-system-misc

# cargo-binutils（objcopy 等）
cargo install cargo-binutils --vers=0.3.3
```

### 验证安装

```bash
qemu-system-riscv64 --version
rustup show active-toolchain   # 在仓库目录下应为 nightly-2024-05-02
rust-objcopy --version
```

---

## 快速开始

```bash
# 克隆仓库
git clone https://cnb.cool/[YOUR_USERNAME]/rCore-Tutorial-2026S.git
cd rCore-Tutorial-2026S

# 切换到指定章节
git checkout ch1     # 以 ch1 为例

# 编译并运行
cd os && make run

# 退出 QEMU：按 Ctrl+A 再按 X
```

---

## 评测

ch3、ch4、ch5、ch6、ch8 需要提交 CNB Pull Request 自动评测。详细流程见原仓库 [README](https://cnb.cool/LearningOS/OSCamp-2026S/rCore-Tutorial-2026S)。

---

## 参考资料

| 资料 | 链接 |
|------|------|
| 精简手册（实验要求） | https://LearningOS.github.io/rCore-Tutorial-Guide/ |
| 详细教程（原理 + 代码） | https://rcore-os.github.io/rCore-Tutorial-Book-v3/ |
| API 文档 | https://learningos.github.io/rCore-Tutorial-Code/ |
| 训练营主页 | https://opencamp.cn/os2edu/camp/2026spring |
