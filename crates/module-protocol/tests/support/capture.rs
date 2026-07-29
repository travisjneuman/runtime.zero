use std::io::{self, Read};

#[derive(Debug)]
pub struct Capture {
    pub bytes: Vec<u8>,
    pub total_bytes: u64,
    pub truncated: bool,
}

pub fn drain_bounded(mut reader: impl Read, limit: u64) -> io::Result<Capture> {
    let capacity = usize::try_from(limit.min(64 * 1024)).unwrap_or(64 * 1024);
    let mut bytes = Vec::with_capacity(capacity);
    let mut total_bytes = 0_u64;
    let mut truncated = false;
    let mut block = [0_u8; 8 * 1024];
    loop {
        let count = reader.read(&mut block)?;
        if count == 0 {
            break;
        }
        total_bytes = total_bytes.saturating_add(count as u64);
        let remaining = limit.saturating_sub(bytes.len() as u64);
        let retained = usize::try_from(remaining.min(count as u64)).unwrap_or(0);
        bytes.extend_from_slice(&block[..retained]);
        if retained < count {
            truncated = true;
        }
    }
    Ok(Capture {
        bytes,
        total_bytes,
        truncated,
    })
}
