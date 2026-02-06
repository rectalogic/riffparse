use binrw::BinRead;

use crate::fourcc::Fourcc;

pub mod tag {
    use super::Fourcc;
    pub const AVI: Fourcc = Fourcc::new(*b"AVI ");
    pub const HDRL: Fourcc = Fourcc::new(*b"hdrl");
    pub const AVIH: Fourcc = Fourcc::new(*b"avih");
    pub const STRL: Fourcc = Fourcc::new(*b"strl");
    pub const STRH: Fourcc = Fourcc::new(*b"strh");
    pub const STRF: Fourcc = Fourcc::new(*b"strf");
    pub const VIDS: Fourcc = Fourcc::new(*b"vids");
    pub const AUDS: Fourcc = Fourcc::new(*b"auds");

    pub const DATA_VIDEO_COMPRESSED: [u8; 2] = *b"dc";
    pub const DATA_VIDEO_UNCOMPRESSED: [u8; 2] = *b"db";
    pub const DATA_PALETTE_CHANGED: [u8; 2] = *b"pc";
    pub const DATA_AUDIO: [u8; 2] = *b"wb";

    pub const fn stream(mut stream_index: u32, datatype: [u8; 2]) -> Fourcc {
        if stream_index > 99 {
            stream_index = 99; // clamp to two digits
        }
        Fourcc::new([
            b'0' + ((stream_index / 10) as u8),
            b'0' + ((stream_index % 10) as u8),
            datatype[0],
            datatype[1],
        ])
    }
}

/// https://learn.microsoft.com/en-us/previous-versions/ms779632(v=vs.85)
#[derive(BinRead, Debug)]
#[br(little)]
pub struct AviMainHeader {
    pub micro_sec_per_frame: u32,
    pub max_bytes_per_sec: u32,
    pub padding_granularity: u32,
    pub flags: u32,
    pub total_frames: u32,
    pub initial_frames: u32,
    pub streams: u32,
    pub suggested_buffer_size: u32,
    pub width: u32,
    pub height: u32,
    pub reserved: [u32; 4],
}

/// https://learn.microsoft.com/en-us/previous-versions/ms779638(v=vs.85)
#[derive(BinRead, Debug)]
#[br(little)]
pub struct AviStreamHeader {
    pub fcc_type: Fourcc,
    pub fcc_handler: Fourcc,
    pub flags: u32,
    pub priority: u16,
    pub language: u16,
    pub initial_frames: u32,
    pub scale: u32,
    pub rate: u32,
    pub start: u32,
    pub length: u32,
    pub suggested_buffer_size: u32,
    pub quality: u32,
    pub sample_size: u32,
    pub frame: Frame,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Frame {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

/// https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2012/z5731wbz(v=vs.110)
#[derive(BinRead, Debug)]
#[br(little)]
pub struct BitmapInfo {
    pub header: BitmapInfoHeader,
    pub colors: RgbQuad,
}

/// https://learn.microsoft.com/en-us/previous-versions/dd183376(v=vs.85)
#[derive(BinRead, Debug)]
#[br(little)]
pub struct BitmapInfoHeader {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bit_count: u16,
    pub compression: u32,
    pub size_image: u32,
    pub x_pels_per_meter: i32,
    pub y_pels_per_meter: i32,
    pub clr_used: u32,
    pub clr_important: u32,
}

/// https://learn.microsoft.com/en-us/previous-versions/ms911013(v=msdn.10)
#[derive(BinRead, Debug)]
#[br(little)]
pub struct RgbQuad {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub reserved: u8,
}

/// https://learn.microsoft.com/en-us/previous-versions/ms788112(v=vs.85)
#[derive(BinRead, Debug)]
#[br(little)]
pub enum WaveFormat {
    #[br(magic = 0x0001u16)]
    Pcm(WaveFormatEx),
    #[br(magic = 0xfffeu16)]
    Extensible(WaveFormatExtensible),
    #[br(magic = 0x0050u16)]
    Mpeg1(Mpeg1WaveFormat),
    #[br(magic = 0x0055u16)]
    Mp3(Mp3WaveFormat),
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct WaveFormatEx {
    pub channels: u16,
    pub samples_per_sec: u32,
    pub av_bytes_per_sec: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub size: u16,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct WaveFormatExtensible {
    pub format: WaveFormatEx,
    // union {
    //   WORD  wValidBitsPerSample;
    //   WORD  wSamplesPerBlock;
    //   WORD  wReserved;
    // } Samples;
    pub samples: u16,
    pub channel_mask: u32,
    pub sub_format: Guid,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Mpeg1WaveFormat {
    pub format: WaveFormatEx,
    pub head_layer: u16,
    pub head_bitrate: u32,
    pub head_mode: u16,
    pub head_mode_ext: u16,
    pub head_emphasis: u16,
    pub head_flags: u16,
    pub pts_low: u32,
    pub pts_high: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Mp3WaveFormat {
    pub format: WaveFormatEx,
    pub id: u16,
    pub flags: u32,
    pub block_size: u16,
    pub frames_per_block: u16,
    pub codec_delay: u16,
}

#[derive(Debug)]
pub enum StreamInfo {
    Audio {
        stream_id: Fourcc,
        stream_header: AviStreamHeader,
        wave_format: WaveFormat,
    },
    Video {
        stream_id: Fourcc,
        stream_header: AviStreamHeader,
        bitmap_info: BitmapInfo,
    },
}
