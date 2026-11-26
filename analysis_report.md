# FBMOD 反编译器静态分析报告

首次整理：2026-11-14；本轮更新：2026-11-16  
范围：只读静态分析 `fbmod-decompiler.exe`、`Anti Chomper V1.0.0.fbmod` 和补充样本 `4.4.4.fbmod`。没有运行未知 EXE，也没有修改输入文件。

## 1. 文件身份核验

| 文件 | 大小 | SHA-256 |
|---|---:|---|
| `fbmod-decompiler.exe` | 1,464,320 字节（0x165800） | `6CE6E746A55E7C43396D105AD5F454C9486CDFBBEBCDC3FD4581327838F747E3` |
| `Anti Chomper V1.0.0.fbmod` | 1,191,072 字节（0x122CA0） | `BBCBE54F2C937C649E65DE06463C9CBBCC179A734C164FB4718659BB5B79DF9B` |
| `4.4.4.fbmod` | 15,095,761 字节（0xE657D1） | `E13F65B8EC36B1047B071AFA1F42931990D451C08F007C8D7A87FF17A4402188` |

摘要：旧报告中的第一组文件哈希与大小和本次读取一致，但旧报告把 profile 误读成 `GW2.Main_Win64_Retail~7`，并把资源数写成 20。`Anti Chomper` 样本实际为 profile `GW2.Main_Win64_Retail`、25 条资源记录、21 个清单项；`~7` 是紧随 profile 的 `game_version=407422` 字段字节，不属于 profile。补充的 `4.4.4.fbmod` 为 v5，包含 373 条资源记录和 373 个清单项，可交叉核验 RES、Chunk 记录以及 selector 9 的 CAS framing。

## 2. FBMOD 文件头与偏移

本样本的头部字段可按以下偏移复现。多字节数值为 little-endian，字符串为 UTF-8、以 NUL 结尾。

| 偏移 | 字段 | 本样本值 |
|---:|---|---|
| `0x00` | 8 字节 magic | `FROSTY 00 01` |
| `0x08` | `u32 version` | `5` |
| `0x0C` | `u64 data_offset` | `0x884` |
| `0x14` | `u32 data_count` | `21` |
| `0x18` | profile 字节数 `u8` | `21` |
| `0x19` | profile 字符串 | `GW2.Main_Win64_Retail` |
| `0x2E` | `u32 game_version` | `407422` (`0x0006377E`) |
| `0x32` 起 | title、author、category、mod version、description、link | `Anti Chomper`、`Z-MoonZombies`、`Gameplay`、`1.0.0`、`This mod will help you to fight against Chomper`、空字符串 |
| `0x8D` | `u32 resource_count` | `25` |
| `0x91` | 资源记录数组起点 | 记录恰好结束于 `0x884` |

`FROSTY` magic、version 5、头字段及清单读取方式与 Frosty 的公开 reader/writer 实现相符；本报告中的数值则直接从上传的字节重新解析。[FrostyModReader.cs](https://github.com/SirhcMods/FrostyToolsuite/blob/main/FrostyPlugin/IO/FrostyModReader.cs)、[FrostyModWriter.cs](https://github.com/SirhcMods/FrostyToolsuite/blob/main/FrostyPlugin/IO/FrostyModWriter.cs)、[FrostyMod.cs](https://github.com/SirhcMods/FrostyToolsuite/blob/main/FrostyPlugin/Mod/FrostyMod.cs)

## 3. 资源记录与数据清单

### 3.1 记录布局

两个 v5 样本都符合以下通用记录布局：

```text
u8 type
i32 resource_index
NUL 结尾名称
如果 resource_index != -1：
    20 字节 SHA-1
    i64 logical_size
    u8 flags
    i32 handler_hash
    NUL 结尾 user_data
i32 added_bundle_count
i32[added_bundle_count] added_bundles
随后为资源类型专属字段（如适用）
```

静态反汇编现已直接确认目标 EXE 采用上述通用顺序和三种尾部。FBMOD 头解析器读取版本并拒绝大于 5 的值；在 RVA `0xF10B` 调用资源数组解析器 `0x1FB20` 时，也把版本值传入记录解析路径。记录读取器含版本分支，因此下面的重建代码明确限定为 v5。`0x1FB20` 逐条读一个类型字节、拒绝大于 4 的值，并通过 RVA `0x11E8A0` 的跳表分派：文件类型码 `0/1/2/3/4` 分别进入 RVA `0x1FC35/0x202AF/0x1FE9C/0x20039/0x1FCE7`。类型 0 和 1 在公共字段后直接结束；类型 2、3、4 分别读取 RES、Chunk、Bundle 尾部。

公共字段由 RVA `0x212D0` 解析：读取 `i32 resource_index`，再调用 NUL 字符串解析器 `0x1C190` 读取名称；当索引不是 `-1` 时，读取 20 字节 SHA-1、8 字节大小、1 字节 flags、4 字节 handler hash 和 NUL 结尾的 user_data。随后解析 added-bundles 的整数向量。RES 分支读取 `u32 res_type`、`u64 res_rid`、`i32 meta_len` 和 `meta_len` 字节；Chunk 分支连续读取 6 个 32 位字段（总计 `0x18` 字节）；Bundle 分支读取 NUL 结尾 bundle_name 和 4 字节 super-bundle hash。Chunk 六个字段的语义名称来自同一二进制内 ResourceRecord 序列化器的字段引用。目标解析器的反汇编不单独区分这些 Chunk 字段的有无符号类型；重建代码的具体 Rust 类型仍与公开 writer 的 schema 对照。[FrostyModReader.cs](https://raw.githubusercontent.com/SirhcMods/FrostyToolsuite/main/FrostyPlugin/IO/FrostyModReader.cs)、[BaseModResource.cs](https://raw.githubusercontent.com/SirhcMods/FrostyToolsuite/main/FrostyPlugin/Mod/BaseModResource.cs)、[FrostyModWriter.cs](https://raw.githubusercontent.com/SirhcMods/FrostyToolsuite/main/FrostyPlugin/IO/FrostyModWriter.cs)

`Anti Chomper` 样本只含类型码 0 和 1。补充样本 `4.4.4.fbmod` 含 4 条 RES、4 条 Chunk、没有 Bundle：四条 RES 均有 16 字节 metadata，四条 Chunk 均有 24 字节尾部；两类记录的 SHA-1 都匹配各自 manifest 存储字节。这样，RES 和 Chunk 尾部现在同时有目标 EXE 静态解析证据及样本字节对照；Bundle 尾部仍只有静态证据。详见 [secondary_sample_audit.txt](evidence/secondary_sample_audit.txt)。

随附 Rust 风格实现把头部和资源记录的读取限制在 `data_offset` 之前，并在分配记录/计数向量前检查可用字节数。样本的资源记录恰好结束于 `0x884`；实现允许记录后存在填充，不把样本现象误当作所有 v5 文件的格式约束。此处的可分配性检查是重建代码的安全加固，不声称目标程序使用同一策略。

### 3.2 `Anti Chomper` 样本记录

共 25 条记录：一个内嵌 Icon（清单索引 0）、四个没有数据索引的 Screenshot 占位记录，以及 20 个 EBX 资源（索引 1–20）。所有 EBX 记录的 `flags`、handler hash、added-bundles 均为 0/空。Icon 对应 PNG、大小 37,580 字节；记录内 SHA-1 字段为 20 个零字节，因此它与 PNG 的 SHA-1 不匹配是预期的。公开 writer 的 `EmbeddedResource` 只设置 manifest 索引和大小、没有设置 SHA-1，这与样本现象相符。[FrostyModWriter.cs](https://raw.githubusercontent.com/SirhcMods/FrostyToolsuite/main/FrostyPlugin/IO/FrostyModWriter.cs)

| 清单索引 | 名称 | 逻辑大小（字节） |
|---:|---|---:|
| 1 | `gameplay/soldiers/human/mpzombie_swat` | 93,904 |
| 2 | `gameplay/soldiers/human/mpzombie_pirate` | 103,120 |
| 3 | `gameplay/soldiers/human/mpzombie_scientist` | 100,464 |
| 4 | `gameplay/soldiers/human/mpzombie_captainbrainz` | 105,072 |
| 5 | `gameplay/soldiers/human/mpzombie_imp` | 126,896 |
| 6 | `gameplay/soldiers/human/mpzombie_engineer` | 96,304 |
| 7 | `gameplay/soldiers/human/mpzombie_allstar` | 93,328 |
| 8 | `gameplay/soldiers/human/mpzombie_hovergoat` | 100,960 |
| 9 | `gameplay/weapons/chomper/primary/darkunicornchomper/darkunicornchomper` | 26,928 |
| 10 | `gameplay/weapons/chomper/primary/armorchomp/armorchomp` | 26,976 |
| 11 | `gameplay/weapons/chomper/primary/bjomperchomper/bjomperchomper` | 26,912 |
| 12 | `gameplay/weapons/chomper/primary/chomperswallow/chomperswallow` | 26,176 |
| 13 | `gameplay/weapons/chomper/primary/chompthing/chompthing` | 29,968 |
| 14 | `gameplay/weapons/chomper/primary/discochomper/discochomper` | 26,528 |
| 15 | `gameplay/weapons/chomper/primary/electricchomper/electricchomp` | 29,760 |
| 16 | `gameplay/weapons/chomper/primary/flamechomp/flamechomp` | 29,760 |
| 17 | `gameplay/weapons/chomper/primary/frenzychomp/frenzychomp` | 26,496 |
| 18 | `gameplay/weapons/chomper/primary/stealthchomp/stealthchomp` | 27,152 |
| 19 | `gameplay/weapons/chomper/primary/toxicchomp/toxicchomp` | 29,360 |
| 20 | `gameplay/weapons/chomper/primary/yetichomper/yetichomper` | 24,688 |

### 3.3 `Anti Chomper` 样本的 `0x884` 清单结构

清单表从 `0x884` 开始，每项 16 字节：`u64 relative_offset`、`u64 size`。21 项占 `21 × 16 = 0x150` 字节，所以 payload 区从 `0x9D4` 开始。绝对 payload 地址为：

```text
payload_base = data_offset + data_count * 16
payload_start = payload_base + relative_offset
```

EXE 的资源数据读取函数 RVA `0xE7A0–0xE8FC` 静态读取索引乘 16 后的两个 64 位值，将相对偏移加到清单表尾部，并对索引、溢出及输入长度做检查。Frosty 的 reader/writer 也采用 16 字节清单项及表尾相对数据区。[FrostyModReader.cs](https://github.com/SirhcMods/FrostyToolsuite/blob/main/FrostyPlugin/IO/FrostyModReader.cs)、[FrostyModWriter.cs](https://github.com/SirhcMods/FrostyToolsuite/blob/main/FrostyPlugin/IO/FrostyModWriter.cs)

实测所有 21 段连续相接：Icon 位于 `0x9D4–0x9CA0`，之后是 20 段 EBX 数据，最后一段结束于 `0x122CA0`，与文件大小相等，末尾没有额外字节。

### 3.4 `4.4.4.fbmod` 补充样本

补充样本有 373 条资源记录和 373 个 manifest 项。资源记录从 `0x382` 开始并恰好结束于 `data_offset=0x94AA`；manifest payload 从 `0xABFA` 开始，最后一段结束于文件末尾 `0xE657D1`。记录类型计数为 embedded 5、EBX 360、RES 4、Chunk 4、Bundle 0。

四条 RES 记录的 metadata 长度均为 16 字节；四条 Chunk 记录均有 24 字节、六个 32 位字段的尾部。八条记录的 SHA-1 都匹配相应 manifest 存储字节。CAS framing 中，360 条 EBX 共 427 个 selector 0 块；四条 RES 各有一个 selector 0 块；四条 Chunk 中两条使用 selector 0，另外两条合计含 65 个 selector 9 块。所有块头声明的输出长度总和都与对应逻辑大小相等，没有发现截断或未解析尾部。

这为 Chunk 记录布局及 LZ4 selector 9 提供了真实样本数据。这里核对的是 CAS framing 和块头的输出长度；尚未用重建 Rust 解码器验证 selector 9 的实际输出字节。该样本没有 selector 2、15 或 Bundle 记录。精简核验结果见 [secondary_sample_audit.txt](evidence/secondary_sample_audit.txt)；原始 FBMOD 未复制到本候选包。

## 4. CAS 块头与解压调用

### 4.1 块头

提取函数 RVA `0x10A60–0x127D6` 在 call site `0x1127F` 调用 CAS 解码函数 RVA `0x24080–0x255A5`。解码函数循环读取每个 8 字节块头。指令对前 4 字节做 byte-swap 后使用，故块输出大小是 big-endian `u32`。后续字段从指令可重建为：

```text
u32 output_size_be
u8  codec_tag       # codec selector = codec_tag & 0x7F
u8  size_flags      # 压缩长度高位 = (size_flags & 0x0F) << 16
u16 packed_size_be  # 压缩长度低 16 位
bytes[payload_size]
```

因此 `payload_size = ((size_flags & 0x0F) << 16) | packed_size_be`。公开 Frosty 代码也将头中的高位 nibble 与低 16 位组合计算压缩数据长度。[FrostyModExecutor.cs](https://github.com/CadeEvs/FrostyToolsuite/blob/1.0.6.3/FrostyModSupport/FrostyModExecutor.cs)

第一个 EBX 块头实例是 `00 01 00 00 00 71 00 00`：输出长度 `0x10000`，codec selector `0`，长度扩展 nibble 为 1，payload 长度为 `0x10000`。第二块头 `00 00 6E D0 00 70 6E D0` 解出长度 `0x6ED0`、codec 0、payload `0x6ED0`。`0x0071` 是两个 tag 字节组合显示；这里不能把低字节 `0x71` 当作 codec id。

`Anti Chomper` 样本的所有 EBX 块均为 selector 0 的原样块：前 8 个较大的资源各含一个 64 KiB 块和一个尾块，后 12 个资源各含一个块。每个资源的块输出长度之和等于记录的逻辑大小；20 个记录的 SHA-1 均匹配其清单索引对应的**存储 payload 字节**。这也验证了资源索引与清单项的对应关系。

这组大小与哈希关系也符合公开 writer 的写入逻辑：EBX 记录中的 `size` 取序列化后原始 EBX 长度，而 SHA-1 对压缩后的数据 blob 计算，blob 再写入 manifest。因此资源的 `size` 与 manifest 存储长度本来就不要求相等；`Anti Chomper` 样本 selector 0 块解码后的总长度恰好等于逻辑 `size`。[FrostyModWriter.cs](https://raw.githubusercontent.com/SirhcMods/FrostyToolsuite/main/FrostyPlugin/IO/FrostyModWriter.cs)

### 4.2 解码分支

解码函数从块头取 codec selector，并通过 16 项分支表处理：

| selector | 静态代码证据 | 当前判断 |
|---:|---|---|
| 0 | 分支复制块数据到输出缓冲区；`Anti Chomper` 的 EBX 与第二样本的 EBX、RES、部分 Chunk 均走此分支 | 原样/未压缩块，已由样本验证 |
| 2 | 分支 `0x2428D` 建立 `0x8000` 工作区；在 `0x242B5` 把布尔值 `1` 传给状态构造器 `0x31C90`；更新函数位于 `0x31D30`；EXE 含 `zlib:` 与 `corrupt deflate stream` 诊断 | `flate2::Decompress::new(true)`：输入预期带 zlib wrapper。目标二进制中的 `flate2 1.1.9` API 文档确认 `true` 表示期待 zlib 头。[flate2 1.1.9 `mem.rs`](https://github.com/rust-lang/flate2-rs/blob/1.1.9/src/mem.rs#L2396-L2412)。32 KiB 流式形态由机器码确认；两个样本都没有 selector 2 块 |
| 9 | 内联 token 高低 nibble、扩展长度、匹配偏移与回拷循环；`4.4.4.fbmod` 的 Chunk 数据含 65 个此 selector 的块 | LZ4 block，机器码算法特征明确；样本输出目前仅核对了块头声明长度，未验证解码字节 |
| 15 | 分支调用辅助函数 RVA `0x1F190`，同一解码函数含 `zstd init:`、`zstd:` 初始化/错误路径字符串 | Zstandard，较高置信度；现已补入等价单帧解码代码，两个样本都没有此 selector |
| 其他 | 分支落入 unsupported compression type 路径 | 本二进制不接受这些 selector |

辅助函数 RVA `0x1F190` 再调用 `0x2E280` 与 `0x2DD90`，并生成带成功/错误状态的返回值。selector 15 的命名是结合该专属辅助路径和 `zstd` 初始化/错误字符串得出的静态判断；两个样本都没有 selector 15 块，不能通过样本输出验证 zstd 解码结果。`Anti Chomper` 样本只含 selector 0；补充样本有 selector 9，但没有 selector 2 或 15。

### 4.3 可恢复的伪代码

下面的结构概括已由样本和机器码支持的部分。它是可读重建，不是原始 Rust 源文件：

```rust
fn manifest_entry(i: usize) -> Result<&[u8]> {
    ensure!(i < data_count);
    let table = data_offset;
    let (relative, size) = read_le_u64_pair(table + i * 16)?;
    let payload_base = table + data_count * 16;
    let start = checked_add(payload_base, relative)?;
    return checked_slice(input, start, size);
}

fn decode_cas(input: &[u8]) -> Result<Vec<u8>> {
    let mut pos = 0;
    let mut output = Vec::new();
    while pos < input.len() {
        ensure!(input.len() - pos >= 8);
        let output_size = read_be_u32(input, pos)?;
        let selector = input[pos + 4] & 0x7f;
        let size_flags = input[pos + 5];
        let payload_size = ((size_flags as usize & 0x0f) << 16)
                         | read_be_u16(input, pos + 6)? as usize;
        let block = checked_slice(input, pos + 8, payload_size)?;
        let decoded = match selector {
            0  => copy_raw(block),
            2  => decode_zlib_stream(block, output_size)?,
            9  => decode_lz4_block(block, output_size)?,
            15 => decode_zstd(block, output_size)?,
            _  => return Err(UnsupportedCompression(selector)),
        };
        append_block(&mut output, decoded);
        pos += 8 + payload_size;
    }
    Ok(output)
}
```

伪代码有意省略 Rust 错误类型、分配器、上下文和边界错误文案。输出长度传给 codec 分支；伪代码没有假定原程序在何处、以什么方式验证每个 codec 的长度。selector 2 的 wrapper 参数来自 `0x242B5` 的布尔实参；两个样本均没有 selector 2 块，selector 9 虽有真实块数据但尚未端到端解码，selector 15 也没有样本块。

随附重建代码提供 EBX、RES 和 Chunk 的 CAS 提取辅助函数。它们把资源记录的 manifest 索引、CAS 解码和逻辑长度对照串成调用路径；Chunk 的逻辑长度取自类型专属尾部。该高层接口是便于阅读和复现的重建代码，不声称原程序有同名函数或完全相同的调用图。补充样本的 framing 长度与这些逻辑长度一致，但 Rust 解码器尚未对该样本运行。

### 4.4 提取函数的资源分派

提取函数从 RVA `0x10A60` 开始。循环读取一个资源对象数组：数组指针来自上下文偏移 `+0xC8`，数量来自 `+0xD0`，代码按 `0xA8` 字节步进。RVA `0x10B90` 读取对象首字段、减去 2 后跳转到 RVA `0x11DD10` 的相对偏移表。内部标签 `2` 的分支 `0x10BB1` 使用 `embedded`（8 字节），标签 `3` 的分支 `0x1110C0` 使用 `ebx`（3 字节），标签 `4` 的分支 `0x10C60` 使用 `res`（3 字节），标签 `5` 的分支 `0x11032` 使用 `chunks`（6 字节）；标签 `6` 跳入共享的向量追加路径。分支内的序列化调用和计数器偏移确认这些是资源类别键，见下文。

带有效数据索引的分支调用 RVA `0xE7A0` 读取清单数据（调用点包括 `0x10D7D`、`0x111DA`、`0x11354`、`0x1174A`）；负索引会走跳过数据的分支。另一条处理路径在 RVA `0x1127F` 调用 CAS 解码器 `0x24080`。字符串引用和调用点提供了更具体的输出证据：RVA `0x11707` 将 `embedded_` 传给 RVA `0xA2DF0`；RVA `0x11396`、`0x1178C` 将 `.bin` 传给同一函数，支持嵌入资源使用 `embedded_<名称>.bin` 的命名模板。RVA `0x11EBE` 将长度为 13 的 `metadata.json` 字符串传给 RVA `0x94760`；RVA `0x12504` 将长度为 12 的 `bundles.json` 字符串传给 RVA `0x94580`；RVA `0x11D8F` 也将 `ebx.json` 传给 RVA `0x94580`。这些调用分别构造或处理不同的路径/字符串对象，尚不能仅凭单个 call site 断定最终落盘函数。提取函数还引用 `resolved_bundles` 和 `unknown`，并包含 `Warn: CAS decompress`、`Warn: EBX parse` 错误文本。

需要注意，静态字符串区把若干相邻字面量紧密放在一起：RVA `0x11DBB2` 起始的字节表显示成 `totalmetadata.json`，但反汇编分别以长度 5 引用 `total`（资源计数 JSON 键）和以长度 13 引用 `metadata.json`（传给 RVA `0x94760` 的字符串参数）；它们不是一个 `totalmetadata.json` 文件名。类似地，`bundles` 键和 `bundles.json` 文件名也是两个不同的引用。`ebx.json` 在此函数中也作为字符串参数传给 RVA `0x94580`。

外层调用链能确认最终写盘形式：调用者 RVA `0x6BCB` 调用提取函数；RVA `0x6E00` 使用字符串 `Writing fbproject to ...` 记录目标，随后 RVA `0x6E21` 调用序列化函数 `0x111CD0`，RVA `0x6E67` 把生成的字节缓冲区交给 `0x8C2F0`。该写入辅助函数调用 `0x90110` 准备目标文件，再循环调用 `0x91E70` 写剩余字节；`0x91E70` 最终调用 NTDLL IAT 的 `NtWriteFile`（槽位 RVA `0x11C378`）。序列化器开头写入 8 字节 `FROSTY\0\0`，接着写入 `u32 14`。因此，资源分支引用的 `embedded_<名称>.bin`、`ebx.json`、`bundles.json` 等更可能是 FBProject 容器内的条目名；它们不能据此解释为分别落盘的独立文件。目标路径如何从命令行参数构造、其余 JSON 条目如何具体写入容器，仍未完全追完。

### 4.5 FBProject 序列化前缀（新确认）

完整静态反汇编 `0x111CD0–0x11623F` 后，可以进一步恢复输出容器的头部顺序和函数收尾。序列化器首先写入 `FROSTY\0\0` 和小端 `u32 14`；随后从传入 project 对象的偏移 `+0x08/+0x10` 复制一个长度指定的字符串并附加 NUL。接着，它取得一次当前时间并把同一个 64 位时间值写两次，再写入 project 对象偏移 `+0xB8` 的 32 位字段。之后按顺序从 `+0x20/+0x28`、`+0x38/+0x40`、`+0x50/+0x58`、`+0x68/+0x70`、`+0x80/+0x88` 取五个长度指定字符串，每个字符串后附加 NUL。由此，已确认的前缀可表达为：

```text
bytes[8]  "FROSTY\0\0"
u32_le    14
c_string  project.str_0       # 指针/长度位于 project + 0x08/+0x10
u64_le    generated_time
u64_le    generated_time      # 与前一字段相同
u32_le    project.field_B8
c_string  project.str_1       # 指针/长度位于 +0x20/+0x28
c_string  project.str_2       # 指针/长度位于 +0x38/+0x40
c_string  project.str_3       # 指针/长度位于 +0x50/+0x58
c_string  project.str_4       # 指针/长度位于 +0x68/+0x70
c_string  project.str_5       # 指针/长度位于 +0x80/+0x88
...       resource/item sections
```

以上偏移和写入顺序来自目标二进制的写入指令（主要在 RVA `0x111D88–0x112123`）；字符串字段的业务名称和 `+0xB8` 字段含义尚未可靠确认，因此暂以偏移命名。序列化器随后遍历 project 对象的 `+0xC8/+0xD0` 数组，步长 `0xA8`，并据记录类型构建分组/索引集合；在后续路径中，按 manifest index 取数据，部分条目再经过 CAS 解码。现有证据支持这些内容都进入同一个 FBProject 字节缓冲区，最终由外层写入单一目标文件；但全部 item variant 的精确字段 schema 尚待继续拆解。

收尾路径在 `0x116215` 进入不可返回的错误陷阱；该序列化函数在 `0x116240` 开始下一函数之前结束。成功时调用者取回一个 Vec 形状的三元组（数据指针、容量、当前长度），并把数据指针和有效长度交给文件写入链。扩容、分配失败等错误分支会落入错误/陷阱路径，而不是输出部分容器。

循环结束后，RVA `0x11F97` 开始构造 JSON 对象；RVA `0x11FD5` 序列化键 `header`，RVA `0x11FFD` 序列化键 `resource_count`。后者接收一个由 6 个计数值组成的对象。前五个值由资源类型分支分别递增：Embedded、EBX、RES、Chunk、Bundle；第六个值来自资源数组总数。序列化辅助函数对键分别使用明确长度：`embedded` 8、`ebx` 3、`res` 3、`chunks` 6、`bundles` 7、`total` 5。重建的高置信度形状是 `{"header": ..., "resource_count": {"embedded": n, "ebx": n, "res": n, "chunks": n, "bundles": n, "total": n}}`。`Anti Chomper` 样本对应 `{embedded: 5, ebx: 20, res: 0, chunks: 0, bundles: 0, total: 25}`；补充样本记录数为 `{embedded: 5, ebx: 360, res: 4, chunks: 4, bundles: 0, total: 373}`。该对象输出到哪个具体文件尚未从完整调用链确认；不要把相邻字符串误读为 `totalmetadata.json`。

## 5. EXE 静态特征与复现

EXE 是 PE32+ x86-64，ImageBase `0x140000000`，入口 RVA `0x110640`，5 个节，Windows console subsystem。只检查了节表、导入表、字符串与反汇编；没有启动进程。静态字符串包含 `Invalid magic: expected FROSTY header`、`src\fbmod.rs`、`src\extract.rs`、`Warn: CAS decompress` 及 codec 错误信息；还发现依赖构建路径标记 `lz4_flex-0.11.6`、`zstd-safe-7.2.4`、`flate2-1.1.9`、`miniz_oxide`。这些字符串能帮助定位代码和库版本，但不表示 EXE 内保留了原始 Rust 源码。

随报告提供的 [analyze_static.py](./analyze_static.py) 只依赖 Python 标准库，可以重算两文件的哈希、解析 FBMOD 头/资源/清单，检查索引、大小、SHA-1，并汇总 CAS 块。本轮从会话附件缓存重新读取了两个原件并重算哈希；复核结果写在 [current_static_audit.txt](evidence/current_static_audit.txt)。

```powershell
python .\analyze_static.py "C:\path\fbmod-decompiler.exe" "C:\path\Anti Chomper V1.0.0.fbmod"
python .\analyze_static.py "C:\path\4.4.4.fbmod"
```

本轮使用 Visual Studio `dumpbin /DISASM` 对原件做静态反汇编，没有启动 EXE。可复现范围过滤脚本为 [pe_dumpbin_range.ps1](./pe_dumpbin_range.ps1)；反汇编输出保存在 [extract_disasm.txt](evidence/extract_disasm.txt)、[manifest_disasm.txt](evidence/manifest_disasm.txt)、[cas_decoder_disasm.txt](evidence/cas_decoder_disasm.txt)、[deflate_helpers_disasm.txt](evidence/deflate_helpers_disasm.txt)、[resource_parse_full.txt](evidence/resource_parse_full.txt)、[resource_parse_leaf.txt](evidence/resource_parse_leaf.txt)、[reader_helper_1c190.txt](evidence/reader_helper_1c190.txt) 和 [resource_parser_dispatch.txt](evidence/resource_parser_dispatch.txt)。提取资源 JSON 字段和输出名称的关键调用片段另存于 [output_trace_disasm.txt](evidence/output_trace_disasm.txt)，外层 FBProject 序列化与写盘调用链在 [fbproject_write_trace.txt](evidence/fbproject_write_trace.txt)，序列化器主体和尾部反汇编分别在 [fbproject_serializer_full.txt](evidence/fbproject_serializer_full.txt) 与 [fbproject_serializer_tail.txt](evidence/fbproject_serializer_tail.txt)，按字符串地址筛出的全程序引用位于 [string_address_refs.txt](evidence/string_address_refs.txt)。例如：

```powershell
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x10A60 -EndRva 0x127D6
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x1FB20 -EndRva 0x20AA0
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x212D0 -EndRva 0x21900
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x2428D -EndRva 0x242C0
.\pe_dumpbin_range.ps1 -PePath "C:\path\fbmod-decompiler.exe" -StartRva 0x111CD0 -EndRva 0x116240
```

也可用 Ghidra 导入为 PE/x86-64 后查看 RVA `0xE7A0`、`0xF10B`、`0x1FB20`、`0x212D0`、`0x24080`、`0x31D30`。资源类型分派和记录尾部已从静态代码路径恢复；完整 Rust 源码仍无法精确还原，缺失的主要是原始标识符、精确错误类型与消息构造、依赖库 API 调用细节，以及没有压缩样本支撑的解压边界行为。本报告将样本字节、目标二进制证据和外部格式对照分开标注。

也可用附带的 [pe_disasm_range.py](./pe_disasm_range.py) 复现指定 RVA 范围（需安装 Python `capstone` 包）：

```powershell
python -m pip install capstone
python .\pe_disasm_range.py "C:\path\fbmod-decompiler.exe" 0x24080 0x24280
```

另有 [reconstructed_core.rs](./reconstructed_core.rs)，把头部、资源记录、清单读取和 CAS 块分派整理为 Rust 风格代码；selector 9 的 LZ4 block 解码按内联机器码重建了 token、扩展长度、偏移与重叠回拷。selector 2 已补入 0x8000 缓冲区的 zlib-wrapped Deflate 流式等价实现，selector 15 已补入固定输出区的 Zstandard 等价实现。旁边的 [Cargo.toml](./Cargo.toml) 固定了 EXE 字符串中识别出的 `flate2 1.1.9` 与 `zstd-safe 7.2.4` 版本，便于继续复现。它们还原了可观察的块输入/输出契约，但不声称复刻二进制内部的所有 Rust API、错误类型或错误映射。`Anti Chomper` 样本验证了 25 条资源记录和 21 个数据段中的 selector 0 块；第二样本补充了 RES/Chunk 记录和 selector 9 的真实 CAS framing。selector 9 解码后的实际输出尚未由重建 Rust 代码核对；selector 2、15 仍无样本块。

