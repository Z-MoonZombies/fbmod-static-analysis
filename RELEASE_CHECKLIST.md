# 发布检查清单

此清单跟踪当前候选目录到正式 GitHub Release 之间的未完成事项。

- [x] 记录发布者偏好：只供阅读，不授予通用复用许可；不附加开源许可证。
- [x] 发布者已明确授权公开展示分析报告与重建代码，并允许查看和 fork；候选包不含原始 EXE 或 FBMOD。
- [x] 使用 Rust 工具链完成 `cargo check --offline`，并记录编译器、依赖锁文件和结果，详见 `VALIDATION.md`。
- [x] 运行 `cargo clippy --locked --offline -- -D warnings`，确认没有 Clippy 警告。
- [x] 复核报告中的推断标记、引用链接与证据文件路径。
- [x] 核对候选档案不包含原始 EXE、FBMOD 样本或用户本机路径。
- [x] 研究草稿已提交到公开仓库 `main` 分支。
- [ ] 创建正式版本标签和 GitHub Release。
- [x] 发布说明写明已验证范围、未验证的压缩分支与已知限制。
- [ ] （可选）上传独立候选源码 ZIP 和 SHA-256；GitHub 自动源码归档已可下载。
