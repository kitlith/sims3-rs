pub mod filetypes;

use lazy_init::LazyTransform;
use scroll::{ctx, Pread, Pwrite, LE};
use std::borrow::Cow;
use std::collections::BTreeMap;

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
    fn try_from_ctx(src: &'a [u8], ctx: IndexType) -> Result<(Self, usize), Self::Error> {
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
            // instance_hi
            let field: u32 = src.gread_with(offset, LE)?;
            data.instance = (data.instance & (u32::max_value() as u64)) // keep only the low half
                          | ((field as u64) << 32); // replace the high half
        }
        if (mask & (1 << 3)) != 0 {
            // instance_lo
            let field: u32 = src.gread_with(offset, LE)?;
            data.instance = (data.instance & ((u32::max_value() as u64) << 32)) // keep only the high half
                          | (field as u64); // replace the low half
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

impl ctx::TryIntoCtx<IndexType> for DBPFIndex {
    type Error = scroll::Error;
    fn try_into_ctx(self, dest: &mut [u8], ctx: IndexType) -> Result<usize, Self::Error> {
        let mask: u8;
        match ctx {
            IndexType::IndexHeader(m) => mask = m,
            IndexType::IndexEntry(m, _) => mask = !m // TODO: check that the data does match between template and entry?
        }
        let size: usize = mask.count_ones() as usize * std::mem::size_of::<u32>();
        let dest = &mut dest[..size];

        let offset = &mut 0;
        if (mask & (1 << 0)) != 0 {
            dest.gwrite_with::<u32>(self.resource_type, offset, LE)?;
        }
        if (mask & (1 << 1)) != 0 {
            dest.gwrite_with::<u32>(self.resource_group, offset, LE)?;
        }
        if (mask & (1 << 2)) != 0 {
            // instance_hi
            dest.gwrite_with::<u32>((self.instance >> 32) as u32, offset, LE)?;
        }
        if (mask & (1 << 3)) != 0 {
            // instance_lo
            dest.gwrite_with::<u32>(self.instance as u32, offset, LE)?;
        }
        if (mask & (1 << 4)) != 0 {
            dest.gwrite_with::<u32>(self.chunk_offset as u32, offset, LE)?;
        }
        if (mask & (1 << 5)) != 0 {
            // FIXME: this should probably really be an error instead of an assert.
            assert!(self.filesize <= i32::max_value() as usize, "File is too large for format!");
            let field: u32 = (self.filesize & 0x7FFFFFFF) as u32 | if self.unk1 {1 << 31} else {0};
            dest.gwrite_with::<u32>(field, offset, LE)?;
        }
        if (mask & (1 << 6)) != 0 {
            dest.gwrite_with::<u32>(self.memsize, offset, LE)?;
        }
        if (mask & (1 << 7)) != 0 {
            let field: u32 = (self.unk2 as u32) << 16 | if self.compressed {0xFFFF} else {0};
            dest.gwrite_with::<u32>(field, offset, LE)?;
        }

        assert_eq!(*offset, size, "Written size {} does not match size implied by mask ({})!", *offset, size);
        Ok(size)
    }
}

// 96 bytes in file.
#[derive(Debug, PartialEq, Pread, Pwrite, Default)]
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
    // FIXME: should return Result instead of just panicing.
    pub fn data(&self) -> &[u8] {
        &self
            .data
            .get_or_create(|mem| Cow::Owned(refpack::easy_decompress::<refpack::format::TheSims34>(mem).unwrap()))
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
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, usize), Self::Error> {
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

                    DBPFEntry {
                        resource_type: index.resource_type,
                        resource_group: index.resource_group,
                        instance: index.instance,
                        unk1: index.unk1,
                        unk2: index.unk2,
                        data,
                    }
                }).collect::<Vec<_>>();
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

impl<'a> ctx::TryIntoCtx for DBPF<'a> {
    type Error = scroll::Error;
    fn try_into_ctx(self, dest: &mut [u8], _ctx: ()) -> Result<usize, Self::Error> {
        let mut header = DBPFHeader::default();
        header.magic = 0x46504244;
        header.major = self.major;
        header.minor = self.minor;
        assert!(self.files.len() <= u32::max_value() as usize);
        header.index_entries = self.files.len() as u32;

        let mut index_mask = 0xF7u8; // ignore position, will always be decompressed for now.
        let first = &self.files[0];
        for entry in &self.files[1..] {
            if entry.resource_type != first.resource_type {
                index_mask &= !(1 << 0);
            }
            if entry.resource_group != first.resource_group {
                index_mask &= !(1 << 1);
            }
            if entry.instance != first.instance {
                // FIXME: This is technically two different values...
                index_mask &= !(0b11 << 2);
            }
            if entry.data().len() != first.data().len() || entry.unk1 != first.unk1 {
                // FIXME: as above, except I'm not dealing with compression yet.
                index_mask &= !(0b11 << 5);
            }
            if entry.unk2 != first.unk2 {
                index_mask &= !(1 << 7);
            }
        }
        let index_size = (index_mask.count_ones() + (!index_mask).count_ones() * header.index_entries) * std::mem::size_of::<u32>() as u32;
        let (header_area, rest) = dest.split_at_mut(96);
        let (index_area, file_area) = rest.split_at_mut(index_size as usize);

        header.index_version = 3;
        let mut index_offset = 0;
        if self.files.len() != 0 {
            header.index_size = index_size as u32;
            header.index_position = 96; // located just after header.
            index_area.gwrite::<u32>(index_mask as u32, &mut index_offset)?;
            index_area.gwrite_with(DBPFIndex {
                resource_type: self.files[0].resource_type,
                resource_group: self.files[0].resource_group,
                instance: self.files[0].instance,
                chunk_offset: 0, // this should never be included in the index header.
                filesize: self.files[0].data().len(),
                unk1: self.files[0].unk1,
                memsize: self.files[0].data().len() as u32,
                compressed: false,
                unk2: self.files[0].unk2,
            }, &mut index_offset, IndexType::IndexHeader(index_mask))?;
        } else {
            header.index_size = 0;
            header.index_position = 0;
        }

        header_area.pwrite(header, 0)?;
        let mut file_offset: usize = 0;
        for file in self.files {
            index_area.gwrite_with(DBPFIndex {
                resource_type: file.resource_type,
                resource_group: file.resource_group,
                instance: file.instance,
                chunk_offset: (96 + index_size) as usize + file_offset,
                filesize: file.data().len(),
                unk1: file.unk1,
                memsize: file.data().len() as u32,
                compressed: false,
                unk2: file.unk2,
            }, &mut index_offset, IndexType::IndexEntry(index_mask, DBPFIndex::default()))?;
            file_area.gwrite(file.data(), &mut file_offset)?;
        }

        Ok((96 + index_size) as usize + file_offset)
    }
}

impl<'a> DBPF<'a> {
    pub fn new(mem: &'a [u8]) -> Result<DBPF, scroll::Error> {
        mem.pread::<DBPF>(0)
    }

    // instance -> name
    pub fn gather_names(&self) -> Result<BTreeMap<u64, String>, scroll::Error> {
        let mut map = BTreeMap::new();
        self.files.iter()
            .filter(|e| e.resource_type == filetypes::ResourceType::NMAP as u32)
            .map(|e| filetypes::nmap::gather_names_into(e, &mut map))
            .collect::<Result<_, scroll::Error>>()?;
        Ok(map)
    }
}
