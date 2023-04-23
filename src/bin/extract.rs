use std::{env, fs::File, path::Path};

use memmap::Mmap;

use sims3_rs::dbpf::DBPF;

fn main() -> Result<(), scroll::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 4 {
        println!(
            "Usage: {} <package> <T:G:I> <output>",
            args[0].to_string_lossy()
        );
        return Ok(());
    }

    let package_path = &args[1];

    let tgi = args[2].to_string_lossy();
    let mut tgi = tgi.split(':');
    let res_type = u32::from_str_radix(tgi.next().unwrap(), 16).unwrap();
    let res_group = u32::from_str_radix(tgi.next().unwrap(), 16).unwrap();
    let instance = u64::from_str_radix(tgi.next().unwrap(), 16).unwrap();
    assert!(tgi.next().is_none());

    let output_path = &args[3];

    let mem = File::open(Path::new(package_path)).and_then(|f| unsafe { Mmap::map(&f) })?;
    let package = DBPF::new(&mem)?;

    let data = package
        .files
        .iter()
        .find(|entry| {
            entry.resource_type == res_type
                && entry.resource_group == res_group
                && entry.instance == instance
        })
        .unwrap()
        .data();

    std::fs::write(output_path, data).unwrap();

    Ok(())
}
