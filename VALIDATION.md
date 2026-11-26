# 验证记录

## 已完成

- 用候选目录中的 `analyze_static.py` 对用户提供的 EXE 与 FBMOD 附件重新进行只读核验。程序只读取文件字节，没有启动目标 EXE。
- 核验结果与报告一致：EXE SHA-256 为 `6CE6E746A55E7C43396D105AD5F454C9486CDFBBEBCDC3FD4581327838F747E3`；FBMOD SHA-256 为 `BBCBE54F2C937C649E65DE06463C9CBBCC179A734C164FB4718659BB5B79DF9B`。
- 样本结果为 FBMOD v5、manifest 偏移 `0x884`、21 个 manifest 项、25 条资源记录（5 个 embedded、20 个 EBX）。完整输出在 `evidence/verification_run.txt`。
- 补充样本 `4.4.4.fbmod` 的 SHA-256 为 `E13F65B8EC36B1047B071AFA1F42931990D451C08F007C8D7A87FF17A4402188`；只记录结构摘要、不复制原件，见 `evidence/secondary_sample_audit.txt`。
- 补充样本为 v5，373 条记录正好结束于 `data_offset=0x94AA`，有 5 embedded、360 EBX、4 RES、4 Chunk、0 Bundle；manifest 到文件末尾无尾随字节。
- 补充样本的 4 条 RES 与 4 条 Chunk 记录 SHA-1 均匹配各自 manifest 字节；其 CAS framing 共 433 个 selector 0 块和 65 个 selector 9 块，所有声明输出长度总和与记录逻辑大小一致。
- 新增 `decode_res_resource` 和 `decode_chunk_resource`，按记录各自的逻辑长度读取并解码 CAS；Chunk 长度取自类型专属尾部。当前通过编译与 Clippy 检查，未在补充样本上调用 Rust 解码器。
- 检查了分析报告中的本地链接，目标文件均存在。
- 复核了候选 Markdown 的本地链接：共 34 处，未发现缺失目标；核对了 Frosty、flate2 和 GitHub 文档的外部引用页面。
- 检查候选目录未包含 `.exe`、`.fbmod` 或 `.bin` 输入资产；审计副本中的本机附件路径已替换为通用文件名。
- 已为候选 ZIP 计算 SHA-256，旁边的 `.sha256` 文件记录了摘要。
- 代码审查改进了 `data_offset` 边界处理、异常计数检查、可失败内存预留以及资源记录到 manifest 数据的访问器。
- 增加 EBX、RES、Chunk 的 CAS 资源辅助函数，把 manifest 数据、CAS 解码和各自逻辑大小校验接成调用路径；已通过编译检查，但未在补充样本上调用 Rust 解码器。
- Zstandard 上下文改用 `DCtx::try_create`，以显式处理上下文分配失败；该 API 在 [zstd-safe 7.2.4 文档](https://docs.rs/zstd-safe/7.2.4/zstd_safe/struct.DCtx.html)中有说明。
- 已安装并识别 `rustc 1.100.0`、`cargo 1.100.0` 和 `rustfmt 1.11.0`；`rustfmt --check reconstructed_core.rs` 通过。
- `cargo metadata --no-deps --format-version 1` 成功解析候选包清单，目标文件和 `flate2`、`zstd-safe` 依赖均已识别。
- 用户在本机 PowerShell 成功下载了锁定依赖；编译首次发现 `zstd-safe 7.2.4` 的 `DCtx::decompress` 接受字节切片而不是 `Vec`。已将调用改为 `output.as_mut_slice()`。
- 修正后使用 `cargo check --offline` 编译成功，使用上述 Rust 工具链版本；`Cargo.lock` 已纳入候选包。
- `cargo clippy --locked --offline -- -D warnings` 通过，没有 Clippy 警告。

## 尚未完成

- 没有添加或运行自动化测试。编译成功只确认类型和接口可通过 Rust 编译器检查；selector `9` 的真实压缩输出尚未通过 Rust 解码器核对，selector `2` 和 `15` 在两个样本中都没有出现。
- 发布者选择只供阅读，不附加开源许可证；公开展示报告和重建代码的权利仍需确认。
