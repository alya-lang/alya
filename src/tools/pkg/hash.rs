use super::discovery::collect_alya_files;
use std::fs;
use std::path::Path;

// ============================================================================
// Pure Rust SHA-256 (RFC 6234 / FIPS 180-4)
// ============================================================================

pub struct Sha256 {
    state: [u32; 8],
    count: u64,
    buffer: [u8; 64],
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            count: 0,
            buffer: [0u8; 64],
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut idx = 0;
        let mut buf_len = (self.count % 64) as usize;
        self.count += data.len() as u64;

        if buf_len > 0 && buf_len + data.len() >= 64 {
            let fill = 64 - buf_len;
            self.buffer[buf_len..64].copy_from_slice(&data[..fill]);
            Self::transform(&mut self.state, &self.buffer);
            idx += fill;
            buf_len = 0;
        }

        while idx + 64 <= data.len() {
            Self::transform(&mut self.state, data[idx..idx + 64].try_into().unwrap());
            idx += 64;
        }

        if idx < data.len() {
            let rem = data.len() - idx;
            self.buffer[buf_len..buf_len + rem].copy_from_slice(&data[idx..]);
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let total_bits = self.count * 8;
        let buf_len = (self.count % 64) as usize;

        self.buffer[buf_len] = 0x80;
        if buf_len + 1 > 56 {
            for b in &mut self.buffer[buf_len + 1..64] {
                *b = 0;
            }
            Self::transform(&mut self.state, &self.buffer);
            self.buffer = [0u8; 64];
        } else {
            for b in &mut self.buffer[buf_len + 1..56] {
                *b = 0;
            }
        }

        self.buffer[56..64].copy_from_slice(&total_bits.to_be_bytes());
        Self::transform(&mut self.state, &self.buffer);

        let mut out = [0u8; 32];
        for (i, &word) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn transform(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..(i + 1) * 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for (i, &w_val) in w.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w_val);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let bytes = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

pub fn compute_cache_key(name: &str, tag_or_branch: &str, url: &str) -> String {
    compute_cache_key_rev(name, tag_or_branch, url, None)
}

/// Revision-scoped cache key. Including the resolved commit SHA makes moved
/// tags (re-pointed `v0.1.0` baselines) naturally miss stale entries instead
/// of serving outdated checkouts. `None` preserves the legacy tag-only key
/// for offline fallbacks.
pub fn compute_cache_key_rev(
    name: &str,
    tag_or_branch: &str,
    url: &str,
    rev: Option<&str>,
) -> String {
    let sanitized_tag = tag_or_branch.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
    let url_hash = &sha256_hex(url.as_bytes())[..8];
    match rev {
        Some(r) if r.len() >= 7 => {
            format!(
                "{}@{}-{}-{}",
                name,
                sanitized_tag,
                url_hash,
                &r[..7.min(r.len())]
            )
        }
        _ => format!("{}@{}-{}", name, sanitized_tag, url_hash),
    }
}

pub fn compute_package_checksum(dir: &Path) -> Result<String, String> {
    checksum_with(dir, EolMode::Lf)
}

/// Legacy (lockfile v1) digest over raw file bytes. Line-ending sensitive:
/// identical revisions check out as CRLF on Windows without an LF pin and as
/// LF elsewhere, so v1 checksums are platform-polluted. Kept solely to
/// verify pre-existing v1 locks during migration.
pub fn compute_package_checksum_legacy(dir: &Path) -> Result<String, String> {
    checksum_with(dir, EolMode::Raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EolMode {
    Raw,
    Lf,
    Crlf,
}

/// Normalizes line endings for hashing: CRLF and lone CR become LF.
/// Operates on raw bytes; CR/LF are ASCII singletons that never appear inside
/// UTF-8 multibyte sequences, so this is encoding-safe.
fn normalize_newlines(data: &[u8]) -> Vec<u8> {
    if !data.contains(&b'\r') {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\r' {
            out.push(b'\n');
            if i + 1 < data.len() && data[i + 1] == b'\n' {
                i += 1;
            }
        } else {
            out.push(data[i]);
        }
        i += 1;
    }
    out
}

/// Converts LF to CRLF (via the normalized form, so mixed endings collapse
/// deterministically). Used to recognize v1 locks hashed from Windows
/// checkouts: CRLF-ifying the current tree and legacy-hashing it reproduces
/// the lock-time digest when the logical content is unchanged.
fn to_crlf_bytes(data: &[u8]) -> Vec<u8> {
    let lf = normalize_newlines(data);
    if !lf.contains(&b'\n') {
        return lf;
    }
    let mut out = Vec::with_capacity(lf.len() + 8);
    for b in lf {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    out
}

fn checksum_with(dir: &Path, mode: EolMode) -> Result<String, String> {
    let mut files = Vec::new();
    collect_alya_files(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    if files.is_empty() {
        return Ok(
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
    }

    let mut hasher = Sha256::new();
    for (rel_path, full_path) in files {
        hasher.update(rel_path.as_bytes());
        let content = fs::read(&full_path).map_err(|e| {
            format!(
                "Failed to read '{}' for checksum: {}",
                full_path.display(),
                e
            )
        })?;
        match mode {
            EolMode::Raw => hasher.update(&content),
            EolMode::Lf => hasher.update(&normalize_newlines(&content)),
            EolMode::Crlf => hasher.update(&to_crlf_bytes(&content)),
        }
    }
    let digest = sha256_hex(&hasher.finalize());
    Ok(format!("sha256:{}", digest))
}

/// Outcome of verifying an installed tree against a locked checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumVerdict {
    /// Digest matches under the lock's native algorithm.
    Match,
    /// Lock v1 whose raw-byte digest no longer matches (line-ending skew
    /// across platforms) but whose LF- or CRLF-canonicalized digest does.
    /// The install proceeds and the lock is rewritten as v2, self-healing
    /// the stale entry.
    LegacyHealed,
    /// No digest matches: genuine content drift or tampering.
    Mismatch,
}

/// Verifies an installed tree against a locked checksum.
///
/// - Lock v2+: normalized digest must match exactly.
/// - Lock v1 (or unversioned): the raw-byte digest is tried first to preserve
///   the original integrity guarantee; LF- and CRLF-canonicalized digests
///   are accepted as fallbacks so platform-polluted v1 locks (CRLF-born
///   locks verified on LF checkouts and vice versa) self-heal instead of
///   hard-failing. Every accepted path is an exact digest match, so tampered
///   content still fails all three and is rejected.
pub fn verify_package_checksum(
    dir: &Path,
    locked: &str,
    lock_version: u32,
) -> Result<ChecksumVerdict, String> {
    if lock_version >= 2 {
        return Ok(if checksum_with(dir, EolMode::Lf)? == locked {
            ChecksumVerdict::Match
        } else {
            ChecksumVerdict::Mismatch
        });
    }
    if checksum_with(dir, EolMode::Raw)? == locked {
        return Ok(ChecksumVerdict::Match);
    }
    if checksum_with(dir, EolMode::Lf)? == locked || checksum_with(dir, EolMode::Crlf)? == locked {
        return Ok(ChecksumVerdict::LegacyHealed);
    }
    Ok(ChecksumVerdict::Mismatch)
}
