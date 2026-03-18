FROM ubuntu:24.04

# Set non-interactive mode
ENV DEBIAN_FRONTEND=noninteractive

# Update and install essential tools
RUN apt-get update && apt-get install -y \
    git \
    ssh \
    curl \
    wget \
    sudo \
    build-essential \
    zsh \
    vim \
    tmux \
    unzip \
    zip \
    net-tools \
    iputils-ping \
    software-properties-common \
    qemu-system-riscv64 \
    qemu-utils \
    gcc-riscv64-linux-gnu \
    gdb-multiarch \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to PATH
ENV PATH="$HOME/.cargo/bin:$PATH"

# Install Rust target for RISC-V
RUN rustup target add riscv64gc-unknown-none-elf

# Install cargo-binutils
RUN cargo install cargo-binutils

# Install llvm-tools-preview
RUN rustup component add llvm-tools-preview

# Install Oh My Zsh
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

# Install zsh plugins
RUN git clone https://github.com/zsh-users/zsh-syntax-highlighting.git ~/.oh-my-zsh/custom/plugins/zsh-syntax-highlighting && \
    git clone https://github.com/zsh-users/zsh-autosuggestions.git ~/.oh-my-zsh/custom/plugins/zsh-autosuggestions

# Install code-server and plugins
RUN curl -fsSL https://code-server.dev/install.sh | sh && \
    code-server --install-extension cnbcool.cnb-welcome && \
    code-server --install-extension redhat.vscode-yaml && \
    code-server --install-extension waderyan.gitblame && \
    code-server --install-extension mhutchie.git-graph && \
    code-server --install-extension donjayamanne.githistory && \
    code-server --install-extension cloudstudio.live-server && \
    code-server --install-extension tencent-cloud.coding-copilot && \
    code-server --install-extension rust-lang.rust-analyzer

# Set zsh as default shell
RUN chsh -s /bin/zsh

# Expose code-server port
EXPOSE 8080

# Start code-server
CMD ["code-server", "--bind-addr", "0.0.0.0:8080", "--auth", "none"]