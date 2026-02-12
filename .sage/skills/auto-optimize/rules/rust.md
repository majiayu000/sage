# Rust NEVER 规则

项目标识: `Cargo.toml` 存在
文件扩展名: `*.rs`
Lint 命令: `cargo clippy --all-targets -- -D warnings`
测试命令: `cargo test --lib`
格式化命令: `cargo fmt --check`

## 规则

- **RS-01**: NEVER use `as` for narrowing integer casts — use `TryFrom`/`try_from`
  - 检测模式: `" as i8\| as i16\| as i32\| as u8\| as u16\| as u32"` (排除 test、注释)

- **RS-02**: NEVER use `as` to convert f64 to integer — validate `is_finite()` + range first
  - 检测模式: f64 变量后跟 `as i64\|as u64\|as i32\|as u32`

- **RS-03**: NEVER use `Vec::remove(0)` in loops — use `VecDeque`
  - 检测模式: `"Vec::remove(0)\|\.remove(0)"`

- **RS-04**: NEVER call `tokio::spawn` without storing JoinHandle — store and abort on Drop
  - 检测模式: `"tokio::spawn"` 所在行无 `let`/`handle`/`join` 赋值

- **RS-05**: NEVER use `std::mem::forget` for lifetime extension — use Arc/ownership
  - 检测模式: `"mem::forget"`

- **RS-06**: NEVER unwrap() on RwLock/Mutex lock operations — use poison recovery
  - 检测模式: `"read().unwrap\|write().unwrap\|lock().unwrap"` (排除 test)

- **RS-07**: NEVER use `.unwrap()` in production code paths — use `?` or `unwrap_or`
  - 检测模式: `"\.unwrap()"` (排除 test/example)
  - 注意: 这是渐进式规则，优先修复 IO/网络/锁相关路径

- **RS-08**: NEVER use `String::remove(0)` in a loop — use `drain(..n)`
  - 检测模式: `"String::remove(0)\|\.remove(0)"` 在 String 上下文

- **RS-09**: NEVER create identity type aliases (`type Foo = Foo`) — use re-exports
  - 检测模式: `"type .* = .*"` 左右相同
