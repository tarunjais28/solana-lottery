use openssl::sha::Sha384;

const LIST_AS_BUFFER: &[u8] = "list".as_bytes();
const BLOB_AS_BUFFER: &[u8] = "blob".as_bytes();
pub const DATAITEM_AS_BUFFER: &[u8] = "dataitem".as_bytes();
pub const ONE_AS_BUFFER: &[u8] = "1".as_bytes();

pub enum DeepHashChunk {
    Blob(Vec<u8>),
    List(Vec<DeepHashChunk>),
}

pub fn deep_hash(chunk: DeepHashChunk) -> Vec<u8> {
    match chunk {
        DeepHashChunk::Blob(blob) => {
            let tag = [BLOB_AS_BUFFER, blob.len().to_string().as_bytes()].concat();
            sha384hash([sha384hash(tag), sha384hash(blob)].concat())
        }
        DeepHashChunk::List(list) => {
            let len = list.len() as f64;
            let tag = [LIST_AS_BUFFER, len.to_string().as_bytes()].concat();
            list.into_iter().fold(sha384hash(tag), |acc, chunk| {
                sha384hash([acc, deep_hash(chunk)].concat())
            })
        }
    }
}

fn sha384hash(bytes: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha384::new();
    hasher.update(&bytes);
    hasher.finish().to_vec()
}
