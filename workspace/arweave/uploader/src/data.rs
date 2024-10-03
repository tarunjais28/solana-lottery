use mime::Mime;

pub struct DataWithMediaType<T: AsRef<[u8]> + Send> {
    pub data: T,
    pub media_type: Mime,
}

impl<T: AsRef<[u8]> + Send> DataWithMediaType<T> {
    pub fn bytes(&self) -> Vec<u8> {
        self.data.as_ref().to_vec()
    }
}
