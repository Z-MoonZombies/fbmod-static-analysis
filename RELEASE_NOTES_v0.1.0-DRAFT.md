# v0.1.0 — FBMOD 静态分析

本版本汇总对 FBMOD 解包工具和两个样本的只读分析，并附上按二进制行为整理的 Rust 重建稿。

## 内容

- FBMOD v5 文件头、资源记录和 manifest 布局。
- CAS 块头与 selector 0、2、9、15 的解码分支。
- `4.4.4.fbmod` 中 RES、Chunk 记录及 selector 9 块的核验结果。
- Rust 重建代码、Python 核验脚本、反汇编摘录和复现步骤。

## 验证范围

Rust 代码通过格式检查、`cargo check --offline` 和 Clippy 检查；没有自动化测试。selector 9 样本块已核对 framing 和声明输出长度，实际解压结果尚未验证。两个样本都没有 selector 2、selector 15 或 Bundle 数据，FBProject 的完整资源结构也未恢复。

原始 EXE 只读取字节，没有运行。EXE 和两个 FBMOD 样本现在收录在仓库 `main` 分支的 `inputs/` 目录；它们不在 v0.1.0 独立 ZIP 中。仓库供公开查看和 fork，不附开源许可证，也不授予分析报告和重建代码的通用复用许可。
