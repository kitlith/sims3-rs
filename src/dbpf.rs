pub mod filetypes;

use crate::refpack;
use lazy_init::LazyTransform;
use scroll::{ctx, Pread, LE};
use std::borrow::Cow;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum IndexType {
    IndexHeader(u8),
    IndexEntry(u8, DBPFIndex),
}

#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
struct DBPFIndex {
    resource_type: u32,
    resource_group: u32, // TODO: Flags?
    instance: u64,
    chunk_offset: usize,
    filesize: usize,
    unk1: bool,
    memsize: u32,
    compressed: bool,
    unk2: u16,
}

impl<'a> ctx::TryFromCtx<'a, IndexType> for DBPFIndex {
    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], ctx: IndexType) -> Result<(Self, Self::Size), Self::Error> {
        let mut data = DBPFIndex::default();
        let mask: u8;
        match ctx {
            IndexType::IndexHeader(m) => mask = m,
            IndexType::IndexEntry(m, template) => {
                mask = !m;
                data = template.clone();
            }
        }

        let offset = &mut 0;
        if (mask & (1 << 0)) != 0 {
            let raw_type = src.gread_with::<u32>(offset, LE)?;
            data.resource_type = raw_type; // would use ResourceType, but there's too many unknowns.
        }
        if (mask & (1 << 1)) != 0 {
            data.resource_group = src.gread_with(offset, LE)?;
        }
        if (mask & (1 << 2)) != 0 {
            // instance_lo
            let field: u32 = src.gread_with(offset, LE)?;
            data.instance = (data.instance & 0x0000FFFF) | (field as u64);
        }
        if (mask & (1 << 3)) != 0 {
            // instance_hi
            let field: u32 = src.gread_with(offset, LE)?;
            data.instance = (data.instance & 0xFFFF0000) | ((field as u64) << 32);
        }
        if (mask & (1 << 4)) != 0 {
            data.chunk_offset = src.gread_with::<u32>(offset, LE)? as usize;
        }
        if (mask & (1 << 5)) != 0 {
            let field: u32 = src.gread_with(offset, LE)?;
            data.filesize = (field & 0x7FFFFFFF) as usize;
            data.unk1 = (field & 0x80000000) != 0;
        }
        if (mask & (1 << 6)) != 0 {
            data.memsize = src.gread_with(offset, LE)?;
        }
        if (mask & (1 << 7)) != 0 {
            let field: u32 = src.gread_with(offset, LE)?;
            data.compressed = (field & 0xFFFF) == 0xFFFF;
            data.unk2 = (field >> 16) as u16;
        }

        Ok((data, *offset))
    }
}

#[derive(Debug, PartialEq, Pread, Pwrite)]
struct DBPFHeader {
    magic: u32, // "DBPF"
    major: u32,
    minor: u32,
    unk1: [u8; 24],
    index_entries: u32,
    unk2: [u8; 4],
    index_size: u32,
    unk3: [u8; 12],
    index_version: u32, // 3
    index_position: u32,
    unk4: [u8; 28],
}

//#[derive(Debug)]
pub struct DBPFEntry<'a> {
    pub resource_type: u32,
    pub resource_group: u32, // TODO: Flags?
    pub instance: u64,
    pub unk1: bool,
    pub unk2: u16,
    // This holds a slice to the compressed region before
    // and a possibly owned decompressed slice afterwards
    // TODO: What if I want to use it, discard, and then reuse again later?
    data: LazyTransform<&'a [u8], Cow<'a, [u8]>>,
}

impl<'a> DBPFEntry<'a> {
    pub fn data(&self) -> &[u8] {
        &self
            .data
            .get_or_create(|mem| Cow::Owned(refpack::decompress(mem).unwrap()))
    }
}

#[derive(Default)]
pub struct DBPF<'a> {
    pub major: u32,
    pub minor: u32,
    pub files: Vec<DBPFEntry<'a>>,
}

impl<'a> ctx::TryFromCtx<'a, ()> for DBPF<'a> {
    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, Self::Size), Self::Error> {
        let header: DBPFHeader = src.pread(0)?;
        // "DBPF"
        if header.magic != 0x46504244 {
            Err(scroll::Error::Custom(format!(
                "Bad Magic 0x{:x}",
                header.magic
            )))?;
        }

        let files: Vec<_>;
        if header.index_entries != 0 {
            let mut index = vec![DBPFIndex::default(); header.index_entries as usize];
            let index_src = &src[header.index_position as usize
                                     ..(header.index_position + header.index_size) as usize];
            let mask: u8 = index_src.pread_with::<u32>(0, LE)? as u8;
            let offset = &mut 4;
            let index_header = index_src.gread_with(offset, IndexType::IndexHeader(mask))?;
            index_src.gread_inout_with(
                offset,
                &mut index,
                IndexType::IndexEntry(mask, index_header),
            )?;
            files = index
                .into_iter()
                .map(|index| {
                    let raw = &src[index.chunk_offset..(index.chunk_offset + index.filesize)];
                    let data = LazyTransform::new(raw);
                    if !index.compressed {
                        // We don't need to defer this.
                        data.get_or_create(|mem| Cow::Borrowed(mem));
                    }

                    Ok(DBPFEntry {
                        resource_type: index.resource_type,
                        resource_group: index.resource_group,
                        instance: index.instance,
                        unk1: index.unk1,
                        unk2: index.unk2,
                        data,
                    })
                }).collect::<Result<_, _>>()
                .map_err(|err: refpack::decompress::Error| {
                    scroll::Error::Custom(err.to_string())
                })?;
        } else {
            files = Vec::with_capacity(0);
        }

        let data = DBPF {
            major: header.major,
            minor: header.minor,
            files: files, // already checked for error.
        };
        Ok((data, src.len()))
    }
}

impl<'a> DBPF<'a> {
    pub fn new(mem: &'a [u8]) -> Result<DBPF, scroll::Error> {
        mem.pread::<DBPF>(0)
    }
}
