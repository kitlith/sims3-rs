use std::{env, ffi::OsStr, fs::File, io::{Cursor, BufWriter, Write}, path::Path};

use memmap::Mmap;
use rayon::prelude::*;
use walkdir::WalkDir;

use sims3_rs::dbpf::{filetypes::ResourceType, DBPF};

use binrw::{binread, BinRead};

use std::sync::mpsc;

#[binread]
struct MTNF {
    //#[br(dbg)]
    size: u32,
    #[br(pad_after = size)]
    _skip: (),
}

#[binread]
struct VertexAttribute {
    //#[br(dbg)]
    _data_type: u32,
    _sub_type: u32,
    //#[br(dbg)]
    bytes: u8,
}

#[binread]
struct SubMesh {
    index_size: u8,
    index_count: u32,
    #[br(pad_after = index_size as usize * index_count as usize)]
    _index_buffer: (),
}

#[binread]
#[br(magic = b"GEOM")]
#[allow(unused)]
struct Geometry {
    _version: u32,
    _tgi_table_offset: u32,
    _tgi_table_size: u32,

    embedded_material_id: u32,
    #[br(if(embedded_material_id != 0))]
    _embedded_material: Option<MTNF>,

    _merge_group: u32,
    _sort_order: u32,

    vertex_count: u32,
    vertex_attrib_count: u32,
    #[br(count = vertex_attrib_count)]
    vertex_attribs: Vec<VertexAttribute>,
    #[br(pad_after = vertex_attribs.iter().map(|a| a.bytes as usize).sum::<usize>() * vertex_count as usize)]
    _vertex_buffer: (),

    item_count: u32,
    #[br(count = item_count)]
    items: Vec<SubMesh>,
}

fn geom_information(path: &Path, send: &mut mpsc::Sender<(String, usize, usize)>) -> Result<(), scroll::Error> {
    let mem = File::open(path).and_then(|f| unsafe { Mmap::map(&f) })?;
    let package = DBPF::new(&mem)?;

    let mut entries = package
        .files
        .iter()
        .filter(|entry| entry.resource_type == ResourceType::GEOM as u32)
        .map(|entry| {
            let magic = memchr::memmem::find(entry.data(), b"GEOM").unwrap_or_else(|| {
                panic!(
                    "failed to find chunk in {:04x}:{:04x}:{:08x}",
                    entry.resource_type, entry.resource_group, entry.instance
                )
            });
            let geom: Geometry = BinRead::read_le(&mut Cursor::new(&entry.data()[magic..]))
                .unwrap_or_else(|e| {
                    panic!(
                        "failed to parse GEOM in {:04x}:{:04x}:{:08x} -- {}",
                        entry.resource_type, entry.resource_group, entry.instance, e
                    )
                });
            (
                geom.vertex_count,
                geom.items
                    .iter()
                    .map(|i| i.index_count as usize)
                    .sum::<usize>(),
                geom.items.len(),
            )
        })
        .collect::<Vec<_>>();

    if entries.len() == 0 {
        return Ok(());
    }

    entries.sort_by_key(|&(_, count, _)| count);
    entries.reverse();

    let filename = path.file_name().unwrap().to_string_lossy();
    println!(
        "{} -- Polys: {:?}{}{}",
        &filename,
        entries.iter().map(|e| e.1 / 3).collect::<Vec<_>>(),
        if entries[0].1 / 3 < entries[0].0 as usize {
            format!(" ({} vertices)", entries[0].0)
        } else {
            "".to_string()
        },
        if entries[0].2 != 1 {
            format!(" ({} submeshes)", entries[0].2)
        } else {
            "".to_string()
        }
    );

    send.send((filename.into_owned(), entries[0].0 as usize, entries[0].1 / 3)).unwrap();

    Result::<_, scroll::Error>::Ok(())
}

fn main() -> Result<(), scroll::Error> {
    let _ = std::panic::catch_unwind(|| {
        let (send, recv) = mpsc::channel::<(String, usize, usize)>();
        std::thread::spawn(move || {
            let output = File::create(dirs::desktop_dir().unwrap().join("sims3_geom_poly_count.csv")).unwrap();
            let mut output = BufWriter::new(output);
            writeln!(&mut output, "Filename, Max Vertices, Max Polygons").unwrap();
            for (filename, verticies, triangles) in recv {
                writeln!(&mut output, "\"{}\", {}, {}", filename, verticies, triangles).unwrap();
            }
        });
        let args: Vec<_> = env::args_os().collect();
        if args.len() < 2 {
            println!(
                "Usage: {} [packages/directories]",
                args[0].to_string_lossy()
            );
            return;
        }

        args.iter()
            .skip(1)
            .flat_map(WalkDir::new)
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|ref e| e.path().extension() == Some(OsStr::new("package")))
            .for_each_with(send, move |send, e| {
                if let Err(err) = geom_information(e.path(), send) {
                    println!(
                        "Error while parsing {}: {}",
                        e.path().file_name().unwrap().to_string_lossy(),
                        err
                    );
                }
            });
    });

    dont_disappear::any_key_to_continue::default();

    Ok(())
}
