use super::ResourceType;
use crate::dbpf;
use crate::dbpf::DBPFEntry;
use scroll::{ctx, Pread};
use std::collections::BTreeMap;
use std::iter::Iterator;

struct NameMapEntry {
    pub instance: u64,
    pub name: String,
}

impl<'a> ctx::TryFromCtx<'a, ()> for NameMapEntry {
    type Error = scroll::Error;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, usize), Self::Error> {
        let instance: u64 = src.pread(0)?;
        let length = src.pread::<u32>(8)? as usize;
        let name: &str = src.pread_with(12, ctx::StrCtx::Length(length))?;
        Ok((
            NameMapEntry {
                instance,
                name: name.to_string(),
            },
            12 + length,
        ))
    }
}

pub fn gather_names_into(
    entry: &DBPFEntry<'_>,
    name_map: &mut BTreeMap<u64, String>,
) -> Result<(), scroll::Error> {

    if entry.resource_type != ResourceType::NMAP as u32 {
        return Err(scroll::Error::Custom("Not an NMAP tag.".to_string()))
    }

    let mut offset = 4;
    // let version: u32 = chunk.pread(0); // We could check the value, but... not relevant rn.
    let count: usize = entry.data().gread::<u32>(&mut offset).expect("Failed to read count!") as usize;
    let mut names: Vec<dbpf::filetypes::nmap::NameMapEntry> = Vec::with_capacity(count);

    unsafe {
        names.set_len(count); // Uninitialized Strings!
        entry.data().gread_inout(&mut offset, &mut names)?;
        // Now either stuff is initialized, or an error was returned.
    }

    names.into_iter().for_each(|dbpf::filetypes::nmap::NameMapEntry { instance, name }| {
        if let Some(old) = name_map.insert(instance, name) {
            println!("Name Table Conflict for instance {:#08X}", instance);
            println!("    Old: {}", old);
            //println!("    New: {}", name); // maybe clone?
        }
    });

    Ok(())
}
