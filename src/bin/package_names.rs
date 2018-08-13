use self::dbpf::filetypes::nmap::gather_names_into;
use self::dbpf::DBPF;
use sims3_rs::dbpf;

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::iter::Iterator;
use std::path::Path;
use std::result::Result;

use memmap::Mmap;

fn main() -> Result<(), scroll::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 2 {
        println!("Usage: {} <package>", args[0].to_string_lossy());
        return Ok(());
    }
    // Turn my file into a byte array usable with scroll.
    // *PLEASE* don't modify the file behind my back.
    let mem = File::open(Path::new(&args[1])).and_then(|f| unsafe { Mmap::map(&f) })?;
    let package = DBPF::new(&mem)?;

    let mut name_map = BTreeMap::new();
    gather_names_into(&package, &mut name_map)?;
    name_map
        .keys()
        .for_each(|key| println!("{:#08X}: '{}'", key, name_map.get(key).unwrap()));

    Ok(())
}
