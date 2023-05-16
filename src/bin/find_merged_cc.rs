use std::path::PathBuf;
use structopt::StructOpt;

use self::dbpf::filetypes::ResourceType;
use self::dbpf::{DBPFReader, DBPF};
use sims3_rs::dbpf;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::iter::{FromIterator, Iterator};

use rayon::prelude::*;
use walkdir::WalkDir;

// TODO: is packageid under dependencyList ever not going to be a 128 bit number?
// What about other languages?
// static ADDON_DEPS: phf::Map<u128, &'static str> = phf_map! {
//     0x062d99d500000000062d99d500000000u128 => "World Adventures Expansion",
//     0x062d99de00000000062d99de00000000u128 => "Ambitions Expansion",
//     0x062d99ed00000000062d99ed00000000u128 => "Late Night Expansion",
//     0x062d9a0200000000062d9a0200000000u128 => "Generations Expansion",
//     0x062d9a0300000000062d9a0300000000u128 => "Pets Expansion",
//     0x062d9a0400000000062d9a0400000000u128 => "Showtime Expansion",
//     0x062d9a0500000000062d9a0500000000u128 => "Supernatural Expansion",
//     0x062d9a0600000000062d9a0600000000u128 => "Seasons Expansion",
//     0x062d9a0600000000062d9a0600000000u128 => "University Expansion",
//     0x062d9a0800000000062d9a0800000000u128 => "Island Paradise Expansion",
//     0x062d9a0900000000062d9a0900000000u128 => "Into The Future Expansion",
//     0x062d9a0900000000062d9a0900000000u128 => "High-End Loft Stuff Pack",
//     0x062d9b1000000001062d9b1000000001u128 => "Fast Lane Stuff Pack",
//     0x062d9b1100000001062d9b1100000001u128 => "Outdoor Living Stuff Pack",
//     0x062d9b1200000001062d9b1200000001u128 => "Town Life Stuff Pack",
//     0x062d9b1300000001062d9b1300000001u128 => "Master Suite Stuff Pack",
//     0x062d9b1400000001062d9b1400000001u128 => "Katy Perry's Sweet Treats Stuff Pack",
//     0x062d9b1500000001062d9b1500000001u128 => "Diesel Stuff Pack",
//     0x062d9b1600000001062d9b1600000001u128 => "70's, 80's and 90's Stuff Pack",
//     0x062d9b1700000001062d9b1700000001u128 => "Movie Stuff Pack"
// };

fn filter_tgi_into_map<Ctx>(package: &DBPF<'_, Ctx>, merged: bool) -> HashSet<(u32, u32, u64)> {
    //println!("DBPF Ver. {}.{}", package.major, package.minor);
    // TODO: Use rayon?
    let tgi_set = HashSet::from_iter(
        package
            .entries
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
            })
            .map(|entry| (entry.resource_type, entry.resource_group, entry.instance)),
    );

    if !tgi_set.is_empty() {
        tgi_set
    } else {
        // If there's nothing else of interest, now pattern XMLs are interesting.
        HashSet::from_iter(
            package
                .entries
                .iter()
                .filter(|entry| entry.resource_type == ResourceType::XMLResource as u32)
                .map(|entry| (entry.resource_type, entry.resource_group, entry.instance)),
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
    #[structopt(name = "DIR", parse(from_os_str), required = true, min_values = 1)]
    search_dirs: Vec<PathBuf>,
}

fn main() -> Result<(), binrw::Error> {
    let opt = Opt::from_args();

    let find;
    {
        let file = File::open(&opt.input_file)?;
        generativity::make_guard!(guard);
        let (mut _reader, merged) = DBPFReader::parse(std::io::BufReader::new(file), guard)?;
        find = filter_tgi_into_map(&merged, true);
    }

    // Maybe parse Resource.cfg if present?
    opt.search_dirs
        .iter()
        .flat_map(|path| WalkDir::new(path))
        .par_bridge() // TODO: filter before or after bridging to rayon?
        .filter_map(|e| e.ok())
        .filter(|ref e| e.path().extension() == Some(OsStr::new("package")))
        .for_each(|e| {
            // println!("Testing {}", e.path().display());
            // Turn my file into a byte array usable with scroll.
            // *PLEASE* don't modify the file behind my back.
            let file = File::open(e.path()).expect("Failed to open file!");
            generativity::make_guard!(guard);
            let (mut _reader, package) = DBPFReader::parse(std::io::BufReader::new(file), guard)
                .expect("Failed to parse DBPF!");

            let hashes = filter_tgi_into_map(&package, false);
            let intersection: HashSet<_> = find.intersection(&hashes).collect();
            if !intersection.is_empty() {
                // println!("{}: {:X?}", e.file_name().to_string_lossy(), intersection);
                // TODO: print relative path
                println!(
                    "{}",
                    if opt.full_path {
                        e.path().to_string_lossy()
                    } else {
                        e.file_name().to_string_lossy()
                    }
                );
            }
        });

    Ok(())
}
