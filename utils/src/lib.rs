pub mod lumberyard;
pub mod types;

use crc32fast::Hasher;
use std::{borrow::Borrow, future::Future, time::Duration};
use tokio_util::sync::CancellationToken;

pub async fn race<F, U, C>(f: F, c: C) -> Option<U>
where
    F: Future<Output = U>,
    C: Borrow<CancellationToken>,
{
    tokio::select! {
        biased;
        _ = c.borrow().cancelled() => {
            None
        },
        r = f => {
            Some(r)
        },
    }
}

pub fn crc32(string: &str) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(string.as_bytes());
    hasher.finalize()
}

pub fn format_bytes(bytes: f64) -> String {
    const KILOBYTE: f64 = 1024.0;
    const MEGABYTE: f64 = KILOBYTE * 1024.0;
    const GIGABYTE: f64 = MEGABYTE * 1024.0;
    const TERABYTE: f64 = GIGABYTE * 1024.0;

    if bytes >= TERABYTE {
        format!("{:.2} TB", bytes / TERABYTE)
    } else if bytes >= GIGABYTE {
        format!("{:.2} GB", bytes / GIGABYTE)
    } else if bytes >= MEGABYTE {
        format!("{:.2} MB", bytes / MEGABYTE)
    } else if bytes >= KILOBYTE {
        format!("{:.2} KB", bytes / KILOBYTE)
    } else {
        format!("{:.2} B", bytes)
    }
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let minutes = secs / 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{:02}h:{:02}m:{:02}s", hours, minutes % 60, secs % 60)
    } else if minutes > 0 {
        format!("{:02}m:{:02}s", minutes, secs % 60)
    } else {
        format!("{:02}s", secs)
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
