use std::{
    cell::RefCell,
    io::{self, Read},
};
use zstd::bulk::Compressor;

// The library supports regular compression levels from 1 up to ZSTD_maxCLevel(),
// which is currently 22. Levels >= 20
// Default level is ZSTD_CLEVEL_DEFAULT==3.
// value 0 means default, which is controlled by ZSTD_CLEVEL_DEFAULT
thread_local! {
    static COMPRESSOR: RefCell<io::Result<Compressor<'static>>> = RefCell::new(Compressor::new(crate::config::COMPRESS_LEVEL));
}

pub const DEFAULT_DECOMPRESS_MAX_LEN: usize = 4 * 1024 * 1024;

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    COMPRESSOR.with(|c| {
        if let Ok(mut c) = c.try_borrow_mut() {
            match &mut *c {
                Ok(c) => match c.compress(data) {
                    Ok(res) => out = res,
                    Err(err) => {
                        crate::log::debug!("Failed to compress: {}", err);
                    }
                },
                Err(err) => {
                    crate::log::debug!("Failed to get compressor: {}", err);
                }
            }
        }
    });
    out
}

pub fn decompress(data: &[u8]) -> Vec<u8> {
    decompress_limited(data, DEFAULT_DECOMPRESS_MAX_LEN).unwrap_or_default()
}

pub fn decompress_limited(data: &[u8], max_len: usize) -> io::Result<Vec<u8>> {
    let mut decoder = zstd::stream::read::Decoder::new(data)?;
    let limit = max_len.saturating_add(1) as u64;
    let mut output = Vec::new();
    decoder.by_ref().take(limit).read_to_end(&mut output)?;
    if output.len() > max_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("zstd decompressed output exceeds limit: {max_len}"),
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_rejects_output_over_default_limit() {
        let input = vec![0u8; DEFAULT_DECOMPRESS_MAX_LEN + 1];
        let compressed = compress(&input);
        assert!(decompress(&compressed).is_empty());
        assert!(decompress_limited(&compressed, DEFAULT_DECOMPRESS_MAX_LEN).is_err());
    }

    #[test]
    fn decompress_limited_accepts_context_limit() {
        let input = vec![7u8; DEFAULT_DECOMPRESS_MAX_LEN + 1];
        let compressed = compress(&input);
        let output = decompress_limited(&compressed, input.len()).unwrap();
        assert_eq!(output, input);
    }
}
