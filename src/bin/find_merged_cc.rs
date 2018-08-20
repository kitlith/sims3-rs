use self::dbpf::filetypes::ResourceType;
use self::dbpf::DBPF;
use sims3_rs::dbpf;

use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::iter::{FromIterator, IntoIterator, Iterator};
use std::path::Path;

use memmap::Mmap;
use num_traits::ToPrimitive;
use rayon::prelude::*;
use walkdir::WalkDir;

fn filter_tgi_into_map(package: &DBPF<'_>) -> HashSet<(u32, u32, u64)> {
    //println!("DBPF Ver. {}.{}", package.major, package.minor);
    // TODO: Use rayon?
    HashSet::from_iter(
        package
            .files
            .iter()
            .filter(|entry| { // TODO: Find patterns as well.
                // Clothing, hair, etc.
                entry.resource_type == ResourceType::CASP.to_u32().unwrap()
                // Sliders
             || entry.resource_type == ResourceType::FACE.to_u32().unwrap()
                // Skins
             || entry.resource_type == ResourceType::SkinTone.to_u32().unwrap()
                // Objects, if someone uses this for that.
             || entry.resource_type == ResourceType::OBJD.to_u32().unwrap()
                // Patterns, but this gives too many false positives.
             // || entry.resource_type == ResourceType::XMLResource.to_u32().unwrap()
            }).map(|entry| (entry.resource_type, entry.resource_group, entry.instance)),
    )
}

fn main() -> Result<(), scroll::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <merged package> <cc folder>",
            args[0].to_string_lossy()
        );
        return Ok(());
    }

    let find;
    {
        // Turn my file into a byte array usable with scroll.
        // *PLEASE* don't modify the file behind my back.
        let mem = File::open(Path::new(&args[1])).and_then(|f| unsafe { Mmap::map(&f) })?;
        let merged = DBPF::new(&mem)?;
        find = filter_tgi_into_map(&merged);
    }

    WalkDir::new(&args[2])
        .into_iter()
        .par_bridge() // TODO: filter before or after bridging to rayon?
        .filter_map(|e| e.ok())
        .filter(|ref e| e.path().extension() == Some(OsStr::new("package")))
        .for_each(|e| {
            // println!("Testing {}", e.path().display());
            // Turn my file into a byte array usable with scroll.
            // *PLEASE* don't modify the file behind my back.
            let mem = File::open(e.path()).and_then(|f| unsafe { Mmap::map(&f) })
                            .expect("Failed to open file!");
            let package = DBPF::new(&mem).expect("Failed to parse DBPF!");
            let hashes = filter_tgi_into_map(&package);
            let intersection: HashSet<_> = find.intersection(&hashes).collect();
            if !intersection.is_empty() {
                // println!("{}: {:X?}", e.file_name().to_string_lossy(), intersection);
                println!("{}", e.file_name().to_string_lossy());
            }
        });

    Ok(())
}
