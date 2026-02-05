use core::fmt::Debug;
use riffparse::{ChunkType, Read, RiffParser, Seek, avi};
use std::fs::File;

fn process_chunk<R: Read + Seek + Debug>(chunk: ChunkType<R>) {
    dbg!(&chunk);
    match chunk {
        ChunkType::List(list) => {
            for listchunk in list.iter() {
                let listchunk = listchunk.unwrap();
                process_chunk(listchunk);
            }
        }
        ChunkType::Chunk(mut chunk) => {
            match chunk.id() {
                avi::tag::AVIH => {
                    let avih = chunk.read_data_struct::<avi::AviMainHeader>().unwrap();
                    dbg!(avih);
                }
                avi::tag::STRH => {
                    let strh = chunk.read_data_struct::<avi::AviStreamHeader>().unwrap();
                    dbg!(strh);
                }
                _ => {} //XXX handle strf struct based on preceding strh type
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
    process_chunk(parser.riff().unwrap());
}
