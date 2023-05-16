use self::dbpf::DBPFReader;
use sims3_rs::dbpf;

use std::env;
use std::fs::File;
use std::iter::Iterator;
use std::result::Result;

fn main() -> Result<(), binrw::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <merged package> <cc folder>",
            args[0].to_string_lossy()
        );
        return Ok(());
    }

    let file = File::open(&args[1])?;
    generativity::make_guard!(guard);
    let (mut reader, package) = DBPFReader::parse(std::io::BufReader::new(file), guard)?;

    for entry in package
        .entries
        .iter()
        .filter(|entry| dbpf::filetypes::resource_is_png(entry.resource_type))
    {
        let mut f = File::create(format!("{:016X}.png", entry.instance))?;
        std::io::copy(&mut entry.chunk.get_reader(&mut reader)?, &mut f)?;
    }

    Ok(())
}
