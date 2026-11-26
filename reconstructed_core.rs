//! Readable reconstruction of the FBMOD parser and CAS dispatch recovered
//! from the supplied binary and sample. This is not the original Rust source.
//! Offsets/RVAs in comments refer to the PE image, not file offsets.

use std::{convert::TryInto, fmt};

// The dependency strings in the binary identify these codec families. These
// calls are readable equivalents, not assertions about the exact source APIs.
use flate2::{Decompress, FlushDecompress, Status};
use zstd_safe::DCtx;

const FROSTY_MAGIC: &[u8; 8] = b"FROSTY\0\x01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    BadMagic,
    UnsupportedVersion(u32),
    Truncated,
    InvalidUtf8,
    InvalidCount,
    InvalidIndex,
    MissingResourceData,
    MissingLogicalSize,
    ResourceKindMismatch { expected: u8, actual: u8 },
    IntegerOverflow,
    AllocationFailed,
    InvalidBlockLength,
    InvalidLz4Offset,
    CodecFailure(&'static str),
    UnsupportedResourceType(u8),
    UnsupportedCompression(u8),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => f.write_str("invalid FBMOD magic"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported FBMOD version {version}"),
            Self::Truncated => f.write_str("input ended before the requested field"),
            Self::InvalidUtf8 => f.write_str("string field is not valid UTF-8"),
            Self::InvalidCount => f.write_str("negative or invalid item count"),
            Self::InvalidIndex => f.write_str("manifest index is out of range"),
            Self::MissingResourceData => f.write_str("resource has no manifest data item"),
            Self::MissingLogicalSize => f.write_str("resource has no recorded logical size"),
            Self::ResourceKindMismatch { expected, actual } => {
                write!(f, "expected resource kind {expected}, got {actual}")
            }
            Self::IntegerOverflow => f.write_str("integer overflow while computing an offset"),
            Self::AllocationFailed => f.write_str("could not reserve memory for parsed data"),
            Self::InvalidBlockLength => {
                f.write_str("decoded block length does not match its header")
            }
            Self::InvalidLz4Offset => {
                f.write_str("LZ4 match offset points before the output buffer")
            }
            Self::CodecFailure(codec) => write!(f, "{codec} decompression failed"),
            Self::UnsupportedResourceType(kind) => write!(f, "unsupported resource type {kind}"),
            Self::UnsupportedCompression(selector) => {
                write!(f, "unsupported CAS compression selector {selector}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone)]
pub struct ModHeader {
    pub version: u32,
    pub data_offset: u64,
    pub data_count: u32,
    pub profile_name: String,
    pub game_version: u32,
    pub title: String,
    pub author: String,
    pub category: String,
    pub mod_version: String,
    pub description: String,
    pub link: String,
}

#[derive(Debug, Clone)]
pub struct ResourceRecord {
    pub kind: u8,
    pub resource_index: i32,
    pub name: String,
    pub sha1: Option<[u8; 20]>,
    pub logical_size: Option<i64>,
    pub flags: Option<u8>,
    pub handler_hash: Option<i32>,
    pub user_data: Option<String>,
    pub added_bundles: Vec<i32>,
    pub extra: ResourceExtra,
}

/// Type-specific tails recovered from the target EXE's static parser:
/// RES is a type/rid/length-prefixed metadata blob, Chunk is six 32-bit
/// values, and Bundle is a NUL-terminated name plus a hash. The disassembly
/// establishes field boundaries and widths; signedness choices follow the
/// public writer schema where machine code cannot distinguish them. The
/// secondary sample confirms RES and Chunk tails; Bundle still has no
/// sample-level record for comparison.
#[derive(Debug, Clone)]
pub enum ResourceExtra {
    None,
    Res {
        res_type: u32,
        res_rid: u64,
        meta: Vec<u8>,
    },
    Chunk {
        range_start: u32,
        range_end: u32,
        logical_offset: u32,
        logical_size: u32,
        h32: i32,
        first_mip: i32,
    },
    Bundle {
        bundle_name: String,
        super_bundle_hash: i32,
    },
}

/// Reconstructed from the extractor's per-kind counters and the strings used
/// for its JSON metadata. `u64` is the readable representation used here; the
/// original in-memory counter widths are not established. The uploaded sample
/// has counts 5/20/0/0/0, total 25.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceCounts {
    pub embedded: u64,
    pub ebx: u64,
    pub res: u64,
    pub chunks: u64,
    pub bundles: u64,
    pub total: u64,
}

/// High-confidence shape inferred from the serializer keys `header` and
/// `resource_count`; exact JSON formatting is not reconstructed here.
#[derive(Debug, Clone)]
pub struct TotalMetadata {
    pub header: ModHeader,
    pub resource_count: ResourceCounts,
}

pub fn count_resources(resources: &[ResourceRecord]) -> Result<ResourceCounts, ParseError> {
    let mut counts = ResourceCounts::default();
    for resource in resources {
        match resource.kind {
            0 => counts.embedded += 1,
            1 => counts.ebx += 1,
            2 => counts.res += 1,
            3 => counts.chunks += 1,
            4 => counts.bundles += 1,
            other => return Err(ParseError::UnsupportedResourceType(other)),
        }
        counts.total += 1;
    }
    Ok(counts)
}

#[derive(Debug, Clone, Copy)]
pub struct ManifestEntry {
    pub relative_offset: u64,
    pub stored_size: u64,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ParseError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(ParseError::IntegerOverflow)?;
        let out = self.bytes.get(self.pos..end).ok_or(ParseError::Truncated)?;
        self.pos = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    fn u32_le(&mut self) -> Result<u32, ParseError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32_le(&mut self) -> Result<i32, ParseError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i64_le(&mut self) -> Result<i64, ParseError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn u64_le(&mut self) -> Result<u64, ParseError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn utf8(&mut self, len: usize) -> Result<String, ParseError> {
        let bytes = self.take(len)?;
        let text = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidUtf8)?;
        let mut value = String::new();
        value
            .try_reserve_exact(len)
            .map_err(|_| ParseError::AllocationFailed)?;
        value.push_str(text);
        Ok(value)
    }

    fn c_string(&mut self) -> Result<String, ParseError> {
        let rest = self.bytes.get(self.pos..).ok_or(ParseError::Truncated)?;
        let nul = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(ParseError::Truncated)?;
        let value = self.utf8(nul)?;
        self.take(1)?; // Consume the NUL terminator.
        Ok(value)
    }
}

fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, ParseError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| ParseError::AllocationFailed)?;
    Ok(values)
}

fn try_vec_from_slice<T: Copy>(source: &[T]) -> Result<Vec<T>, ParseError> {
    let mut values = try_vec_with_capacity(source.len())?;
    values.extend_from_slice(source);
    Ok(values)
}

/// Header offsets and field order recovered from the sample and `fbmod.rs`
/// parser. The target binary's resource parser confirms the common record
/// fields and type-specific tails for codes 0..4. Types 2/3/4 are absent from
/// this sample, so their records cannot be checked against sample bytes.
pub fn parse_fbmod_v5(bytes: &[u8]) -> Result<(ModHeader, Vec<ResourceRecord>), ParseError> {
    let mut r = Reader { bytes, pos: 0 };
    if r.take(8)? != FROSTY_MAGIC {
        return Err(ParseError::BadMagic);
    }
    let version = r.u32_le()?;
    if version != 5 {
        return Err(ParseError::UnsupportedVersion(version));
    }
    let data_offset = r.u64_le()?;
    let data_count = r.u32_le()?;
    let data_limit = usize::try_from(data_offset).map_err(|_| ParseError::IntegerOverflow)?;
    let record_region = bytes.get(..data_limit).ok_or(ParseError::Truncated)?;
    // Keep all header and record reads before the manifest table. This also
    // permits padding between the final counted record and data_offset.
    r.bytes = record_region;
    let profile_len = r.u8()? as usize;
    let profile_name = r.utf8(profile_len)?;
    let game_version = r.u32_le()?;
    let title = r.c_string()?;
    let author = r.c_string()?;
    let category = r.c_string()?;
    let mod_version = r.c_string()?;
    let description = r.c_string()?;
    let link = r.c_string()?;
    let resource_count = r.u32_le()?;

    let resource_count =
        usize::try_from(resource_count).map_err(|_| ParseError::IntegerOverflow)?;
    // The smallest record is: kind byte, i32 index, empty NUL string, and an
    // i32 added-bundle count. Reject impossible counts before reserving.
    if resource_count > r.remaining() / 10 {
        return Err(ParseError::Truncated);
    }

    let header = ModHeader {
        version,
        data_offset,
        data_count,
        profile_name,
        game_version,
        title,
        author,
        category,
        mod_version,
        description,
        link,
    };

    let mut resources = try_vec_with_capacity(resource_count)?;
    for _ in 0..resource_count {
        let kind = r.u8()?;
        if kind > 4 {
            return Err(ParseError::UnsupportedResourceType(kind));
        }
        let resource_index = r.i32_le()?;
        let name = r.c_string()?;
        let (sha1, logical_size, flags, handler_hash, user_data) = if resource_index != -1 {
            let sha1: [u8; 20] = r.take(20)?.try_into().unwrap();
            (
                Some(sha1),
                Some(r.i64_le()?),
                Some(r.u8()?),
                Some(r.i32_le()?),
                Some(r.c_string()?),
            )
        } else {
            (None, None, None, None, None)
        };
        let bundle_count = r.i32_le()?;
        if bundle_count < 0 {
            return Err(ParseError::InvalidCount);
        }
        let bundle_count =
            usize::try_from(bundle_count).map_err(|_| ParseError::IntegerOverflow)?;
        if bundle_count > r.remaining() / 4 {
            return Err(ParseError::Truncated);
        }
        let mut added_bundles = try_vec_with_capacity(bundle_count)?;
        for _ in 0..bundle_count {
            added_bundles.push(r.i32_le()?);
        }
        let extra = match kind {
            0 | 1 => ResourceExtra::None,
            2 => {
                let res_type = r.u32_le()?;
                let res_rid = r.u64_le()?;
                let meta_len = r.i32_le()?;
                if meta_len < 0 {
                    return Err(ParseError::InvalidCount);
                }
                let meta_len =
                    usize::try_from(meta_len).map_err(|_| ParseError::IntegerOverflow)?;
                let meta = try_vec_from_slice(r.take(meta_len)?)?;
                ResourceExtra::Res {
                    res_type,
                    res_rid,
                    meta,
                }
            }
            3 => ResourceExtra::Chunk {
                range_start: r.u32_le()?,
                range_end: r.u32_le()?,
                logical_offset: r.u32_le()?,
                logical_size: r.u32_le()?,
                h32: r.i32_le()?,
                first_mip: r.i32_le()?,
            },
            4 => ResourceExtra::Bundle {
                bundle_name: r.c_string()?,
                super_bundle_hash: r.i32_le()?,
            },
            other => return Err(ParseError::UnsupportedResourceType(other)),
        };
        resources.push(ResourceRecord {
            kind,
            resource_index,
            name,
            sha1,
            logical_size,
            flags,
            handler_hash,
            user_data,
            added_bundles,
            extra,
        });
    }
    // The supplied sample ends exactly at data_offset. Other v5 files may
    // leave padding after the counted records; reads remain bounded above.
    Ok((header, resources))
}

/// Reconstructed from the reader at RVA 0xE7A0–0xE8FC: validate index,
/// read one little-endian pair at index*16, then slice from the table end.
pub fn manifest_data<'a>(
    bytes: &'a [u8],
    h: &ModHeader,
    index: u32,
) -> Result<&'a [u8], ParseError> {
    if index >= h.data_count {
        return Err(ParseError::InvalidIndex);
    }
    let table = usize::try_from(h.data_offset).map_err(|_| ParseError::IntegerOverflow)?;
    let entry_off = table
        .checked_add(
            (index as usize)
                .checked_mul(16)
                .ok_or(ParseError::IntegerOverflow)?,
        )
        .ok_or(ParseError::IntegerOverflow)?;
    let entry_end = entry_off
        .checked_add(16)
        .ok_or(ParseError::IntegerOverflow)?;
    let entry = bytes
        .get(entry_off..entry_end)
        .ok_or(ParseError::Truncated)?;
    let relative = u64::from_le_bytes(entry[0..8].try_into().unwrap());
    let stored_size = u64::from_le_bytes(entry[8..16].try_into().unwrap());
    let payload_base = table
        .checked_add(
            (h.data_count as usize)
                .checked_mul(16)
                .ok_or(ParseError::IntegerOverflow)?,
        )
        .ok_or(ParseError::IntegerOverflow)?;
    let start = payload_base
        .checked_add(usize::try_from(relative).map_err(|_| ParseError::IntegerOverflow)?)
        .ok_or(ParseError::IntegerOverflow)?;
    let end = start
        .checked_add(usize::try_from(stored_size).map_err(|_| ParseError::IntegerOverflow)?)
        .ok_or(ParseError::IntegerOverflow)?;
    bytes.get(start..end).ok_or(ParseError::Truncated)
}

/// Resolve the manifest payload referenced by a resource record. The target
/// parser uses `-1` for records without an associated data item.
pub fn resource_manifest_data<'a>(
    bytes: &'a [u8],
    h: &ModHeader,
    resource: &ResourceRecord,
) -> Result<Option<&'a [u8]>, ParseError> {
    match resource.resource_index {
        -1 => Ok(None),
        index if index >= 0 => Ok(Some(manifest_data(bytes, h, index as u32)?)),
        _ => Err(ParseError::InvalidIndex),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CasBlock<'a> {
    pub output_size: u32,
    pub selector: u8,
    /// Raw header bytes 4–5 combined as big-endian u16; not the codec id.
    pub tag: u16,
    pub payload: &'a [u8],
}

/// Block framing reconstructed from decoder RVA 0x24080–0x255A5.
pub fn cas_blocks(mut bytes: &[u8]) -> Result<Vec<CasBlock<'_>>, ParseError> {
    let mut blocks = Vec::new();
    while !bytes.is_empty() {
        let header = bytes.get(..8).ok_or(ParseError::Truncated)?;
        let output_size = u32::from_be_bytes(header[0..4].try_into().unwrap());
        let selector = header[4] & 0x7f;
        let size_flags = header[5];
        let tag = u16::from_be_bytes([header[4], header[5]]);
        let payload_size = (((size_flags & 0x0f) as usize) << 16)
            | u16::from_be_bytes([header[6], header[7]]) as usize;
        let rest = bytes.get(8..).ok_or(ParseError::Truncated)?;
        let payload = rest.get(..payload_size).ok_or(ParseError::Truncated)?;
        blocks
            .try_reserve(1)
            .map_err(|_| ParseError::AllocationFailed)?;
        blocks.push(CasBlock {
            output_size,
            selector,
            tag,
            payload,
        });
        bytes = &rest[payload_size..];
    }
    Ok(blocks)
}

/// Reconstructed from the inlined selector-9 branch at RVA 0x24400 onward.
/// The instruction pattern reads a token, optional 0xFF length extensions,
/// a little-endian match distance, and performs overlap-safe back references.
pub fn decode_lz4_block(input: &[u8], expected_output: usize) -> Result<Vec<u8>, ParseError> {
    let mut output = try_vec_with_capacity(expected_output)?;
    let mut ip = 0usize;

    while ip < input.len() {
        let token = *input.get(ip).ok_or(ParseError::Truncated)?;
        ip += 1;

        let literal_code = (token >> 4) as usize;
        let literal_len = read_lz4_length(input, &mut ip, literal_code, literal_code == 15)?;
        let literal_end = ip
            .checked_add(literal_len)
            .ok_or(ParseError::IntegerOverflow)?;
        let literals = input.get(ip..literal_end).ok_or(ParseError::Truncated)?;
        let output_end = output
            .len()
            .checked_add(literal_len)
            .ok_or(ParseError::IntegerOverflow)?;
        if output_end > expected_output {
            return Err(ParseError::InvalidBlockLength);
        }
        output.extend_from_slice(literals);
        ip = literal_end;

        // A final sequence may contain literals only, with no match distance.
        if ip == input.len() {
            break;
        }

        let offset_end = ip.checked_add(2).ok_or(ParseError::IntegerOverflow)?;
        let offset_bytes = input.get(ip..offset_end).ok_or(ParseError::Truncated)?;
        let distance = u16::from_le_bytes([offset_bytes[0], offset_bytes[1]]) as usize;
        ip += 2;
        if distance == 0 || distance > output.len() {
            return Err(ParseError::InvalidLz4Offset);
        }

        let base_match_len = (token as usize & 0x0f) + 4;
        let match_len = read_lz4_length(input, &mut ip, base_match_len, token & 0x0f == 0x0f)?;
        let match_end = output
            .len()
            .checked_add(match_len)
            .ok_or(ParseError::IntegerOverflow)?;
        if match_end > expected_output {
            return Err(ParseError::InvalidBlockLength);
        }

        // Byte-wise copying intentionally permits source and destination ranges
        // to overlap, as required by LZ4 back references.
        while output.len() < match_end {
            let value = output[output.len() - distance];
            output.push(value);
        }
    }

    if output.len() != expected_output {
        return Err(ParseError::InvalidBlockLength);
    }
    Ok(output)
}

fn read_lz4_length(
    input: &[u8],
    ip: &mut usize,
    base: usize,
    has_extension: bool,
) -> Result<usize, ParseError> {
    let mut length = base;
    if has_extension {
        loop {
            let extension = *input.get(*ip).ok_or(ParseError::Truncated)?;
            *ip = (*ip).checked_add(1).ok_or(ParseError::IntegerOverflow)?;
            length = length
                .checked_add(extension as usize)
                .ok_or(ParseError::IntegerOverflow)?;
            if extension != 0xff {
                break;
            }
        }
    }
    Ok(length)
}

/// Selector 2's machine-code path uses a 0x8000-byte streaming workspace.
/// The callsite at RVA 0x242B5 passes `true` to the decompressor constructor,
/// selecting a zlib header around the Deflate stream.
pub fn decode_deflate_block(input: &[u8], expected_output: usize) -> Result<Vec<u8>, ParseError> {
    let mut decoder = Decompress::new(true);
    let mut output = try_vec_with_capacity(expected_output.min(0x100000))?;
    let mut workspace = try_vec_with_capacity(0x8000)?;
    workspace.resize(0x8000, 0);
    let mut input_pos = 0usize;

    loop {
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let flush = if input_pos == input.len() {
            FlushDecompress::Finish
        } else {
            FlushDecompress::None
        };
        let status = decoder
            .decompress(&input[input_pos..], &mut workspace, flush)
            .map_err(|_| ParseError::CodecFailure("zlib/Deflate"))?;
        let consumed = usize::try_from(decoder.total_in() - before_in)
            .map_err(|_| ParseError::IntegerOverflow)?;
        let produced = usize::try_from(decoder.total_out() - before_out)
            .map_err(|_| ParseError::IntegerOverflow)?;
        input_pos = input_pos
            .checked_add(consumed)
            .ok_or(ParseError::IntegerOverflow)?;
        if produced > expected_output.saturating_sub(output.len()) {
            return Err(ParseError::InvalidBlockLength);
        }
        output
            .try_reserve(produced)
            .map_err(|_| ParseError::AllocationFailed)?;
        let chunk = workspace
            .get(..produced)
            .ok_or(ParseError::InvalidBlockLength)?;
        output.extend_from_slice(chunk);

        if status == Status::StreamEnd {
            if input_pos != input.len() || output.len() != expected_output {
                return Err(ParseError::InvalidBlockLength);
            }
            return Ok(output);
        }
        if consumed == 0 && produced == 0 {
            return Err(ParseError::CodecFailure("incomplete zlib/Deflate stream"));
        }
    }
}

/// Equivalent one-frame Zstandard decode using the crate family identified in
/// the binary. The machine-code path creates/uses a helper context; this
/// recovers the same input/output contract with a readable `zstd-safe` call.
pub fn decode_zstd_block(input: &[u8], expected_output: usize) -> Result<Vec<u8>, ParseError> {
    let mut decoder = DCtx::try_create().ok_or(ParseError::AllocationFailed)?;
    let mut output = try_vec_with_capacity(expected_output)?;
    output.resize(expected_output, 0);
    let written = decoder
        .decompress(output.as_mut_slice(), input)
        .map_err(|_| ParseError::CodecFailure("Zstandard"))?;
    if written != expected_output {
        return Err(ParseError::InvalidBlockLength);
    }
    Ok(output)
}

/// Codec dispatch reconstructed from the decoder's jump table. Selector 2
/// enters a 0x8000-byte streaming path (setup RVA 0x2428D, state helper RVA
/// 0x31C90, update routine RVA 0x31D30); selector 15 calls helper RVA 0x1F190,
/// which in turn calls 0x2E280 and 0x2DD90. These are equivalent codec
/// implementations; they do not claim the original methods or error mapping.
pub fn decode_block(block: CasBlock<'_>) -> Result<Vec<u8>, ParseError> {
    match block.selector {
        0 => {
            if block.payload.len() != block.output_size as usize {
                return Err(ParseError::InvalidBlockLength);
            }
            try_vec_from_slice(block.payload)
        }
        2 => decode_deflate_block(block.payload, block.output_size as usize),
        9 => decode_lz4_block(block.payload, block.output_size as usize),
        15 => decode_zstd_block(block.payload, block.output_size as usize),
        other => Err(ParseError::UnsupportedCompression(other)),
    }
}

/// Decodes supported CAS selector branches.
pub fn decode_cas(bytes: &[u8]) -> Result<Vec<u8>, ParseError> {
    let blocks = cas_blocks(bytes)?;
    let mut output = Vec::new();
    for block in blocks {
        let decoded = decode_block(block)?;
        output
            .try_reserve(decoded.len())
            .map_err(|_| ParseError::AllocationFailed)?;
        output.extend(decoded);
    }
    Ok(output)
}

fn decode_cas_resource(
    bytes: &[u8],
    header: &ModHeader,
    resource: &ResourceRecord,
    expected_size: usize,
) -> Result<Vec<u8>, ParseError> {
    let payload =
        resource_manifest_data(bytes, header, resource)?.ok_or(ParseError::MissingResourceData)?;
    let decoded = decode_cas(payload)?;
    if decoded.len() != expected_size {
        return Err(ParseError::InvalidBlockLength);
    }
    Ok(decoded)
}

/// Extract and decode one EBX record (kind 1), then compare its output length
/// with the common resource logical-size field.
pub fn decode_ebx_resource(
    bytes: &[u8],
    header: &ModHeader,
    resource: &ResourceRecord,
) -> Result<Vec<u8>, ParseError> {
    if resource.kind != 1 {
        return Err(ParseError::ResourceKindMismatch {
            expected: 1,
            actual: resource.kind,
        });
    }
    let expected_size = resource
        .logical_size
        .ok_or(ParseError::MissingLogicalSize)?;
    let expected_size = usize::try_from(expected_size).map_err(|_| ParseError::InvalidCount)?;
    decode_cas_resource(bytes, header, resource, expected_size)
}

/// Decode one RES record (kind 2). The secondary sample stores RES payloads
/// as CAS blocks whose total output matches the common logical-size field.
pub fn decode_res_resource(
    bytes: &[u8],
    header: &ModHeader,
    resource: &ResourceRecord,
) -> Result<Vec<u8>, ParseError> {
    if resource.kind != 2 {
        return Err(ParseError::ResourceKindMismatch {
            expected: 2,
            actual: resource.kind,
        });
    }
    let expected_size = resource
        .logical_size
        .ok_or(ParseError::MissingLogicalSize)?;
    let expected_size = usize::try_from(expected_size).map_err(|_| ParseError::InvalidCount)?;
    decode_cas_resource(bytes, header, resource, expected_size)
}

/// Decode one Chunk record (kind 3). Its expected output size is stored in
/// the type-specific Chunk tail, not the common resource-size field.
pub fn decode_chunk_resource(
    bytes: &[u8],
    header: &ModHeader,
    resource: &ResourceRecord,
) -> Result<Vec<u8>, ParseError> {
    if resource.kind != 3 {
        return Err(ParseError::ResourceKindMismatch {
            expected: 3,
            actual: resource.kind,
        });
    }
    let expected_size = match &resource.extra {
        ResourceExtra::Chunk { logical_size, .. } => {
            usize::try_from(*logical_size).map_err(|_| ParseError::InvalidCount)?
        }
        _ => return Err(ParseError::MissingLogicalSize),
    };
    decode_cas_resource(bytes, header, resource, expected_size)
}
