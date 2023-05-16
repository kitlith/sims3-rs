use super::ResourceType;
use crate::dbpf::{DBPFIndexEntry, FileCtx};
use std::collections::BTreeMap;

use binrw::{binrw, BinRead, BinResult};

use crate::util::{write_btreemap, LengthString};

#[binrw]
#[brw(magic = 1u32)] // version
struct NMAPHeader {
    count: u32,
}

impl TryFrom<usize> for NMAPHeader {
    type Error = <u32 as TryFrom<usize>>::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(NMAPHeader {
            count: value.try_into()?,
        })
    }
}

#[binrw]
pub struct NMAP {
    #[br(temp)]
    #[bw(try_calc = map.len().try_into() )]
    header: NMAPHeader,
    // TODO: allow HashMap?
    #[br(parse_with = binrw::helpers::count(header.count as usize))]
    #[bw(write_with = write_btreemap)]
    map: BTreeMap<u64, LengthString>,
}

pub fn gather_names_into<'brand>(ctx: &mut impl FileCtx<'brand>, entry: &DBPFIndexEntry<'brand>, name_map: &mut BTreeMap<u64, String>) -> BinResult<()> {
    if entry.resource_type != ResourceType::NMAP as u32 {
        return Err(binrw::Error::AssertFail { pos: 0, message: "Not an NMAP tag.".to_string() });
    }

    let mut reader = entry.chunk.get_reader(ctx)?;
    let nmap: NMAP = BinRead::read_le(&mut reader)?;
    name_map.extend(nmap.map.into_iter().map(|(i, s)| (i, String::from_utf8_lossy(&s.inner).into_owned())));

    Ok(())
}