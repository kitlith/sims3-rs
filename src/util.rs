use binrw::{binrw, BinWrite, BinResult, io};
use std::collections::BTreeMap;

#[binrw]
pub(crate) struct LengthString {
    #[br(temp)]
    #[bw(try_calc = inner.len().try_into())]
    len: u32,
    #[br(count = len)]
    pub inner: Vec<u8>
}

pub(crate) fn write_btreemap<'args, K, V, Args, W: io::Write + io::Seek>(
    collection: &BTreeMap<K, V>,
    writer: &mut W,
    endian: binrw::Endian,
    args: Args,
) -> BinResult<()>
where
    // NOTE: this for<'a> appears to be the barrier to making this generic on collection type.
    for<'a> (&'a K, &'a V): BinWrite<Args<'args> = Args>,
    Args: Clone,
{
    for item in collection.into_iter() {
        BinWrite::write_options(&item, writer, endian, args.clone())?;
    }
    Ok(())
}