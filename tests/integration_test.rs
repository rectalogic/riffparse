use riffparse::{ChunkRead, ChunkType, Error as BinError, RiffParser};
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
                                        let _data = subsubchunk.read_data_vec().unwrap();
                                        dbg!(subsubchunk);
                                    }
                                };
                                Ok(())
                            })?;
                    }
                    ChunkType::Chunk(chunk) => {
                        dbg!(chunk);
                    }
                };
                Ok(())
            })
            .unwrap();
    };
}
