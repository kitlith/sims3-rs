use std::path::PathBuf;
use structopt::StructOpt;

use self::dbpf::filetypes::ResourceType;
use self::dbpf::DBPF;
use sims3_rs::dbpf;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::iter::{FromIterator, Iterator};

use memmap::Mmap;
use rayon::prelude::*;
use walkdir::WalkDir;

fn filter_tgi_into_map(package: &DBPF<'_>, merged: bool) -> HashSet<(u32, u32, u64)> {
    //println!("DBPF Ver. {}.{}", package.major, package.minor);
    // TODO: Use rayon?
    let tgi_set = HashSet::from_iter(
        package
            .files
            .iter()
            .filter(|entry| {
                // Clothing, hair, etc.
                entry.resource_type == ResourceType::CASP as u32
                // Sliders
             || entry.resource_type == ResourceType::FACE as u32
                // Skins
             || entry.resource_type == ResourceType::SkinTone as u32
                // Objects, if someone uses this for that.
             || entry.resource_type == ResourceType::OBJD as u32
                // Patterns -- but only if this is a merged package that I'm searching.
                // Clothing has duplicates, so unless this is the package I'm searching I only
                // want to see it if there's nothing else.
             || (merged && entry.resource_type == ResourceType::XMLResource as u32)
            }).map(|entry| (entry.resource_type, entry.resource_group, entry.instance)),
    );

    if !tgi_set.is_empty() {
        tgi_set
    } else { // If there's nothing else of interest, now pattern XMLs are interesting.
        HashSet::from_iter(
            package.files.iter()
                .filter(|entry| entry.resource_type == ResourceType::XMLResource as u32)
                .map(|entry| (entry.resource_type, entry.resource_group, entry.instance))
        )
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "find_merged_cc")]
struct Opt {
    /// Print full paths instead of just the package filenames
    #[structopt(short = "v", long = "full")]
    full_path: bool,

    /// Merged file that contains custom content
    #[structopt(name = "PACKAGE", parse(from_os_str))]
    input_file: PathBuf,

    /// Directories to search for custom content in
    #[structopt(
        name = "DIR",
        parse(from_os_str),
        raw(required = "true", min_values = "1")
    )]
    search_dirs: Vec<PathBuf>,
}

fn main() -> Result<(), scroll::Error> {
    let opt = Opt::from_args();

    let find;
    {
        // Turn my file into a byte array usable with scroll.
        // *PLEASE* don't modify the file behind my back.
        let mem = File::open(&opt.input_file).and_then(|f| unsafe { Mmap::map(&f) })?;
        let merged = DBPF::new(&mem)?;
        find = filter_tgi_into_map(&merged, true);
    }

    // Maybe parse Resource.cfg if present?
    opt.search_dirs.iter()
        .flat_map(|path| WalkDir::new(path))
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
            let hashes = filter_tgi_into_map(&package, false);
            let intersection: HashSet<_> = find.intersection(&hashes).collect();
            if !intersection.is_empty() {
                // println!("{}: {:X?}", e.file_name().to_string_lossy(), intersection);
                // TODO: print relative path
                println!("{}", if opt.full_path { e.path().to_string_lossy() } else { e.file_name().to_string_lossy() });
            }
        });

    Ok(())
}
