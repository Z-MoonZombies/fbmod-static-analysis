# v0.1.0 — FBMOD 静态分析

本版本汇总对 FBMOD 解包工具和两个样本的只读分析，并附上按二进制行为整理的 Rust 重建稿。

## 内容

- FBMOD v5 文件头、资源记录和 manifest 布局。
- CAS 块头与 selector 0、2、9、15 的解码分支。
- `4.4.4.fbmod` 中 RES、Chunk 记录及 selector 9 块的核验结果。
- Rust 重建代码、Python 核验脚本、反汇编摘录和复现步骤。

## 验证范围

Rust 代码通过格式检查、`cargo check --offline` 和 Clippy 检查；没有自动化测试。selector 9 样本块已核对 framing 和声明输出长度，实际解压结果尚未验证。两个样本都没有 selector 2、selector 15 或 Bundle 数据，FBProject 的完整资源结构也未恢复。

v0.1.0 发布包没有包含原始 EXE 和 FBMOD 样本。它们现已收录在仓库 `inputs/` 目录，并按 MIT 许可提供；v0.1.1 发布包会包含这三个文件。该许可不适用于分析报告和重建代码。
