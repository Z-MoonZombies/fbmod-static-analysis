# 验证记录

## 样本

以下文件已收录在仓库的 `inputs/` 目录。EXE 只作为数据检查，没有运行。

| 文件 | 大小 | SHA-256 |
|---|---:|---|
| `inputs/fbmod-decompiler.exe` | 1,464,320 字节 | `6CE6E746A55E7C43396D105AD5F454C9486CDFBBEBCDC3FD4581327838F747E3` |
| `inputs/Anti Chomper V1.0.0.fbmod` | 1,191,072 字节 | `BBCBE54F2C937C649E65DE06463C9CBBCC179A734C164FB4718659BB5B79DF9B` |
| `inputs/4.4.4.fbmod` | 15,095,761 字节 | `E13F65B8EC36B1047B071AFA1F42931990D451C08F007C8D7A87FF17A4402188` |

Anti Chomper 样本为 v5，含 25 条资源记录和 21 个 manifest 项，manifest 偏移为 `0x884`。补充样本为 v5，含 373 条记录：5 个 embedded、360 个 EBX、4 个 RES、4 个 Chunk，没有 Bundle。4 条 RES 和 4 条 Chunk 的 SHA-1 均与各自 manifest 数据一致。逐项记录见 `evidence/`。

## Rust 检查

使用 rustc 1.100.0、cargo 1.100.0 和 rustfmt 1.11.0 完成：

- `rustfmt --check reconstructed_core.rs`
- `cargo check --offline`
- `cargo clippy --locked --offline -- -D warnings`

三项检查均通过。没有自动化测试；也没有在补充样本上调用 Rust 的 RES、Chunk 提取函数。

## 尚未验证

- 65 个 selector 9 样本块的 framing 和声明输出长度已核对，实际解压字节未核对。
- 两个样本都没有 selector 2、selector 15 或 Bundle 数据。
- FBProject 的完整资源条目结构尚未恢复。
