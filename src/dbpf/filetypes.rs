// TODO: What should I do with these? I'm just making this public for now.
pub mod nmap;
pub mod casp;

//use num_traits::ToPrimitive;
use scroll::ctx::TryFromCtx;
use std::convert::TryInto;
use scroll::Pread;

// TODO: bring in ALL resource types.
//       Perhaps name them a bit better/more consistently as well?
#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, FromPrimitive, ToPrimitive)]
pub enum ResourceType {
    Unknown = 0,
    BONE = 0x00AE6C67,
    IMG = 0x00B2D882, // ddt
    SPT = 0x00B552EA,
    GEOM = 0x015A1849,
    NMAP = 0x0166038C,
    MODL = 0x01661233,
    AUDSNR = 0x01A527DB,
    AUDSNS = 0x01EEF63A,
    VBUF1 = 0x01D0E6FB,
    IBUF1 = 0x01D0E70F,
    // "World Description Name Hash" 0x022B756C
    VRTF = 0x01D0E723,
    MATD = 0x01D0E75D,
    SKIN = 0x01D0E76B,
    MLOD = 0x01D10F34,
    MTST = 0x02019972,
    SPT2 = 0x021D7E8C,
    VBUF2 = 0x0229684B,
    IBUF2 = 0x0229684F,
    CSS = 0x025C90A6,
    LAYO = 0x025C95B6,
    SIMO = 0x025ED6F4,
    VOCE = 0x029E333B,
    MIXR = 0x02C9EFF2,
    JAZZ = 0x02D5DF13,
    OBJK = 0x02DC343F,
    TKMK = 0x033260E3,
    XMLResource = 0x0333406C,
    TXTC = 0x033A1435,
    // unknown 0x033B2B66
    TXTF = 0x0341ACC9,
    CASP = 0x034AEECB,
    SkinTone = 0x0354796A,
    HairTone = 0x03555BA8,
    BoneDelta = 0x0355E0A6,
    FACE = 0x0358B08A,
    ITUN = 0x03B33DDF,
    LITE = 0x03B4C61D,
    CCHE = 0x03D843C2,
    DETL = 0x03D86EA4,
    // unknown 0x03E80CDC
    CFEN = 0x0418FE2A,
    // unknown 0x044735DD
    COMP = 0x044AE110,
    LotLoc = 0x046A7235,
    // "UNKN" 0x048A166D
    LotID = 0x0498DA7E,
    CSTR = 0x049CA4CD,
    StairLocation = 0x04A09283,
    WorldDetail = 0x04A4D951,
    CPRX = 0x04AC5D93,
    CTTL = 0x04B30669,
    CRAL = 0x04C58103,
    CMRU = 0x04D82D90,
    CTPT = 0x04ED4BB2,
    // "UNKN" "Lot terrain texture list" 0x04EE6ABB
    CFIR = 0x04F3CC01,
    SBNO = 0x04F51033,
    // "UNKN" "Fireplace/chimney groups" 0x04F66BCC
    SIME = 0x04F88964,
    CBLN = 0x051DF2DD,
    // 0x05512255
    // 0x0553EAD4
    // 0x0563919E
    // 0x0580A2B4
    // 0x0580A2B5 -- appears in object packages
    // 0x0580A2B6
    SimSNAPUnk = 0x0580A2CD,   // png
    SimSNAPSmall = 0x0580A2CE, // png
    SimSNAPLarge = 0x0580A2CF, // png
    // 0x0580A2B4, 0x0580A2B5, 0x0580A2B6, // THUM
    // 0x0589DC44, 0x0589DC45, 0x0589DC46, // AllThumbnails.package
    UPST = 0x0591B1AF,
    // 0x05B17698, 0x05B17699, 0x05B1769A, // AllThumbnails.package
    // 0x05B1B524, 0x05B1B525, 0x05B1B526, // AllThumbnails.package

    // ...
    TWNI = 0x0668F635, // png
    // ...
    OBJIconSmall = 0x2E75C764,  // png
    OBJIconMedium = 0x2E75C765, // png
    OBJIconLarge = 0x2E75C766,  // png
    OBJIconXLarge = 0x2E75C767, // png
    UIImageTGA = 0x2F7D0002,    // png?
    UIImagePNG = 0x2F7D0004,    // png
    // ...
    OBJD = 0x319E4F1D,
    // ...
    TravelSNAP = 0x54372472, // png
    // ...
    FamilySNAPSmall = 0x6B6D837D,  // png
    FamilySNAPMedium = 0x6B6D837E, // png
    FamilySNAPLarge = 0x6B6D837F,  // png
    // ...
    XMLManifest = 0x73E93EEB,
    // ...
    PTRN = 0xD4D9FBE5,
    // ...
    LotIconSmall = 0xD84E7FC5,  // png
    LotIconMedium = 0xD84E7FC6, // png
    LotIconLarge = 0xD84E7FC7,  // png
    // etc...
    ColorThumb = 0xFCEAB65B, // png
}

impl Default for ResourceType {
    fn default() -> ResourceType {
        ResourceType::Unknown
    }
}

lazy_static! {
    static ref PNG_RESOURCES: [u32; 41] = [
        ResourceType::SimSNAPUnk as u32,
        ResourceType::SimSNAPSmall as u32,
        ResourceType::SimSNAPLarge as u32,
        0x0580A2B4,
        0x0580A2B5,
        0x0580A2B6, // THUM
        0x0589DC44,
        0x0589DC45,
        0x0589DC46, // AllThumbnails.package
        0x05B17698,
        0x05B17699,
        0x05B1769A, // AllThumbnails.package
        0x05B1B524,
        0x05B1B525,
        0x05B1B526, // AllThumbnails.package
        ResourceType::TWNI as u32,
        0x2653E3C8,
        0x2653E3C9,
        0x2653E3CA, // AllThumbnails.package
        0x2D4284F0,
        0x2D4284F1,
        0x2D4284F2, // AllThumbnails.package
        ResourceType::OBJIconSmall as u32,
        ResourceType::OBJIconMedium as u32,
        ResourceType::OBJIconLarge as u32,
        ResourceType::OBJIconXLarge as u32,
        // ResourceType::UIImageTGA as u32,
        ResourceType::UIImagePNG as u32,
        ResourceType::TravelSNAP as u32,
        0x5DE9DBA0,
        0x5DE9DBA1,
        0x5DE9DBA2, // AllThumbnails.package
        0x626F60CC,
        0x626F60CD,
        0x626F60CE, // CasThumbnails.package
        ResourceType::FamilySNAPSmall as u32,
        ResourceType::FamilySNAPMedium as u32,
        ResourceType::FamilySNAPLarge as u32,
        ResourceType::LotIconSmall as u32,
        ResourceType::LotIconMedium as u32,
        ResourceType::LotIconLarge as u32,
        ResourceType::ColorThumb as u32,
    ];
}

pub fn resource_is_png(resource: u32) -> bool {
    PNG_RESOURCES.iter().any(|&x| x == resource)
}

struct TaggedUTF16<LengthT, Endian>(String, std::marker::PhantomData<(LengthT, Endian)>);

impl<'a, LengthT, Endian> TryFromCtx<'a> for TaggedUTF16<LengthT, Endian>
    where LengthT: TryInto<usize, Error = std::num::TryFromIntError>
           + TryFromCtx<'a, Error = scroll::Error, Size = usize>
           + 'a,
          Endian: byteorder::ByteOrder + 'a
{
    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, Self::Size), Self::Error> {
        let offset = &mut 0usize;
        let length: usize = src.gread::<LengthT>(offset)?.try_into().unwrap();

        let mut data = vec![0u16; length/2];
        Endian::read_u16_into(&src[*offset..*offset+length], &mut data);

        Ok((String::from_utf16_lossy(&data).into(), *offset+length))
    }
}

impl<LengthT, Endian> From<TaggedUTF16<LengthT,Endian>> for String {
    fn from(src: TaggedUTF16<LengthT,Endian>) -> Self {
        src.0
    }
}

impl<LengthT, Endian> From<String> for TaggedUTF16<LengthT, Endian> {
    fn from(src: String) -> Self {
        TaggedUTF16(src, std::marker::PhantomData)
    }
}

struct LengthData<Data, Length>(Data, std::marker::PhantomData<Length>);

impl <'a, Data, Length> TryFromCtx<'a> for LengthData<Data, Length>
    where Data: TryFromCtx<'a, Size = usize> + 'a,
          Length: TryInto<usize, Error = std::num::TryFromIntError>
                + TryFromCtx<'a, Size = usize> + 'a,
          scroll::Error: std::convert::From<<Data as scroll::ctx::TryFromCtx<'a>>::Error>
                       + std::convert::From<<Length as scroll::ctx::TryFromCtx<'a>>::Error>,
          <Data as TryFromCtx<'a>>::Error: std::convert::From<scroll::Error>,
          <Length as TryFromCtx<'a>>::Error: std::convert::From<scroll::Error> {

    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, Self::Size), Self::Error> {
        let read = &mut 0usize;
        let length: usize = src.gread::<Length>(read)?.try_into().unwrap();

        Ok((LengthData((&src[*read..*read+length]).pread(0)?, std::marker::PhantomData), *read))
    }
}

// Reads an entire buffer as a UTF-16 string. maybe use StrCtx from scroll in the future?
struct UTF16<Endian>(String, std::marker::PhantomData<Endian>);

impl<'a, Endian> TryFromCtx<'a> for UTF16<Endian> where Endian: byteorder::ByteOrder + 'a {
    type Error = scroll::Error;
    type Size = usize;
    fn try_from_ctx(src: &'a [u8], _ctx: ()) -> Result<(Self, Self::Size), Self::Error> {
        if src.len() % 2 != 0 {
            return Err(scroll::Error::Custom("Length of utf-16 string is not a multiple of 2!".to_owned()));
        }

        let mut data = vec![0u16; src.len()/2];
        Endian::read_u16_into(src, &mut data);
        Ok((UTF16(String::from_utf16_lossy(&data).into(), std::marker::PhantomData), src.len()))
    }
}

impl<Length, Endian> From<LengthData<UTF16<Endian>, Length>> for String {
    fn from(src: LengthData<UTF16<Endian>, Length>) -> Self {
        (src.0).0
    }
}
