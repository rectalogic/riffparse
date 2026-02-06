#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};
use core::fmt::Debug;
use riffparse::{
    List, Read, Riff, RiffParser, RiffType, Seek, avi,
    binrw::io::{Cursor, Write},
};

// Generate test video:
// ffmpeg -y -f lavfi -i testsrc=size=32x24:rate=20:duration=1:decimals=3 -f lavfi -i sine=frequency=1000:sample_rate=16000 -c:v mjpeg -c:a pcm_s16le -shortest -r 20 -f avi test.avi

const SNAPSHOT: &str = include_str!("test.avi.snapshot");
const AVI: &[u8] = include_bytes!("test.avi");

fn debug<T: Debug, W: Write>(o: T, output: &mut W) {
    write!(output, "{}", format_args!("{o:?}\n")).unwrap();
}

fn process_list<R: Read + Seek + Debug, W: Write>(list: Riff<List, R>, output: &mut W) {
    debug(&list, output);

    let mut stream: Option<avi::AviStreamHeader> = None;
    for chunk in list.iter() {
        let chunk = chunk.unwrap();
        match chunk {
            RiffType::List(riff_list) => {
                process_list(riff_list, output);
            }
            RiffType::Chunk(mut riff_chunk) => {
                debug(&riff_chunk, output);
                match riff_chunk.id() {
                    avi::tag::AVIH => {
                        let avih = riff_chunk.read_data_struct::<avi::AviMainHeader>().unwrap();
                        debug(avih, output);
                    }
                    avi::tag::STRH => {
                        let strh = riff_chunk
                            .read_data_struct::<avi::AviStreamHeader>()
                            .unwrap();
                        debug(&strh, output);
                        stream = Some(strh);
                    }
                    avi::tag::STRF => {
                        if let Some(strh) = stream {
                            match strh.fcc_type {
                                avi::tag::VIDS => {
                                    let vids =
                                        riff_chunk.read_data_struct::<avi::BitmapInfo>().unwrap();
                                    debug(vids, output);
                                }
                                avi::tag::AUDS => {
                                    let auds =
                                        riff_chunk.read_data_struct::<avi::WaveFormat>().unwrap();
                                    debug(auds, output);
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

fn dump_avi<W: Write>(output: &mut W) {
    let parser = RiffParser::new(Cursor::new(AVI));
    process_list(parser.riff().unwrap(), output);
}

#[test]
fn test_avi() {
    let mut output = Vec::new();
    dump_avi(&mut output);
    assert_eq!(SNAPSHOT, String::from_utf8(output).unwrap());
}

#[cfg(feature = "std")]
#[test]
#[ignore]
fn test_avi_snapshot() {
    use std::{fs::File, path::PathBuf};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test.avi.snapshot");
    let mut file = File::create(path).unwrap();
    dump_avi(&mut file);
}
