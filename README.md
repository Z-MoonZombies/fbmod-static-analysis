# FBMOD 静态分析与重建

本仓库整理了对 FBMOD 解包工具的只读静态分析，以及两个 FBMOD 样本的结构核验。Rust 文件是按二进制可观察行为编写的重建稿，不是原工具源码。原始 EXE 和两个 FBMOD 样本放在 [inputs/](inputs/) 中；EXE 只作为数据检查，没有运行。

## v0.1.1 下载

- [独立 ZIP](https://github.com/Z-MoonZombies/fbmod-static-analysis/releases/download/v0.1.1/fbmod-static-analysis-v0.1.1.zip)
- [SHA-256 文件](https://github.com/Z-MoonZombies/fbmod-static-analysis/releases/download/v0.1.1/fbmod-static-analysis-v0.1.1.zip.sha256)

v0.1.1 的 ZIP 包含原始 EXE 和两个 FBMOD 样本。v0.1.0 的已发布附件保持不变。

## 已核对的内容

- FBMOD v5 文件头、资源记录和 manifest 布局；Anti Chomper 样本有 25 条资源记录，manifest 从 `0x884` 开始。
- `4.4.4.fbmod` 有 373 条记录，包括 5 个 embedded、360 个 EBX、4 个 RES 和 4 个 Chunk；没有 Bundle。RES 和 Chunk 的 SHA-1 与对应 manifest 数据一致。
- CAS selector 0、2、9、15 的解码分支。样本中的 selector 9 块已核对 framing 和声明输出长度，实际解压字节还没有验证。

## 文件

- [分析报告](analysis_report.md)：结构推断、反汇编证据和复现步骤。
- [Rust 重建稿](reconstructed_core.rs)：文件解析、资源记录和 CAS 解码逻辑。
- [静态核验脚本](analyze_static.py)：使用 Python 标准库检查 PE 和 FBMOD 文件。
- [PE 反汇编脚本](pe_dumpbin_range.ps1)：截取指定 RVA 范围；也可用可选的 Capstone 脚本 `pe_disasm_range.py`。
- [证据目录](evidence/)：反汇编摘录和样本核验记录。
- [原始分析文件](inputs/)：反编译器 EXE 和两个 FBMOD 样本，均按 MIT 许可提供，见 [许可文件](inputs/LICENSE)。

## 检查与限制

Rust 代码使用 rustc 1.100.0 通过格式检查、`cargo check --offline` 和 Clippy 检查。没有自动化测试，Rust 解码器也尚未在样本上端到端验证。

目前没有 selector 2、selector 15 或 Bundle 样本；FBProject 的完整资源条目结构仍未恢复。详细记录见 [验证说明](VALIDATION.md)。

## 复现

使用仓库内的文件检查样本：

```text
python analyze_static.py inputs/fbmod-decompiler.exe "inputs/Anti Chomper V1.0.0.fbmod"
```

Windows 上可用 Visual Studio 的 dumpbin 截取 PE 反汇编：

```powershell
.\pe_dumpbin_range.ps1 -PePath "inputs\fbmod-decompiler.exe" -StartRva 0x24080 -EndRva 0x255A5
```

## 使用范围

分析报告和重建代码仅供阅读，本仓库没有授予其通用复用、修改或再发布许可。MIT 许可只覆盖 `inputs/` 中列出的三个原始文件。详情见 [许可说明](LICENSE_STATUS.md)。
