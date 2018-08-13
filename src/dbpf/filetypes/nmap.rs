use super::ResourceType;
use crate::dbpf;
use crate::dbpf::DBPF;
use num_traits::ToPrimitive;
use scroll::{ctx, Pread};
use std::collections::BTreeMap;

struct NameMapEntry {
    pub instance: u64,
    pub name: String,
}

impl<'a> ctx::TryFromCtx<'a, ()> for NameMapEntry {
    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, Self::Size), Self::Error> {
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

// TODO: This should move into dbpf::filetypes::nmap
pub fn gather_names_into(
    package: &DBPF<'_>,
    name_map: &mut BTreeMap<u64, String>,
) -> Result<(), scroll::Error> {
    package.files.iter()
    .filter(|entry| entry.resource_type == ResourceType::NMAP.to_u32().unwrap())
    .flat_map(|entry| {
        let mut offset = 4;
        // let version: u32 = chunk.pread(0); // We could check the value, but... not relevant rn.
        let count: usize = entry.data().gread::<u32>(&mut offset).expect("Failed to read count!") as usize;
        let mut names: Vec<dbpf::filetypes::nmap::NameMapEntry> = Vec::with_capacity(count);

        unsafe { names.set_len(count); }
        // references are now uninitialized!
        if let Err(_) = entry.data().gread_inout(&mut offset, &mut names) {
            println!("Failed to read name map!");
            names.truncate(0);
        }
        // Now it's all initialized, or it's empty because an error occured.
        // TODO: Pass error through map?
        names
    }).for_each(|dbpf::filetypes::nmap::NameMapEntry { instance, name }| {
        if let Some(old) = name_map.insert(instance, name) {
            println!("Name Table Conflict for instance {:#08X}", instance);
            println!("    Old: {}", old);
            //println!("    New: {}", name); // maybe clone?
        }
    });

    Ok(())
}
