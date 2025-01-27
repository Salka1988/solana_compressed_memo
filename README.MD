# Prerequisites for Running Tests on macOS

To run tests on macOS, follow these setup steps:

## 1. Install Solana CLI Tools

Install the required version of Solana CLI tools (e.g., `v1.18.22`):

```bash
sh -c "$(curl -sSfL https://release.solana.com/v1.18.22/install)"
```

## 2. Set Environment Variable for macOS File Copying

Disable resource forks to ensure compatibility:

```bash
export COPYFILE_DISABLE=1
```

## 3. Ensure Cargo.lock Version

Ensure your Cargo.lock file uses version 3.

## 4. Set Compiler for Rust on macOS

```bash
export CC=/usr/bin/clang
export CFLAGS="-isysroot $(xcrun --sdk macosx --show-sdk-path)"
```
## 5. Install Node.js Package Manager (npm)

```bash
curl -qL https://www.npmjs.com/install.sh | sh
```