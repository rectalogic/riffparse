use core::fmt::Debug;
use riffparse::{List, Read, Riff, RiffParser, RiffType, Seek, avi};
use std::fs::File;

fn process_list<R: Read + Seek + Debug>(list: Riff<List, R>) {
    dbg!(&list);

    let mut stream: Option<avi::AviStreamHeader> = None;
    for chunk in list.iter() {
        let chunk = chunk.unwrap();
        match chunk {
            RiffType::List(riff_list) => {
                process_list(riff_list);
            }
            RiffType::Chunk(mut riff_chunk) => {
                dbg!(&riff_chunk);
                match riff_chunk.id() {
                    avi::tag::AVIH => {
                        let avih = riff_chunk.read_data_struct::<avi::AviMainHeader>().unwrap();
                        dbg!(avih);
                    }
                    avi::tag::STRH => {
                        let strh = riff_chunk
                            .read_data_struct::<avi::AviStreamHeader>()
                            .unwrap();
                        dbg!(&strh);
                        stream = Some(strh);
                    }
                    avi::tag::STRF => {
                        if let Some(strh) = stream {
                            match strh.fcc_type {
                                avi::tag::VIDS => {
                                    let vids =
                                        riff_chunk.read_data_struct::<avi::BitmapInfo>().unwrap();
                                    dbg!(vids);
                                }
                                avi::tag::AUDS => {
                                    let auds =
                                        riff_chunk.read_data_struct::<avi::WaveFormat>().unwrap();
                                    dbg!(auds);
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

#[test]
fn test_avi() {
    let file =
        File::open("/Users/aw/Projects/rectalogic/experiments/vendor/esp32-tv/player/milk2.avi")
            .unwrap();
    let parser = RiffParser::new(file);
    process_list(parser.riff().unwrap());
}
