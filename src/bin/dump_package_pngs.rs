use self::dbpf::DBPF;
use sims3_rs::dbpf;

use std::env;
use std::fs::File;
use std::io::Write;
use std::iter::Iterator;
use std::path::Path;
use std::result::Result;

use memmap::Mmap;

fn main() -> Result<(), scroll::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <merged package> <cc folder>",
            args[0].to_string_lossy()
        );
        return Ok(());
    }

    let mem = File::open(Path::new(&args[1])).and_then(|f| unsafe { Mmap::map(&f) })?;
    let package = DBPF::new(&mem)?;

    for entry in package
        .files
        .iter()
        .filter(|entry| dbpf::filetypes::resource_is_png(entry.resource_type))
    {
        let mut f = File::create(format!("{:016X}.png", entry.instance))?;
        f.write_all(entry.data())?;
    }

    Ok(())
}
