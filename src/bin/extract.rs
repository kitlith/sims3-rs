use std::{env, fs::File};


use sims3_rs::dbpf::DBPFReader;

fn main() -> Result<(), binrw::Error> {
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

    let file = File::open(package_path)?;
    generativity::make_guard!(guard);
    let (mut reader, package) = DBPFReader::parse(std::io::BufReader::new(file), guard)?;

    let entry = package
        .entries
        .iter()
        .find(|entry| {
            entry.resource_type == res_type
                && entry.resource_group == res_group
                && entry.instance == instance
        })
        .unwrap();

    let mut chunk_reader = entry.chunk.get_reader(&mut reader)?;

    let mut output_file = File::create(output_path)?;
    std::io::copy(&mut chunk_reader, &mut output_file)?;

    Ok(())
}
