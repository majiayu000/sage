#!/bin/bash
set -e

# ============================================================
# Sage Agent Installer
# https://github.com/majiayu000/sage
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/majiayu000/sage/main/install.sh | bash
#
# Options:
#   SAGE_VERSION    - Specific version to install (default: latest)
#   SAGE_INSTALL_DIR - Installation directory (default: ~/.local/bin)
#
# ============================================================

VERSION="${SAGE_VERSION:-latest}"
INSTALL_DIR="${SAGE_INSTALL_DIR:-$HOME/.local/bin}"
REPO="majiayu000/sage"
BINARY_NAME="sage"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ============================================================
# Helper Functions
# ============================================================

print_banner() {
    echo ""
    echo -e "${CYAN}${BOLD}"
    cat << 'EOF'
  ____
 / ___|  __ _  __ _  ___
 \___ \ / _` |/ _` |/ _ \
  ___) | (_| | (_| |  __/
 |____/ \__,_|\__, |\___|
              |___/
EOF
    echo -e "${NC}"
    echo -e "${BOLD}Blazing fast code agent in pure Rust${NC}"
    echo ""
}

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# ============================================================
# Platform Detection
# ============================================================

detect_platform() {
    local os arch

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64|amd64)
                    echo "x86_64-unknown-linux-gnu"
                    ;;
                aarch64|arm64)
                    echo "aarch64-unknown-linux-gnu"
                    ;;
                armv7l)
                    echo "armv7-unknown-linux-gnueabihf"
                    ;;
                *)
                    echo "unsupported"
                    ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64|amd64)
                    echo "x86_64-apple-darwin"
                    ;;
                arm64|aarch64)
                    echo "aarch64-apple-darwin"
                    ;;
                *)
                    echo "unsupported"
                    ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            case "$arch" in
                x86_64|amd64)
                    echo "x86_64-pc-windows-msvc"
                    ;;
                *)
                    echo "unsupported"
                    ;;
            esac
            ;;
        *)
            echo "unsupported"
            ;;
    esac
}

# ============================================================
# Version Management
# ============================================================

get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    if command -v curl &> /dev/null; then
        version=$(curl -sL "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget &> /dev/null; then
        version=$(wget -qO- "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "curl or wget is required"
    fi

    if [ -z "$version" ]; then
        error "Failed to fetch latest version. Please check your internet connection."
    fi

    echo "$version"
}

# ============================================================
# Download and Install
# ============================================================

download_and_install() {
    local platform=$1
    local version=$2

    if [ "$version" = "latest" ]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
    fi

    # Remove 'v' prefix if present for filename
    local version_num="${version#v}"
    local filename="${BINARY_NAME}-${version}-${platform}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${filename}"

    info "Downloading Sage ${version} for ${platform}..."

    # Create temp directory
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf '$tmp_dir'" EXIT

    # Download
    local download_path="$tmp_dir/sage.tar.gz"

    if command -v curl &> /dev/null; then
        if ! curl -fsSL "$url" -o "$download_path" 2>/dev/null; then
            # Try alternative naming convention
            filename="${BINARY_NAME}-v${version_num}-${platform}.tar.gz"
            url="https://github.com/${REPO}/releases/download/${version}/${filename}"
            curl -fsSL "$url" -o "$download_path" || error "Download failed. URL: $url"
        fi
    elif command -v wget &> /dev/null; then
        wget -q "$url" -O "$download_path" || error "Download failed. URL: $url"
    else
        error "curl or wget is required"
    fi

    success "Downloaded successfully"

    # Extract
    info "Extracting..."
    tar -xzf "$download_path" -C "$tmp_dir" || error "Extraction failed"

    # Find binary (might be in subdirectory)
    local binary_path
    binary_path=$(find "$tmp_dir" -name "$BINARY_NAME" -type f -perm -u+x 2>/dev/null | head -1)

    if [ -z "$binary_path" ]; then
        binary_path=$(find "$tmp_dir" -name "$BINARY_NAME" -type f 2>/dev/null | head -1)
    fi

    if [ -z "$binary_path" ]; then
        error "Binary not found in archive"
    fi

    # Install
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    mv "$binary_path" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "Installed successfully!"
}

# ============================================================
# PATH Setup
# ============================================================

setup_path() {
    local shell_rc=""
    local shell_name=""

    # Detect shell config file
    if [ -n "$ZSH_VERSION" ] || [ "$SHELL" = "$(which zsh 2>/dev/null)" ]; then
        shell_rc="$HOME/.zshrc"
        shell_name="zsh"
    elif [ -n "$BASH_VERSION" ] || [ "$SHELL" = "$(which bash 2>/dev/null)" ]; then
        if [ -f "$HOME/.bashrc" ]; then
            shell_rc="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            shell_rc="$HOME/.bash_profile"
        fi
        shell_name="bash"
    elif [ -f "$HOME/.profile" ]; then
        shell_rc="$HOME/.profile"
        shell_name="sh"
    fi

    # Check if already in PATH
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return 0
    fi

    # Add to shell config if found
    if [ -n "$shell_rc" ]; then
        if ! grep -q "$INSTALL_DIR" "$shell_rc" 2>/dev/null; then
            echo "" >> "$shell_rc"
            echo "# Sage Agent" >> "$shell_rc"
            echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$shell_rc"
            warn "Added ${INSTALL_DIR} to PATH in ${shell_rc}"
            echo ""
            echo -e "  ${YELLOW}Run this to use sage immediately:${NC}"
            echo -e "  ${CYAN}source ${shell_rc}${NC}"
            echo ""
            echo -e "  ${YELLOW}Or restart your terminal.${NC}"
        fi
    else
        warn "Could not detect shell config file."
        echo ""
        echo -e "  ${YELLOW}Add this to your shell config:${NC}"
        echo -e "  ${CYAN}export PATH=\"\$PATH:$INSTALL_DIR\"${NC}"
    fi
}

# ============================================================
# Verify Installation
# ============================================================

verify_installation() {
    local sage_path="$INSTALL_DIR/$BINARY_NAME"

    if [ -x "$sage_path" ]; then
        echo ""
        echo -e "${GREEN}${BOLD}Installation complete!${NC}"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo -e "  ${BOLD}Get started:${NC}"
        echo ""
        echo -e "    ${CYAN}sage --help${NC}              Show help"
        echo -e "    ${CYAN}sage interactive${NC}         Start interactive mode"
        echo -e "    ${CYAN}sage \"Your task\"${NC}         Run a one-shot task"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo -e "  ${BOLD}Documentation:${NC} ${BLUE}https://github.com/${REPO}${NC}"
        echo ""

        # Show version if possible
        if command -v "$sage_path" &> /dev/null || [ -x "$sage_path" ]; then
            echo -e "  ${BOLD}Installed version:${NC} $($sage_path --version 2>/dev/null || echo 'unknown')"
            echo ""
        fi
    else
        error "Installation verification failed. Binary not found at $sage_path"
    fi
}

# ============================================================
# Build from Source (Fallback)
# ============================================================

build_from_source() {
    info "Building from source..."

    if ! command -v cargo &> /dev/null; then
        error "Rust is not installed. Please install Rust first: https://rustup.rs"
    fi

    if ! command -v git &> /dev/null; then
        error "Git is not installed"
    fi

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf '$tmp_dir'" EXIT

    info "Cloning repository..."
    git clone --depth 1 "https://github.com/${REPO}.git" "$tmp_dir/sage"

    info "Building (this may take a few minutes)..."
    cd "$tmp_dir/sage"
    cargo build --release

    info "Installing..."
    mkdir -p "$INSTALL_DIR"
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "Built and installed successfully!"
}

# ============================================================
# Main
# ============================================================

main() {
    print_banner

    # Check for help flag
    if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
        echo "Usage: install.sh [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --source       Build from source instead of downloading binary"
        echo ""
        echo "Environment variables:"
        echo "  SAGE_VERSION      Version to install (default: latest)"
        echo "  SAGE_INSTALL_DIR  Installation directory (default: ~/.local/bin)"
        echo ""
        exit 0
    fi

    # Check for source build flag
    if [ "$1" = "--source" ]; then
        build_from_source
        setup_path
        verify_installation
        exit 0
    fi

    # Detect platform
    local platform
    platform=$(detect_platform)

    if [ "$platform" = "unsupported" ]; then
        warn "Pre-built binary not available for $(uname -s)/$(uname -m)"
        echo ""
        echo -e "  ${YELLOW}Attempting to build from source...${NC}"
        echo ""
        build_from_source
        setup_path
        verify_installation
        exit 0
    fi

    info "Detected platform: ${platform}"

    # Download and install
    download_and_install "$platform" "$VERSION"
    setup_path
    verify_installation
}

main "$@"
