use sims3_rs::dbpf::filetypes::ResourceType;
use sims3_rs::dbpf::DBPFReader;

use std::env;
use std::fs::File;
use std::iter::Iterator;
use std::path::Path;
use std::result::Result;

use num_traits::ToPrimitive;

fn main() -> Result<(), binrw::Error> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 2 {
        println!("Usage: {} <package>", args[0].to_string_lossy());
        return Ok(());
    }

    let package_path = Path::new(&args[1]);
    let name_map;
    let tag_name = {
        let file = File::open(package_path)?;
        generativity::make_guard!(guard);
        let (mut reader, package) = DBPFReader::parse(std::io::BufReader::new(file), guard)?;

        name_map = package.gather_names(&mut reader).unwrap();
        package
            .entries
            .iter()
            .filter(|e| {
                e.resource_type == ResourceType::CASP.to_u32().unwrap()
                    || e.resource_type == ResourceType::OBJD.to_u32().unwrap()
                    || e.resource_type == ResourceType::NMAP.to_u32().unwrap()
                    || e.resource_type == 0xB52F5055
            }) // FBLN
            .find_map(|e| name_map.get(&e.instance))
    };
    if let Some(name) = tag_name {
        let new_path = package_path.with_file_name(format!("{}.package", name));
        print!(
            "'{}' -> '{}'",
            package_path.to_string_lossy(),
            new_path.to_string_lossy()
        );
        if new_path.exists() {
            println!(" but destination already exists! Ignoring.");
        } else {
            println!("");
            std::fs::rename(package_path, new_path)?;
        }
    } else {
        println!(
            "Unable to find a name for '{}'!",
            package_path.to_string_lossy()
        );
    }
    Ok(())
}
