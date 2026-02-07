#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};
use core::fmt::Debug;
#[cfg(feature = "embedded-io")]
use riffparse::EmbeddedAdapter;
use riffparse::{
    List, Read, Riff, RiffParser, RiffType, Seek, avi,
    binrw::io::{Cursor, Write},
};

// Generate test video:
// ffmpeg -y -f lavfi -i testsrc=size=32x24:rate=20:duration=1:decimals=3 -f lavfi -i sine=frequency=1000:sample_rate=16000 -c:v mjpeg -c:a pcm_s16le -shortest -r 20 -f avi test.avi
// ffmpeg -y -f lavfi -i testsrc=size=32x24:rate=20:duration=1:decimals=3 -f lavfi -i sine=frequency=1000:sample_rate=16000 -vn -c:a mp3 -t 2 -r 20 -f avi mp3.avi

const TEST_AVI_SNAPSHOT: &str = include_str!("test.avi.snapshot");
const TEST_AVI: &[u8] = include_bytes!("test.avi");
const MP3_AVI_SNAPSHOT: &str = include_str!("mp3.avi.snapshot");
const MP3_AVI: &[u8] = include_bytes!("mp3.avi");

fn debug<T: Debug, W: Write>(o: T, output: &mut W, indent: u8) {
    writeln!(output, "{:indent$}{o:?}", "", indent = indent as usize).unwrap();
}

fn process_list<R: Read + Seek + Debug, W: Write>(
    list: Riff<List, R>,
    output: &mut W,
    mut indent: u8,
) {
    debug(&list, output, indent);
    indent += 4;

    let mut stream: Option<avi::AviStreamHeader> = None;
    for chunk in list.iter() {
        let chunk = chunk.unwrap();
        match chunk {
            RiffType::List(riff_list) => {
                process_list(riff_list, output, indent);
            }
            RiffType::Chunk(mut riff_chunk) => {
                debug(&riff_chunk, output, indent);
                match riff_chunk.id() {
                    avi::tag::AVIH => {
                        let avih = riff_chunk.read_data_struct::<avi::AviMainHeader>().unwrap();
                        debug(avih, output, indent);
                    }
                    avi::tag::STRH => {
                        let strh = riff_chunk
                            .read_data_struct::<avi::AviStreamHeader>()
                            .unwrap();
                        debug(&strh, output, indent);
                        stream = Some(strh);
                    }
                    avi::tag::STRF => {
                        if let Some(strh) = stream {
                            match strh.fcc_type {
                                avi::tag::VIDS => {
                                    let vids =
                                        riff_chunk.read_data_struct::<avi::BitmapInfo>().unwrap();
                                    debug(vids, output, indent);
                                }
                                avi::tag::AUDS => {
                                    let auds =
                                        riff_chunk.read_data_struct::<avi::WaveFormat>().unwrap();
                                    debug(auds, output, indent);
                                }
                                _ => {}
                            };
                            stream = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn dump_avi<R: Read + Seek + Debug, W: Write>(avi: R, output: &mut W) {
    let parser = RiffParser::new(avi);
    process_list(parser.riff().unwrap(), output, 0);
}

#[test]
fn test_test_avi() {
    let mut output = Vec::new();
    dump_avi(Cursor::new(TEST_AVI), &mut output);
    assert_eq!(TEST_AVI_SNAPSHOT, String::from_utf8(output).unwrap());
}

#[test]
fn test_mp3_avi() {
    let mut output = Vec::new();
    dump_avi(Cursor::new(MP3_AVI), &mut output);
    assert_eq!(MP3_AVI_SNAPSHOT, String::from_utf8(output).unwrap());
}

#[test]
fn test_avi_video() {
    let mut parser = RiffParser::new(Cursor::new(TEST_AVI));
    let avi_parser = avi::AviParser::new(&mut parser).unwrap();

    let avi::StreamInfo::Video {
        stream_id: video_id,
        ..
    } = avi_parser.stream_info[0]
    else {
        panic!("stream 0 not video");
    };
    let avi::StreamInfo::Audio {
        stream_id: audio_id,
        ..
    } = avi_parser.stream_info[1]
    else {
        panic!("stream 1 not audio");
    };
    assert_eq!(avi_parser.iter(video_id).count(), 20);
    assert_eq!(avi_parser.iter(audio_id).count(), 15);
}

#[cfg(feature = "embedded-io")]
pub mod embedded {
    use core::convert::Infallible;

    use super::*;

    #[derive(Debug)]
    pub struct Reader {
        data: Vec<u8>,
        pos: usize,
    }

    impl Reader {
        pub fn new(data: Vec<u8>) -> Self {
            Self { data, pos: 0 }
        }
    }

    impl embedded_io::ErrorType for Reader {
        type Error = Infallible;
    }
    impl embedded_io::Read for Reader {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let available = self.data.len().saturating_sub(self.pos);
            let to_read = available.min(buf.len());
            if to_read == 0 {
                return Ok(0);
            }
            buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
            self.pos += to_read;
            Ok(to_read)
        }
    }
    impl embedded_io::Seek for Reader {
        fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
            let data_len = self.data.len() as i128;
            let cur = self.pos as i128;
            let next = match pos {
                embedded_io::SeekFrom::Start(n) => n as i128,
                embedded_io::SeekFrom::End(n) => data_len.saturating_add(n as i128),
                embedded_io::SeekFrom::Current(n) => cur.saturating_add(n as i128),
            };
            let clamped = next.clamp(0, usize::MAX as i128);
            self.pos = clamped as usize;
            Ok(self.pos as u64)
        }
    }
}

#[cfg(all(feature = "embedded-io", not(feature = "std")))]
#[test]
fn test_avi_embeddedio() {
    let mut output = Vec::new();
    let avi = embedded::Reader::new(TEST_AVI.to_vec());
    dump_avi(EmbeddedAdapter(avi), &mut output);
    assert_eq!(TEST_AVI_SNAPSHOT, String::from_utf8(output).unwrap());
}

#[cfg(feature = "std")]
fn write_snapshot(avi: &[u8], snapshot_file: &str) {
    use std::{fs::File, path::PathBuf};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(snapshot_file);
    let mut file = File::create(path).unwrap();
    dump_avi(Cursor::new(avi), &mut file);
}

#[cfg(feature = "std")]
#[test]
#[ignore]
fn test_test_avi_snapshot() {
    write_snapshot(TEST_AVI, "tests/test.avi.snapshot");
}

#[cfg(feature = "std")]
#[test]
#[ignore]
fn test_mp3_avi_snapshot() {
    write_snapshot(MP3_AVI, "tests/mp3.avi.snapshot");
}
