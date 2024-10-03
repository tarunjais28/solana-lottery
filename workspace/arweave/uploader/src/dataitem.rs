use {
    crate::{
        deephash::{deep_hash, DeepHashChunk, DATAITEM_AS_BUFFER, ONE_AS_BUFFER},
        tag::{AvroEncode, Tag},
    },
    anyhow::Result,
    ring::rand::SecureRandom,
    solana_sdk::signature::{Keypair, Signer},
};

pub struct DataItem(Vec<u8>);

impl DataItem {
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    pub fn create(data: Vec<u8>, tags: Vec<Tag>, keypair: &Keypair) -> Result<Self> {
        let encoded_tags = if !tags.is_empty() { tags.encode()? } else { vec![] };

        let length = 2  // signature type length
            + 64    // Solana signature length
            + 32    // Solana pubkey length
            + 34    // 1 + 1 + 32 (anchor)
            + 16    // Tags length
            + encoded_tags.len()
            + data.len();

        let mut bytes: Vec<u8> = Vec::with_capacity(length);
        let mut randoms: [u8; 32] = [0; 32];
        let sr = ring::rand::SystemRandom::new();
        sr.fill(&mut randoms).unwrap();

        let anchor = randoms.to_vec();

        let sig_type = 2u16;

        let sig_type_bytes = sig_type.to_string().as_bytes().to_vec();

        let message = deep_hash(DeepHashChunk::List(vec![
            DeepHashChunk::Blob(DATAITEM_AS_BUFFER.to_vec()),
            DeepHashChunk::Blob(ONE_AS_BUFFER.to_vec()),
            DeepHashChunk::Blob(sig_type_bytes),
            DeepHashChunk::Blob(keypair.pubkey().as_ref().to_vec()),
            DeepHashChunk::Blob(vec![]),
            DeepHashChunk::Blob(anchor.clone()),
            DeepHashChunk::Blob(encoded_tags.clone()),
            DeepHashChunk::Blob(data.clone()),
        ]));

        let sig = keypair.sign_message(&message);

        let sig_type = 2u16.to_le_bytes();
        bytes.extend(&sig_type);

        bytes.extend(sig.as_ref());

        bytes.extend(keypair.pubkey().as_ref());

        let target = &[0u8];
        bytes.extend(target);

        bytes.extend(&[1u8]);
        bytes.extend(anchor);

        let number_of_tags = (tags.len() as u64).to_le_bytes();
        let number_of_tags_bytes = (encoded_tags.len() as u64).to_le_bytes();
        bytes.extend(&number_of_tags);
        bytes.extend(&number_of_tags_bytes);
        if !number_of_tags_bytes.is_empty() {
            bytes.extend(&encoded_tags);
        }

        bytes.extend(data);

        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, solana_sdk::signature::Keypair};

    #[test]
    fn test_data_item_create() {
        let keypair = Keypair::new();
        let data = Vec::from("hello");
        let tags = vec![Tag {
            name: "name".to_string(),
            value: "value".to_string(),
        }];
        let data_item = DataItem::create(data.clone(), tags.clone(), &keypair).unwrap();
        let raw_data_item = data_item.into_inner();
        let encoded_tags = tags.encode().unwrap();
        let offset = 2 + 64 + 32 + 34 + 16 + encoded_tags.len();
        let expected_length = offset + data.len();
        assert_eq!(raw_data_item.len(), expected_length);
        assert_eq!(&raw_data_item[offset..], &data);
    }
}
