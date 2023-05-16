use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::{BufWriter, Cursor, Write},
    panic::catch_unwind,
    path::Path,
};

use rayon::prelude::*;
use walkdir::WalkDir;

use sims3_rs::dbpf::{filetypes::ResourceType, DBPFReader};

use binrw::{binread, error::ContextExt, io, BinRead, PosValue};

use std::sync::mpsc;

#[binread]
#[derive(Debug, Clone, Copy)]
struct ITG {
    instance: u64,
    ty: u32,
    group: u32,
}

impl Display for ITG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TGI({:08x}:{:08x}:{:016x})",
            self.ty, self.group, self.instance
        )
    }
}

#[derive(Debug, BinRead, Clone, Copy)]
struct Dummy;

impl binrw::file_ptr::IntoSeekFrom for Dummy {
    fn into_seek_from(self) -> io::SeekFrom {
        io::SeekFrom::Current(0)
    }
}

type Placement<T> = binrw::FilePtr<Dummy, T>;

#[binread]
#[br(import(pos: u64))]
struct RCOLChunk {
    #[br(temp)]
    position: u32,
    #[br(temp)]
    size: u32,
    #[br(args { offset: position as u64 + pos, inner: binrw::args! { count: size as usize }})]
    data: Placement<Vec<u8>>,
}

#[binread]
struct RCOL {
    #[br(temp)]
    pos: PosValue<()>,
    #[brw(magic = 3u32)]
    _version: (),
    #[br(temp)]
    public_internal_count: u32,
    #[br(temp)]
    unused: u32,
    #[br(temp)]
    external_count: u32,
    #[br(temp)]
    internal_count: u32,
    #[br(count = internal_count)]
    _internal_idents: Vec<ITG>,
    #[br(count = external_count)]
    _references: Vec<ITG>,
    #[br(args { count: internal_count as usize, inner: (pos.pos,)})]
    chunks: Vec<RCOLChunk>,
}

#[binread]
struct MTNF {
    #[br(temp)]
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
    #[br(temp)]
    index_size: u8,
    index_count: u32,
    #[br(pad_after = index_size as usize * index_count as usize)]
    _index_buffer: (), // TODO: read data
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

fn geom_information(path: &Path) -> Result<Option<(String, usize, usize)>, binrw::Error> {
    let file = File::open(path)?;
    generativity::make_guard!(guard);
    let (mut reader, package) = DBPFReader::parse(std::io::BufReader::new(file), guard)?;

    let mut entries = package
        .entries
        .iter()
        .filter(|entry| entry.resource_type == ResourceType::GEOM as u32)
        // Workaround: I've found *one* file that has LODs "present" but a size of 0, therefore it immediately fails to read anything.
        //  This prevents it from printing an error message for this case.
        .filter(|entry| entry.chunk.memsize() != 0)
        .map(|entry| {
            let mut reader = entry.chunk.get_reader(&mut reader)?;
            let itg = ITG {
                ty: entry.resource_type,
                group: entry.resource_group,
                instance: entry.instance,
            };
            let rcol = RCOL::read_le(&mut reader)
                .with_message("parsing RCOL")
                .with_context(itg)?;

            let geom: Geometry = BinRead::read_le(&mut Cursor::new(
                &rcol.chunks[0].data.value.as_ref().unwrap(),
            ))
            .with_message("parsing GEOM")
            .with_context(itg)?;
            Ok::<_, binrw::Error>((
                geom.vertex_count,
                geom.items
                    .iter()
                    .map(|i| i.index_count as usize)
                    .sum::<usize>(),
                geom.items.len(),
            ))
        })
        .filter_map(|r| match r {
            Ok(res) => Some(res),
            Err(e) => {
                let e = e.with_message(path.to_string_lossy().into_owned());
                println!("Chunk Error: {}", e);
                None
            }
        })
        .collect::<Vec<_>>();

    if entries.len() == 0 {
        return Ok(None);
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

    Ok(Some((
        filename.into_owned(),
        entries[0].0 as usize,
        entries[0].1 / 3,
    )))
}

fn main() -> Result<(), scroll::Error> {
    let _ = std::panic::catch_unwind(|| {
        let (send, recv) = mpsc::channel::<(String, usize, usize)>();
        std::thread::spawn(move || {
            let output = File::create(
                dirs::desktop_dir()
                    .unwrap()
                    .join("sims3_geom_poly_count.csv"),
            )
            .unwrap();
            let mut output = BufWriter::new(output);
            writeln!(&mut output, "Filename, Max Vertices, Max Polygons").unwrap();
            for (filename, verticies, triangles) in recv {
                writeln!(
                    &mut output,
                    "\"{}\", {}, {}",
                    filename, verticies, triangles
                )
                .unwrap();
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
                //.for_each(|e| {
                let res = catch_unwind(|| geom_information(e.path()));
                match res {
                    Err(unwind) => println!(
                        "Caught panic while parsing {}: {:?}",
                        // e.path().file_name().unwrap().to_string_lossy(),
                        e.path().to_string_lossy(),
                        unwind
                    ),
                    Ok(Err(err)) => println!(
                        "Error while parsing {}: {}",
                        // e.path().file_name().unwrap().to_string_lossy(),
                        e.path().to_string_lossy(),
                        err
                    ),
                    Ok(Ok(Some(res))) => drop(send.send(res)),
                    Ok(Ok(None)) => {}
                }
            });
    });

    dont_disappear::any_key_to_continue::default();

    Ok(())
}
