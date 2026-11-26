# FBMOD 静态分析与重建

**状态：v0.1.0 研究稿 Release 已发布。** 本项目记录对一个 Rust 编译的 FBMOD 解包工具和两个 FBMOD 样本所做的只读静态分析，并提供可读的 Rust 风格重建代码。它不是原工具的源代码，也不是可替代原工具的完整程序。

## 范围

- 复核 PE 与 FBMOD 样本的大小、SHA-256 和关键头部字段。
- 分析 FBMOD v5 资源记录、`0x884` manifest、CAS 块头和解码分支；第二个样本另含 RES、Chunk 和 LZ4 selector 9 数据。
- 提供标准库 Python 文件检查脚本、PE RVA 反汇编脚本和 Rust 风格核心重建。
- 记录证据、推断和目前无法端到端验证的部分。

没有把原始 EXE 或 FBMOD 样本复制到本项目。分析脚本需要用户自行提供有权检查的文件。目标 EXE 只作为静态字节读取，仓库中的步骤不会启动它。

## 文件

- `analysis_report.md`：完整分析报告。
- `reconstructed_core.rs`：FBMOD 头、资源记录、manifest、EBX/RES/Chunk 提取和 CAS 解码逻辑的可读重建。
- `analyze_static.py`：使用 Python 标准库计算哈希并检查 PE/FBMOD 结构。
- `pe_dumpbin_range.ps1`：调用 Visual Studio `dumpbin`，截取 PE RVA 范围的静态反汇编。
- `pe_disasm_range.py`：可选的 Capstone RVA 反汇编器。
- `evidence/`：反汇编摘录与脱敏后的文件核验记录。
- `Cargo.toml`：重建代码使用的依赖版本。
- `Cargo.lock`：已验证构建使用的依赖解析结果。

## 复现

使用 Python 3 检查本地输入文件：

```text
python analyze_static.py /path/to/fbmod-decompiler.exe "/path/to/Anti Chomper V1.0.0.fbmod"
```

Windows 上截取 CAS 解码器的反汇编范围：

```powershell
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x24080 -EndRva 0x255A5
```

脚本会查找 `dumpbin.exe`；也可以通过 `-DumpbinPath` 显式传入路径。可选 Python 反汇编脚本需要 Capstone：

```text
python -m pip install capstone
python pe_disasm_range.py /path/to/fbmod-decompiler.exe 0x24080 0x255A5
```

`reconstructed_core.rs` 是供阅读的重建代码，不是原工具或可直接替代原工具的程序。使用 `rustc 1.100.0` 完成了格式检查、`cargo check --offline` 编译检查和 Clippy 检查；尚未运行自动化测试。详细验证记录见 `VALIDATION.md`。

Rust API 的预期调用顺序如下。此示例说明接口关系，并已通过 `cargo check --offline` 编译检查。

```rust
use fbmod_reconstruction::{decode_ebx_resource, parse_fbmod_v5, ParseError};

fn decode_first_ebx(bytes: &[u8]) -> Result<Vec<u8>, ParseError> {
    let (header, resources) = parse_fbmod_v5(bytes)?;
    let ebx = resources
        .iter()
        .find(|resource| resource.kind == 1)
        .ok_or(ParseError::InvalidIndex)?;
    decode_ebx_resource(bytes, &header, ebx)
}
```

## 已知限制

- `Anti Chomper` 样本只有资源类型 `0` 和 `1`；第二个样本含 RES 和 Chunk 记录，可对照这两种尾部。两个样本都没有 Bundle 记录。
- 第二个样本含 65 个 LZ4 selector `9` 块，目前核对了块头 framing 和声明输出长度，尚未验证实际解码字节。两个样本都没有 selector `2` 或 `15` 块。
- FBProject 输出容器的头部顺序已部分恢复，所有资源条目的完整 schema 尚未确定。
- 原始标识符、错误类型和编译器生成细节无法从当前二进制中完整恢复。

## 阅读范围与许可

此候选包按“只供阅读”整理，不授予重建代码或报告的复用、修改或再分发许可；没有附加开源许可证。详见 `VIEWING_ONLY_NOTICE.md` 和 `LICENSE_STATUS.md`。

GitHub 对公开仓库适用的平台条款允许用户查看和 fork。若“只供阅读”也意味着不允许 GitHub 内 fork，就需要改用私有仓库；该限制不能仅靠本仓库的说明实现。见 [GitHub 许可证说明](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)。

## 发布状态

当前仓库已发布 v0.1.0 标签与 GitHub Release；后续工作聚焦于已知分析限制。详见 `RELEASE_CHECKLIST.md` 和 `RELEASE_NOTES_v0.1.0-DRAFT.md`。
