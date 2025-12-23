# Installation Guide / 安装指南

## System Requirements / 系统要求

**Minimum Requirements / 最低要求:**
- Rust 1.85 or higher / Rust 1.85 或更高版本
- Operating System / 操作系统:
  - Linux (Ubuntu 20.04+, Debian 11+, etc.)
  - macOS 12.0+ (Monterey or later)
  - Windows 10/11 (WSL2 recommended)

**Required Tools / 必需工具:**
- Git
- Cargo (Rust package manager / Rust 包管理器)

---

## Installation Methods / 安装方式

### Method 1: Install from Source (Recommended) / 方法一:从源码安装(推荐)

**Step 1: Clone the repository / 克隆仓库**
```bash
git clone https://github.com/majiayu000/sage
cd sage
```

**Step 2: Build and install / 构建并安装**
```bash
# Install the CLI globally
# 全局安装 CLI
cargo install --path crates/sage-cli

# Or use the Makefile shortcut
# 或使用 Makefile 快捷命令
make install
```

**Step 3: Verify installation / 验证安装**
```bash
sage --version
# Should output: sage 0.1.0
# 应输出: sage 0.1.0
```

---

### Method 2: Build Without Installing / 方法二:仅构建不安装

If you want to try Sage without installing it globally:
如果您想在不全局安装的情况下试用 Sage:

```bash
# Build debug version
# 构建调试版本
cargo build

# Build optimized release version
# 构建优化的发布版本
cargo build --release

# Run from build directory
# 从构建目录运行
./target/release/sage --version
```

---

### Method 3: Development Installation / 方法三:开发者安装

For development and contributing:
用于开发和贡献:

```bash
# Clone with full git history
# 克隆完整 git 历史
git clone https://github.com/majiayu000/sage
cd sage

# Install development dependencies
# 安装开发依赖
rustup component add rustfmt clippy

# Build in development mode
# 以开发模式构建
make dev

# Run tests to verify setup
# 运行测试以验证设置
make test
```

---

## Post-Installation Setup / 安装后设置

### Create Configuration Directory / 创建配置目录

```bash
# Create Sage configuration directory
# 创建 Sage 配置目录
mkdir -p ~/.config/sage

# Optional: Initialize .sage directory in your project
# 可选:在您的项目中初始化 .sage 目录
cd /path/to/your/project
mkdir -p .sage
```

### Initialize Configuration / 初始化配置

```bash
# Generate default configuration file
# 生成默认配置文件
sage config init

# Or create in a specific location
# 或在指定位置创建
sage config init --config-file ~/.config/sage/config.json
```

---

## Verify Installation / 验证安装

After installation, verify that Sage is working correctly:
安装后,验证 Sage 是否正常工作:

```bash
# Check version
# 检查版本
sage --version

# View available tools
# 查看可用工具
sage tools

# Run a simple test
# 运行简单测试
sage run "echo Hello from Sage"
```

---

## Update / Upgrade / 更新/升级

To update Sage to the latest version:
更新 Sage 到最新版本:

```bash
# Navigate to Sage directory
# 进入 Sage 目录
cd /path/to/sage

# Pull latest changes
# 拉取最新更改
git pull origin main

# Rebuild and reinstall
# 重新构建并安装
cargo install --path crates/sage-cli --force
```

---

## Troubleshooting / 故障排除

### Common Issues / 常见问题

**1. Rust version too old / Rust 版本过旧**
```bash
# Update Rust to latest version
# 更新 Rust 到最新版本
rustup update stable
```

**2. Permission denied when installing / 安装时权限被拒绝**
```bash
# Make sure cargo bin directory is in PATH
# 确保 cargo bin 目录在 PATH 中
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**3. Build fails with linker errors / 构建失败,链接器错误**
```bash
# On Ubuntu/Debian, install build essentials
# 在 Ubuntu/Debian 上,安装构建基础工具
sudo apt-get install build-essential pkg-config libssl-dev

# On macOS, install Xcode Command Line Tools
# 在 macOS 上,安装 Xcode 命令行工具
xcode-select --install
```

**4. Command not found after installation / 安装后命令未找到**
```bash
# Verify cargo bin directory exists in PATH
# 验证 cargo bin 目录在 PATH 中
echo $PATH | grep -q "$HOME/.cargo/bin" && echo "Found" || echo "Not found"

# Add to PATH if missing
# 如果缺失,添加到 PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

---

## Uninstallation / 卸载

To remove Sage from your system:
从系统中移除 Sage:

```bash
# Uninstall the CLI
# 卸载 CLI
cargo uninstall sage

# Remove configuration (optional)
# 删除配置(可选)
rm -rf ~/.config/sage
rm -rf ~/.sage
```

---

## Next Steps / 下一步

After successful installation:
安装成功后:

1. **Configure API Keys** → See [Configuration Guide](configuration.md)
   **配置 API 密钥** → 查看 [配置指南](configuration.md)

2. **Quick Start** → See [Quick Start Guide](quick-start.md)
   **快速开始** → 查看 [快速入门指南](quick-start.md)

3. **Explore Examples** → Run examples with `make examples`
   **探索示例** → 使用 `make examples` 运行示例

---

## Additional Resources / 其他资源

- **GitHub Repository**: https://github.com/majiayu000/sage
- **Issue Tracker**: https://github.com/majiayu000/sage/issues
- **Documentation**: [/docs](/docs)
