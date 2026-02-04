use riffparse::{ChunkRead, ChunkType, Error as BinError, RiffParser, avi};
use std::fs::File;

#[test]
fn test_avi() {
    let file =
        File::open("/Users/aw/Projects/rectalogic/experiments/vendor/esp32-tv/player/milk2.avi")
            .unwrap();
    let parser = RiffParser::new(file);
    if let ChunkType::List(riff) = parser.riff().unwrap() {
        dbg!(&riff);
        riff.iter()
            .try_for_each(|chunk| -> Result<(), BinError> {
                let chunk = chunk?;
                match chunk {
                    ChunkType::List(list) => {
                        dbg!(&list);
                        list.iter()
                            .try_for_each(|subchunk| -> Result<(), BinError> {
                                let subchunk = subchunk?;
                                match subchunk {
                                    ChunkType::List(sublist) => {
                                        dbg!(sublist);
                                    }
                                    ChunkType::Chunk(mut subsubchunk) => {
                                        dbg!(&subsubchunk);
                                        match subsubchunk.id() {
                                            avi::tag::AVIH => {
                                                let avih = subsubchunk
                                                    .read_data_struct::<avi::AviStreamHeader>()
                                                    .unwrap();
                                                dbg!(avih);
                                            }
                                            _ => {}
                                        }
                                    }
                                };
                                Ok(())
                            })?;
                    }
                    ChunkType::Chunk(mut chunk) => {
                        dbg!(&chunk);
                    }
                };
                Ok(())
            })
            .unwrap();
    };
}
