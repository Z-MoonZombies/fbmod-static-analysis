# v0.1.0 — 草稿

**研究草稿已公开于 GitHub；尚未创建正式 v0.1.0 Release。** 此草稿汇总当前只读静态分析阶段的内容。

## 包含

- FBMOD v5 头部、资源记录和 manifest 布局分析。
- CAS 块头及 selector 0、2、9、15 的静态解码路径说明。
- 第二个 FBMOD 样本对 RES/Chunk 尾部和 65 个 selector 9 块提供字节级 framing 对照。
- Rust 风格核心增加 RES 和 Chunk CAS 提取辅助函数。
- Python 核验脚本、RVA 反汇编脚本、Rust 风格核心重建和关键反汇编证据。
- 文件哈希、复现步骤和限制说明。

## 验证状态

- 原始 EXE 没有运行。
- Bundle 记录、selector 2/15 块没有样本；selector 9 有样本块，但尚未端到端解码验证。
- 使用 `rustc 1.100.0` 完成格式检查、`cargo check --offline` 和 Clippy 检查；尚未运行自动化测试，见 `VALIDATION.md`。
- 本草稿没有声明重建代码通过测试。
- 按发布者要求，此包只供阅读，不附加开源许可证或复用授权。
- 公开 GitHub 仓库允许用户查看和 fork；若要阻止 fork，应使用私有仓库。

## 后续事项

- 如需正式发行，创建 v0.1.0 标签和 GitHub Release；当前公开仓库已可浏览和 fork。
- GitHub 自动生成的源码归档可直接下载；若要额外提供独立附件，再上传候选源码 ZIP 与 SHA-256。
- 已知的解压分支和结构限制继续列在本说明的“验证状态”及 `analysis_report.md` 中。
