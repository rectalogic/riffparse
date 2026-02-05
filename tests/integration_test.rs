use core::fmt::Debug;
use riffparse::{ChunkType, List, Read, RiffItem, RiffParser, Seek, avi};
use std::fs::File;

fn process_list<R: Read + Seek + Debug>(list: RiffItem<List, R>) {
    dbg!(&list);

    let mut stream: Option<avi::AviStreamHeader> = None;
    for chunk in list.iter() {
        let chunk = chunk.unwrap();
        match chunk {
            ChunkType::List(list_item) => {
                process_list(list_item);
            }
            ChunkType::Chunk(mut chunk_item) => {
                dbg!(&chunk_item);
                match chunk_item.id() {
                    avi::tag::AVIH => {
                        let avih = chunk_item.read_data_struct::<avi::AviMainHeader>().unwrap();
                        dbg!(avih);
                    }
                    avi::tag::STRH => {
                        let strh = chunk_item
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
                                        chunk_item.read_data_struct::<avi::BitmapInfo>().unwrap();
                                    dbg!(vids);
                                }
                                avi::tag::AUDS => {
                                    let auds =
                                        chunk_item.read_data_struct::<avi::WaveFormat>().unwrap();
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
