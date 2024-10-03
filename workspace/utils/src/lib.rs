use std::sync::Mutex;

pub fn with_mutex<T, U>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> U) -> U {
    let mut l = m.lock().expect("Can't lock mutex");
    f(&mut l)
}

pub async fn with_mutex_async<T, U>(m: &tokio::sync::Mutex<T>, f: impl FnOnce(&mut T) -> U) -> U {
    let mut l = m.lock().await;
    f(&mut l)
}
