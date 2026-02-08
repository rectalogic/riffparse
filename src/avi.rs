use crate::{List, Riff, RiffParser, RiffType, fourcc::Fourcc, riff::Header};
use alloc::{format, vec::Vec};
use binrw::{
    BinRead, Error,
    io::{self, Read, Seek},
};
use core::{convert::TryFrom, fmt::Debug};

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
    pub const MOVI: Fourcc = Fourcc::new(*b"movi");

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
/// https://learn.microsoft.com/en-us/previous-versions/dd183376(v=vs.85)
// Ignore RGBQUAD bmiColors[1] array at end
#[derive(BinRead, Debug)]
#[br(little)]
pub struct BitmapInfo {
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
    #[br(try)]
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
    Audio(AudioStream),
    Video(VideoStream),
}

pub trait Stream {
    fn stream_id(&self) -> Fourcc;
    fn stream_header(&self) -> &AviStreamHeader;
}

#[derive(Debug)]
pub struct AudioStream {
    pub stream_id: Fourcc,
    pub stream_header: AviStreamHeader,
    pub wave_format: WaveFormat,
}

impl<'a> TryFrom<&'a StreamInfo> for &'a AudioStream {
    type Error = ();

    fn try_from(value: &'a StreamInfo) -> Result<Self, Self::Error> {
        match value {
            StreamInfo::Audio(a) => Ok(a),
            _ => Err(()),
        }
    }
}

impl Stream for AudioStream {
    fn stream_id(&self) -> Fourcc {
        self.stream_id
    }

    fn stream_header(&self) -> &AviStreamHeader {
        &self.stream_header
    }
}

#[derive(Debug)]
pub struct VideoStream {
    pub stream_id: Fourcc,
    pub stream_header: AviStreamHeader,
    pub bitmap_info: BitmapInfo,
}

impl<'a> TryFrom<&'a StreamInfo> for &'a VideoStream {
    type Error = ();

    fn try_from(value: &'a StreamInfo) -> Result<Self, Self::Error> {
        match value {
            StreamInfo::Video(v) => Ok(v),
            _ => Err(()),
        }
    }
}

impl Stream for VideoStream {
    fn stream_id(&self) -> Fourcc {
        self.stream_id
    }

    fn stream_header(&self) -> &AviStreamHeader {
        &self.stream_header
    }
}

pub struct AviParser<R> {
    parser: RiffParser<R>,
    pub avi_header: AviMainHeader,
    pub stream_info: Vec<StreamInfo>,
    pub movi: Riff<List>,
}

impl<R: Read + Seek> AviParser<R> {
    pub fn new(parser: RiffParser<R>) -> Result<Self, Error> {
        let riff = parser.riff()?;
        Self::validate_tag(&riff, tag::AVI)?;

        let mut avi_iter = parser.chunks(riff);
        let RiffType::List(hdrl) = avi_iter.next().ok_or_else(Self::eof_error)?? else {
            return Err(Self::missing_error(avi_iter.position(), tag::HDRL));
        };
        Self::validate_tag(&hdrl, tag::HDRL)?;

        let mut hdrl_iter = parser.chunks(hdrl);
        let RiffType::Chunk(avih) = hdrl_iter.next().ok_or_else(Self::eof_error)?? else {
            return Err(Self::missing_error(hdrl_iter.position(), tag::AVIH));
        };
        Self::validate_tag(&avih, tag::AVIH)?;

        let main_header = parser.read_data_struct::<AviMainHeader>(avih)?;
        let mut stream_info = Vec::with_capacity(main_header.streams as usize);

        for stream_index in 0..main_header.streams {
            let RiffType::List(strl) = hdrl_iter.next().ok_or_else(Self::eof_error)?? else {
                return Err(Self::missing_error(hdrl_iter.position(), tag::STRL));
            };
            Self::validate_tag(&strl, tag::STRL)?;

            let mut strl_iter = parser.chunks(strl);
            let RiffType::Chunk(strh) = strl_iter.next().ok_or_else(Self::eof_error)?? else {
                return Err(Self::missing_error(strl_iter.position(), tag::STRH));
            };
            Self::validate_tag(&strh, tag::STRH)?;
            let stream_header = parser.read_data_struct::<AviStreamHeader>(strh)?;
            let RiffType::Chunk(strf) = strl_iter.next().ok_or_else(Self::eof_error)?? else {
                return Err(Self::missing_error(strl_iter.position(), tag::STRF));
            };
            Self::validate_tag(&strf, tag::STRF)?;

            match stream_header.fcc_type {
                tag::VIDS => {
                    let bitmap_info = parser.read_data_struct::<BitmapInfo>(strf)?;
                    stream_info.push(StreamInfo::Video(VideoStream {
                        stream_id: tag::stream(
                            stream_index,
                            //XXX how do we pick compressed/uncompressed
                            tag::DATA_VIDEO_COMPRESSED,
                        ),
                        stream_header,
                        bitmap_info,
                    }));
                }
                tag::AUDS => {
                    let wave_format = parser.read_data_struct::<WaveFormat>(strf)?;
                    stream_info.push(StreamInfo::Audio(AudioStream {
                        stream_id: tag::stream(stream_index, tag::DATA_AUDIO),
                        stream_header,
                        wave_format,
                    }));
                }
                _ => {}
            }
        }

        let movi = avi_iter
            .find_map(|result| match result {
                Ok(RiffType::List(movi)) if movi.id() == tag::MOVI => Some(Ok(movi)),
                Err(e) => Some(Err(e)),
                _ => None,
            })
            .ok_or_else(Self::eof_error)??;

        Ok(Self {
            parser,
            avi_header: main_header,
            stream_info,
            movi,
        })
    }

    pub fn find_best_stream<S>(&self) -> Option<&S>
    where
        for<'a> &'a S: TryFrom<&'a StreamInfo, Error = ()>,
        S: Stream,
    {
        self.stream_info
            .iter()
            .filter_map(|stream| <&S>::try_from(stream).ok())
            .max_by_key(|&stream| stream.stream_header().priority)
    }

    pub fn stream_chunks(
        &self,
        stream_id: Fourcc,
        movi: Riff<List>,
    ) -> impl Iterator<Item = Result<RiffType, Error>> + '_ {
        self.parser.chunks(movi).filter(move |result| {
            if let Ok(RiffType::Chunk(chunk)) = result
                && chunk.id() == stream_id
            {
                true
            } else {
                false
            }
        })
    }

    pub fn movi_chunks(
        &self,
        stream_id: Fourcc,
    ) -> impl Iterator<Item = Result<RiffType, Error>> + '_ {
        self.stream_chunks(stream_id, self.movi)
    }

    fn eof_error() -> Error {
        Error::Io(io::Error::from(io::ErrorKind::UnexpectedEof))
    }

    fn missing_error(position: u64, tag: Fourcc) -> Error {
        Error::AssertFail {
            pos: position,
            message: format!("missing {}", tag),
        }
    }

    fn validate_tag<H: Header>(riff: &Riff<H>, tag: Fourcc) -> Result<(), Error> {
        if riff.id() != tag {
            Err(Self::missing_error(riff.position(), tag))
        } else {
            Ok(())
        }
    }
}

impl<R: Read + Seek> Debug for AviParser<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AviParser")
            .field("stream_info", &self.stream_info)
            .finish()
    }
}
