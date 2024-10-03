use log::{error, info};
use rand::Rng;
use tokio::sync::mpsc::{self, Receiver};

pub const DEFAULT_SIZE: usize = 5000;

/// StartRng will start a background tokio task that keeps a buffered channel filled so that generating large amounts
/// of random numbers takes less time than generating numbers one by one. To make things a little more efficient
/// we use a buffer with half the size of the channel, this means that we don't need to wait for the buffer to be empty
/// in order to start replenishing (this is what would happen if we used a buffer with the same size as the channel)
///
/// TODO: I think it would be interesting to turn this into a struct and impl Rng for it, this way we don't need to
/// change the existing interfaces.
pub async fn start_rng<R>(mut rng: R, size: usize) -> Receiver<u8>
where
    R: Rng + Sync + Send + 'static,
{
    let (tx, rx) = mpsc::channel(size);

    tokio::spawn(async move {
        let size = if size <= 1 { size } else { size / 2 };
        let mut bs = vec![0u8; size];

        'outer: loop {
            rng.fill_bytes(&mut bs);

            for i in bs.iter() {
                match tx.send(*i).await {
                    Ok(_) => continue,
                    Err(err) => {
                        error!("send error: {}", err);
                        break 'outer;
                    }
                }
            }
        }

        info!("rng generator has stopped, sender is closed");
    });

    rx
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use rand::SeedableRng;
    use tokio::sync::mpsc::Receiver;

    use crate::rng::buffered::start_rng;

    const SIZE: usize = 2_000_000;
    const SEED: [u8; 32] = [
        1, 0, 52, 0, 0, 0, 0, 0, 1, 0, 10, 0, 22, 32, 0, 0, 2, 0, 55, 49, 0, 11, 0, 0, 3, 0, 0, 0, 0, 0, 2, 92,
    ];

    #[tokio::test]
    async fn test_check_buffer_rewritten() {
        let rng = rand_chacha::ChaCha20Rng::from_seed(SEED);

        let mut rx = start_rng(rng, SIZE).await;

        let vec1 = take(&mut rx, SIZE)
            .await
            .expect("failed to generate SIZE entries for vec1");
        let vec2 = take(&mut rx, SIZE)
            .await
            .expect("failed to generate SIZE entries for vec2");

        assert_ne!(vec1, vec2);
    }

    async fn take(rx: &mut Receiver<u8>, n: usize) -> Result<Vec<u8>> {
        let mut res = Vec::with_capacity(n);
        while res.len() < n {
            let x = rx.recv().await.unwrap();
            res.push(x);
        }

        Ok(res)
    }
}
