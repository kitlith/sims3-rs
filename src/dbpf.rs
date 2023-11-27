pub mod filetypes;

use binrw::io::TakeSeekExt;
use binrw::{binrw, io, BinRead, BinResult};
use std::collections::BTreeMap;
use std::io::Read;
use std::marker::PhantomData;

use bilge::prelude::*;

macro_rules! dbpf_index_entry {
    (
        $( #[$meta:meta] )*
        $vis:vis struct $name:ident {
            $(
                $( #[$field_meta:meta] )*
                $field_vis:vis $field_name:ident : $field_ty:ty
            ),*
            $(,)?
        }
    ) => {
        ::paste::paste! {
            #[bitsize(32)]
            #[::binrw::binrw]
            #[br(try_map = |b: u32| [<$name Mask>]::try_from(b))]
            #[bw(map = |bf: &[<$name Mask>]| bf.value)]
            #[derive(Copy, Clone, TryFromBits, DebugBits, Default)]
            #[doc = concat!(
                "Generated bitfield for [`", stringify!($name), "`].\n",
                "\n",
                "Each field corresponds to a field in [`", stringify!($name), "`] and [`", stringify!([<Partial $name>]), "`],\n",
                "and controls whether that field is read or written at that time."
            )]
            struct [<$name Mask>] {
                $(
                    $field_name : bool
                ),* ,
                // HACK: bilge doesn't support unfilled structs
                reserved: u24
            }

            impl [<$name Mask>] {
                fn count_present(&self) -> u32 {
                    0
                    $(
                        + self.$field_name() as u32
                    )*
                }
            }

            #[::binrw::binrw]
            #[br(import(mask: [<$name Mask>]))]
            #[derive(Clone, Default)]
            #[doc = concat!(
                "Generated 'partial' struct for [`", stringify!($name), "`].\n",
                "\n",
                "Each field corresponds to a field in [`", stringify!($name), "`], where each field's type is wrapped in [`Option`].\n",
                "This is used to read/write the common/deduplicated items of the index."
            )]
            struct [<Partial $name>] {
                $(
                    #[br(if(mask.$field_name()))]
                    $field_name : ::core::option::Option<$field_ty>
                ),*
            }

            impl [<Partial $name>] {
                fn calc_common_entries<'a>(mut iter: impl Iterator<Item=&'a $name>) -> Self {
                    let mut common: Self = if let Some(i) = iter.next() {
                        i.into()
                    } else {
                        return Self::default();
                    };

                    for entry in iter {
                        $(
                            if common.$field_name.map(|f| f == entry.$field_name).unwrap_or(false) {
                                common.$field_name = None;
                            }
                        )*
                    }

                    common
                }

                fn calc_mask(&self) -> [<$name Mask>] {
                    let mut res = [<$name Mask>]::default();
                    $(
                        res.[<set_ $field_name>](self.$field_name.is_some());
                    )*
                    res
                }
            }

            #[::binrw::binrw]
            $( #[$meta] )*
            #[br(import(partial: /*&*/ [<Partial $name>]))]
            #[bw(import(mask: [<$name Mask>]))]
            #[doc = concat!(
                "A full index entry.\n",
                "\n",
                "While reading, fields are copied from a [`", stringify!([<Partial $name>]), "`] passed as an argument,\n",
                "and missing fields are read from the file.\n",
                "\n",
                "While writing, only the fields that are unset in the [`", stringify!([<$name Mask>]), "`] passed as an argument are written."
            )]
            $vis struct $name {
                $(
                    $( #[$field_meta] )*
                    // Fill in fields from the common fields, and otherwise read them
                    #[br(if(partial.$field_name.is_none(), partial.$field_name.unwrap()))]
                    // Only write if the field is *not* in the common field mask
                    #[bw(if(!mask.$field_name()))]
                    $field_vis $field_name : $field_ty
                ),*
            }

            impl From<&$name> for [<Partial $name>] {
                fn from(value: &$name) -> Self {
                    [<Partial $name>] {
                        $(
                            $field_name: Some(value.$field_name)
                        ),*
                    }
                }
            }
        }
    }
}

#[bitsize(32)]
#[derive(Clone, Copy, PartialEq, FromBits, DebugBits)]
#[binrw]
#[br(map = |b: u32| b.into())]
#[bw(map = |bf: &IndexFilesize| bf.value)]
struct IndexFilesize {
    filesize: u31,
    unk1: bool,
}

dbpf_index_entry! {
    struct IndexEntry {
        resource_type: u32,
        resource_group: u32, // TODO: Flags?
        instance_hi: u32,
        instance_lo: u32,
        chunk_offset: u32,
        filesize_unk1: IndexFilesize,
        memsize: u32,
        compressed_unk2: (u16, u16),
    }
}

#[derive(Debug, Clone)]
pub enum ChunkHandle<'brand> {
    Uncompressed {
        offset: u32,
        filesize: u31,
        brand: generativity::Id<'brand>,
    },
    Compressed {
        offset: u32,
        filesize: u31,
        memsize: u32,
        decompressed: Option<Vec<u8>>, // can be cleared
        brand: generativity::Id<'brand>,
    },
    Dirty {
        decompressed: Vec<u8>,
        should_compress: bool,
        // the brand is ditched in the dirty state,
        // will make it required to read from the file
    },
}

impl ChunkHandle<'_> {
    pub fn memsize(&self) -> u32 {
        match self {
            ChunkHandle::Compressed { memsize, .. } => *memsize,
            ChunkHandle::Uncompressed { filesize, .. } => (*filesize).into(),
            ChunkHandle::Dirty { decompressed, .. } => decompressed.len() as u32,
        }
    }
}

pub trait ReadSeek: io::Read + io::Seek {}
impl<T: io::Read + io::Seek> ReadSeek for T {}

enum ChunkReader<'a, R> {
    CursorBorrow(io::Cursor<&'a [u8]>),
    // TODO: convert to borrow from cache
    CursorOwned(io::Cursor<Vec<u8>>),
    Reader(R),
}

impl<'a, R: io::Read> io::Read for ChunkReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            ChunkReader::CursorBorrow(r) => r.read(buf),
            ChunkReader::CursorOwned(r) => r.read(buf),
            ChunkReader::Reader(r) => r.read(buf),
        }
    }
}

impl<'a, R: io::Seek> io::Seek for ChunkReader<'a, R> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            ChunkReader::CursorBorrow(r) => r.seek(pos),
            ChunkReader::CursorOwned(r) => r.seek(pos),
            ChunkReader::Reader(r) => r.seek(pos),
        }
    }
}

impl<'brand> ChunkHandle<'brand> {
    pub fn get_reader<'a, Ctx: FileCtx<'brand>>(
        &'a self,
        ctx: &'a mut Ctx,
    ) -> io::Result<impl ReadSeek + 'a> {
        match self {
            ChunkHandle::Uncompressed {
                offset,
                filesize,
                brand,
            } => Ok(ChunkReader::Reader(ctx.get_chunk_reader(
                *offset as u64,
                (*filesize).into(),
                brand,
            )?)),
            ChunkHandle::Compressed {
                offset,
                filesize,
                memsize: _,      // TODO: check against decompression output
                decompressed: _, // TODO: caching
                brand,
            } => {
                let mut reader =
                    ctx.get_chunk_reader(*offset as u64, (*filesize).into(), &brand)?;
                let mut compressed = Vec::new();
                reader.read_to_end(&mut compressed)?;
                // TODO: cache this
                let decompressed =
                    refpack::easy_decompress::<refpack::format::SimEA>(&compressed)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(ChunkReader::CursorOwned(io::Cursor::new(decompressed)))
            }
            ChunkHandle::Dirty { decompressed, .. } => {
                Ok(ChunkReader::CursorBorrow(io::Cursor::new(decompressed)))
            }
        }
    }

    // only works on already read compressed items, and unwritten dirty items
    // fn try_get_reader(&self) -> Option<impl ReadSeek + '_> {
    //     match self {
    //         ChunkHandle::Uncompressed { .. } => None,
    //         ChunkHandle::Compressed { decompressed, .. } => {
    //             decompressed.as_ref().map(io::Cursor::new)
    //         }
    //         ChunkHandle::Dirty { decompressed, .. } => Some(io::Cursor::new(decompressed)),
    //     }
    // }
}

#[derive(Clone, Debug)]
pub struct DBPFIndexEntry<'brand> {
    pub resource_type: u32,
    pub resource_group: u32,
    pub instance: u64,
    // chunk_offset: u32,
    // filesize: u32,
    pub unk1: bool,
    // memsize: u32,
    // compressed: bool, // expose a way to request/disable compression?
    pub unk2: u16,
    pub chunk: ChunkHandle<'brand>,
}

impl<'brand> DBPFIndexEntry<'brand> {
    fn from_raw(value: IndexEntry, brand: generativity::Id<'brand>) -> Self {
        DBPFIndexEntry {
            resource_type: value.resource_type,
            resource_group: value.resource_group,
            instance: ((value.instance_hi as u64) << 32) | value.instance_lo as u64,
            unk1: value.filesize_unk1.unk1(),
            unk2: value.compressed_unk2.1,
            chunk: if value.compressed_unk2.0 != 0 {
                ChunkHandle::Compressed {
                    offset: value.chunk_offset,
                    filesize: value.filesize_unk1.filesize(),
                    memsize: value.memsize,
                    decompressed: None,
                    brand,
                }
            } else {
                ChunkHandle::Uncompressed {
                    offset: value.chunk_offset,
                    filesize: value.filesize_unk1.filesize(),
                    brand,
                }
            },
        }
    }
}

impl IndexEntry {
    fn from_nice<'brand>(value: &DBPFIndexEntry<'brand>, current_offset: &mut u32) -> Self {
        let chunk_offset = *current_offset;
        let chunk_filesize: u31;
        let chunk_memsize: u32;
        let compressed;
        match value.chunk {
            ChunkHandle::Uncompressed { filesize, .. } => {
                compressed = false;
                chunk_memsize = filesize.into();
                chunk_filesize = filesize;
            }
            ChunkHandle::Compressed {
                filesize, memsize, ..
            } => {
                compressed = true;
                chunk_memsize = memsize;
                chunk_filesize = filesize;
            }
            ChunkHandle::Dirty {
                ref decompressed,
                should_compress: _,
            } => {
                compressed = false; // TODO
                chunk_memsize = decompressed.len().try_into().unwrap();
                chunk_filesize = u31::try_new(decompressed.len().try_into().unwrap()).unwrap();
            }
        }
        *current_offset += u32::from(chunk_filesize);
        IndexEntry {
            resource_type: value.resource_type,
            resource_group: value.resource_group,
            instance_hi: (value.instance >> 32) as u32,
            instance_lo: value.instance as u32,
            chunk_offset,
            filesize_unk1: IndexFilesize::new(chunk_filesize, value.unk1),
            memsize: chunk_memsize,
            compressed_unk2: (if compressed { 0xFFFF } else { 0 }, value.unk2),
        }
    }
}

mod private {
    pub trait SealedCtx {}
    impl SealedCtx for () {}
    impl<'brand, Read> SealedCtx for super::DBPFReader<'brand, Read> {}

    // pub trait SealedReader {}
    // impl<T: binrw::io::Read + binrw::io::Seek> SealedReader for T {}
}

pub enum NeverReader {}
impl io::Read for NeverReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        unreachable!()
    }
}
impl io::Seek for NeverReader {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        unreachable!()
    }
}

pub trait FileReader: private::SealedCtx {
    type ChunkReader<'r>: io::Read + io::Seek
    where
        Self: 'r;
}
pub trait FileCtx<'brand>: FileReader {
    fn get_chunk_reader<'a>(
        &'a mut self,
        pos: u64,
        size: u64,
        _brand: &generativity::Id<'brand>,
    ) -> io::Result<Self::ChunkReader<'a>>;
}
impl FileReader for () {
    type ChunkReader<'r> = NeverReader where Self: 'r;
}
impl FileCtx<'static> for () {
    fn get_chunk_reader(
        &mut self,
        _pos: u64,
        _size: u64,
        _brand: &generativity::Id<'static>,
    ) -> io::Result<NeverReader> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to get chunk reader from unwritten file!",
        ))
    }
}
impl<'brand, Read: io::Read + io::Seek> FileReader for DBPFReader<'brand, Read> {
    type ChunkReader<'r> = io::TakeSeek<&'r mut Read> where Self: 'r;
}
impl<'brand, Read: io::Read + io::Seek> FileCtx<'brand> for DBPFReader<'brand, Read> {
    fn get_chunk_reader<'a>(
        &'a mut self,
        pos: u64,
        size: u64,
        _brand: &generativity::Id<'brand>,
    ) -> io::Result<io::TakeSeek<&'a mut Read>> {
        self.0.seek(io::SeekFrom::Start(pos))?;
        // TODO: this is buggy, since seeks in the resulting reader are relative to the original file!
        Ok((&mut self.0).take_seek(size))
    }
}

pub struct DBPFReader<'brand, Read>(Read, generativity::Id<'brand>);

impl<'brand, Read> DBPFReader<'brand, Read>
where
    Read: io::Read + io::Seek,
{
    pub fn parse(
        mut reader: Read,
        guard: generativity::Guard<'brand>,
    ) -> BinResult<(Self, DBPF<'brand, Self>)> {
        let id: generativity::Id = guard.into();
        let dbpf = DBPF::<'brand>::read(&mut reader, id.clone())?;
        Ok((DBPFReader(reader, id), dbpf))
    }
}

#[derive(Debug)]
pub struct DBPF<'brand, Ctx> {
    pub maybe_flags: u32,
    pub created_timestamp: u32,  // usually 0
    pub modified_timestamp: u32, // usually 0
    pub entries: Vec<DBPFIndexEntry<'brand>>,
    phantom: PhantomData<Ctx>,
}

impl DBPF<'static, ()> {
    pub fn new() -> Self {
        DBPF {
            maybe_flags: 0,
            created_timestamp: 0,
            modified_timestamp: 0,
            entries: Vec::new(),
            phantom: PhantomData,
        }
    }
}

// This is *not* a BinRead impl, so that it can be private and I can make it take an Id<'_> instead of a Guard Id<'_>
impl<'brand, Ctx: FileCtx<'brand>> DBPF<'brand, Ctx> {
    fn read<R: io::Read + io::Seek>(
        reader: &mut R,
        brand: generativity::Id<'brand>,
    ) -> BinResult<Self> {
        let header: DBPFHeader = BinRead::read_le(reader)?;

        // index
        reader.seek(io::SeekFrom::Start(header.index_position as u64))?;
        let mask: IndexEntryMask = BinRead::read_le(reader)?;
        let common: PartialIndexEntry = BinRead::read_le_args(reader, (mask,))?;
        let entries_args = binrw::VecArgs {
            count: header.index_entries as usize,
            inner: (common,),
        };
        let entries: Vec<IndexEntry> = BinRead::read_le_args(reader, entries_args)?;
        let entries = entries
            .into_iter()
            .map(|e| DBPFIndexEntry::from_raw(e, brand.clone()))
            .collect();

        Ok(Self {
            maybe_flags: header.maybe_flags,
            created_timestamp: header.created_timestamp,
            modified_timestamp: header.modified_timestamp,
            entries,
            phantom: PhantomData,
        })
    }
}

impl<'brand, Ctx: FileCtx<'brand>> binrw::BinWrite for DBPF<'brand, Ctx> {
    type Args<'a> = Ctx;

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        mut args: Self::Args<'_>,
    ) -> BinResult<()> {
        // we write the contents after both the header and index.
        // we don't know how large the index is going to be yet,
        // so we'll start after the header and add the size of the index later.
        let mut current_offset = 96u32;
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .map(|e| IndexEntry::from_nice(e, &mut current_offset))
            .collect();
        let common = PartialIndexEntry::calc_common_entries(entries.iter());
        let mask = common.calc_mask();

        // (mask + common_fields + different_fields * num_entries) * 4
        let index_size =
            (1 + mask.count_present() + (8 - mask.count_present()) * (entries.len() as u32)) * 4;
        entries
            .iter_mut()
            .for_each(|e| e.chunk_offset += index_size);

        let header = DBPFHeader {
            maybe_flags: self.maybe_flags,
            created_timestamp: self.created_timestamp,
            modified_timestamp: self.modified_timestamp,
            index_entries: self.entries.len() as u32,
            index_position_old: 0,
            index_size,
            hole_index_count: 0,
            hole_index_position: 0,
            hole_index_size: 0,
            index_position: 96,
            _index_major: 7,
        };

        header.write_le(writer)?;
        mask.write_le(writer)?;
        common.write_le(writer)?;
        entries.write_le_args(writer, (mask,))?;
        binrw::BinWrite::write_options(&mask, writer, endian, ())?;
        binrw::BinWrite::write_options(&common, writer, endian, ())?;
        binrw::BinWrite::write_options(&entries, writer, endian, (mask,))?;
        for chunk in self.entries.iter().map(|e| &e.chunk) {
            std::io::copy(&mut chunk.get_reader(&mut args)?, writer)?;
        }
        Ok(())
    }
}

// 96 bytes in file.
#[binrw]
#[derive(Debug, PartialEq, Default)]
#[brw(magic = b"DBPF")]
struct DBPFHeader {
    // Would it be nice to expand to more versions of DBPF eventually?
    // Sure. But not right now.
    #[br(temp)]
    #[brw(magic = 2u32, calc = ())]
    _major: (), // 2
    #[br(temp)]
    #[brw(magic = 0u32, calc = ())]
    _minor: (), // 0
    #[br(temp)]
    #[brw(magic = 0u32, calc = ())]
    _major_user: (), // 0
    #[br(temp)]
    #[brw(magic = 0u32, calc = ())]
    _minor_user: (), // 0

    maybe_flags: u32,
    created_timestamp: u32,  // usually 0
    modified_timestamp: u32, // usually 0

    // #[br(temp)]
    // #[brw(magic = 7u32, calc = ())]
    _index_major: u32,

    index_entries: u32,
    index_position_old: u32, // "Index Location (DBPF 1.x)"
    index_size: u32,
    // hole index is usually empty, thus all 0
    hole_index_count: u32,
    hole_index_position: u32,
    hole_index_size: u32,

    #[br(temp)]
    #[brw(magic = 3u32, calc = ())]
    _index_minor: (),
    #[brw(pad_after = 28)]
    index_position: u32,
    //reserved: [u8; 28],
}

impl<'brand, Ctx: FileCtx<'brand>> DBPF<'brand, Ctx> {
    // instance -> name
    pub fn gather_names(&self, ctx: &mut Ctx) -> Result<BTreeMap<u64, String>, binrw::Error> {
        let mut map = BTreeMap::new();
        self.entries
            .iter()
            .filter(|e| e.resource_type == filetypes::ResourceType::NMAP as u32)
            .map(|e| filetypes::nmap::gather_names_into(ctx, e, &mut map))
            .collect::<Result<_, binrw::Error>>()?;
        Ok(map)
    }
}
