//! MAC layer Mode 1 security: trait, helpers, and a software backend.
//!
//! See ETSI TS 103 636-4, §5.9.1 for the procedures. Mode 1 uses
//! AES-128-CMAC for integrity and AES-128-CTR for confidentiality, with
//! two distinct 16-byte keys.
//!
//! The crate's typestate builder ([`crate::mac::pdu::MacPduBuilder`])
//! and the secured parse entry point ([`crate::mac::pdu::Message::parse`])
//! handle the IV derivation, MIC compute / verify, and cipher-range
//! plumbing internally. Most callers only need to provide a backend
//! that implements [`MacCrypto`] (e.g. the bundled [`SoftwareCrypto`])
//! plus the two AES-128 keys and a [`SecurityContext`].

use core::ops::Range;

use subtle::ConstantTimeEq;

use crate::constants;
use crate::mac::headers::MacHeaderType;
use crate::mac::ie::{AnyIeType, InformationElement};
use crate::types::{
    IEType5bitLen0, IEType6bit, LongRdId, MacSecurity, SequenceNumber, ShortIeType,
};

/// Length of the MAC security trailer (MIC) in bytes when Mode 1
/// security is applied. See §5.9.1.2 / Figure 6.3.1-1.
pub const MIC_LEN: usize = 5;

/// Length of an AES-128 key in bytes.
pub const KEY_LEN: usize = 16;

/// Pluggable AES-based crypto operations needed for DECT NR+ MAC Mode 1
/// security. Implementors may delegate to software (see
/// [`SoftwareCrypto`]) or to hardware accelerators.
///
/// Both methods are infallible for the bundled software backend; the
/// associated `Error` type lets hardware backends surface device errors
/// (e.g. CryptoCell DMA failure) without forcing them to panic.
pub trait MacCrypto {
    /// Backend-specific error type.
    type Error;

    /// Compute AES-128-CMAC over `data` using `key`. Returns the full
    /// 16-byte tag; DECT truncates to the first [`MIC_LEN`] bytes for
    /// the MIC.
    fn cmac(&mut self, key: &[u8; KEY_LEN], data: &[u8]) -> Result<[u8; 16], Self::Error>;

    /// Apply AES-128-CTR (encrypt or decrypt; CTR is symmetric) to
    /// `data` in place using `key` and the 16-byte counter `iv`. The
    /// counter is incremented per 16-byte block internally per FIPS
    /// PUB 197.
    fn ctr_apply(
        &mut self,
        key: &[u8; KEY_LEN],
        iv: &[u8; 16],
        data: &mut [u8],
    ) -> Result<(), Self::Error>;
}

/// Addressing + counter inputs needed to derive a Mode 1 IV per
/// Table 5.9.1.3-1. The PSN is not stored here: on the builder side it
/// is captured automatically from the common-header push call, and
/// [`crate::mac::pdu::Message::parse`] reads it from the
/// received common header.
///
/// Per-header-type conventions:
/// * Beacon: set `rx = LongRdId::BROADCAST`.
/// * RD Broadcasting: set `rx = LongRdId::BROADCAST`.
/// * Unicast / Data: `tx` and `rx` are the two endpoints.
///
/// # HPC lifecycle (clause 5.9.1.3) - the caller's responsibility
///
/// The (HPC, PSN) pair is the AES-CTR nonce. Reusing a pair under the
/// same cipher key reuses the keystream and breaks confidentiality,
/// so the spec mandates:
///
/// * Data / Unicast / RD Broadcast: increment `hpc` by one whenever
///   the PSN is 0 or rolls over 0 from the previously transmitted
///   PSN.
/// * Beacon: increment `hpc` by at least one for EVERY secured
///   beacon. A beacon's PSN is fixed at 0, so the HPC is the only
///   varying IV input.
///
/// This crate keeps `hpc` a plain field and does not enforce these
/// rules; the MAC layer driving it must.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct SecurityContext {
    pub tx: LongRdId,
    pub rx: LongRdId,
    pub hpc: u32,
}

/// Errors surfaced by the secured-builder / secured-parser flows.
/// Generic over the crypto backend's error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MacSecurityError<E> {
    /// Buffer is shorter than the MIC trailer.
    BufferTooShort,
    /// MAC header byte has the reserved security value or an unknown
    /// MAC header type.
    InvalidHeader,
    /// Verify path: the computed MIC does not match what's in the PDU.
    BadMic,
    /// Underlying crypto backend returned an error.
    Crypto(E),
}

/// Non-generic error type returned by the crate-internal `cipher_range`
/// helper. Convertible into any [`MacSecurityError<E>`] via the `?`
/// operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CipherRangeError {
    /// Buffer is empty (no MAC header byte present).
    BufferTooShort,
    /// MAC header byte has the reserved security value or an unknown
    /// MAC header type.
    InvalidHeader,
}

impl<E> From<CipherRangeError> for MacSecurityError<E> {
    fn from(value: CipherRangeError) -> Self {
        match value {
            CipherRangeError::BufferTooShort => Self::BufferTooShort,
            CipherRangeError::InvalidHeader => Self::InvalidHeader,
        }
    }
}

impl<E> From<crate::BufferFull> for MacSecurityError<E> {
    fn from(_: crate::BufferFull) -> Self {
        Self::BufferTooShort
    }
}

impl<E> From<crate::ParsingError> for MacSecurityError<E> {
    fn from(_: crate::ParsingError) -> Self {
        Self::InvalidHeader
    }
}

impl<E> From<crate::SerializationError> for MacSecurityError<E> {
    // The only body serialized on this path is the fixed-layout MAC
    // Security Info IE, which can only fail on a short buffer.
    fn from(_: crate::SerializationError) -> Self {
        Self::BufferTooShort
    }
}

/// Build the 16-byte Mode 1 IV per Table 5.9.1.3-1.
///
/// Layout (bytes, big-endian within each field, MSB-first bit ordering):
/// * 0..=3: Tx Long RD ID
/// * 4..=7: Rx Long RD ID
/// * 8..=11: HPC
/// * 12..=15: PSN (12 bits) followed by the per-block counter (20 bits,
///   starts at 0; the CTR mode increments it per block).
#[must_use]
pub(crate) fn build_iv(ctx: &SecurityContext, psn: SequenceNumber) -> [u8; 16] {
    let mut iv = [0; 16];
    iv[0..4].copy_from_slice(&ctx.tx.as_u32().to_be_bytes());
    iv[4..8].copy_from_slice(&ctx.rx.as_u32().to_be_bytes());
    iv[8..12].copy_from_slice(&ctx.hpc.to_be_bytes());
    // PSN (12 bits) | counter (20 bits, starts at 0) packed BE into bytes 12..16.
    let psn_in_high_bits = (u32::from(psn.as_u16()) & 0x0FFF) << 20;
    iv[12..16].copy_from_slice(&psn_in_high_bits.to_be_bytes());
    iv
}

/// Return the byte range to encrypt/decrypt under Mode 1, given a PDU
/// `buffer` whose first byte is the MAC header type. Returns `None`
/// when the security field is `NotUsed`.
///
/// Per Table 6.3.2-1 the ciphered part starts immediately after the
/// MAC Common header (`UsedNoIe`) or immediately after the MAC
/// Security Info IE (`UsedWithIe`). For the latter the plaintext IE
/// stream is walked until the MAC Security Info IE is found; the spec
/// allows plaintext IEs to precede it (clause 5.9.1.3, "payload
/// length 0 option").
pub(crate) fn cipher_range(buffer: &[u8]) -> Result<Option<Range<usize>>, CipherRangeError> {
    let head_byte = *buffer.first().ok_or(CipherRangeError::BufferTooShort)?;
    let head = MacHeaderType(head_byte);
    let security = head
        .mac_security_typed()
        .ok_or(CipherRangeError::InvalidHeader)?;
    if matches!(security, MacSecurity::NotUsed) {
        return Ok(None);
    }
    let common_size =
        common_header_size(head.mac_header_type()).ok_or(CipherRangeError::InvalidHeader)?;
    let tail_start = 1 + common_size;
    if buffer.len() < tail_start {
        return Err(CipherRangeError::BufferTooShort);
    }
    match security {
        MacSecurity::NotUsed => Ok(None),
        MacSecurity::UsedNoIe => Ok(Some(tail_start..buffer.len())),
        MacSecurity::UsedWithIe => {
            let mut rest = &buffer[tail_start..];
            loop {
                if rest.is_empty() {
                    // Ran out of plaintext IEs without finding the
                    // MAC Security Info IE the header promised.
                    return Err(CipherRangeError::InvalidHeader);
                }
                let ie = InformationElement::parse(&mut rest)
                    .map_err(|_| CipherRangeError::InvalidHeader)?;
                let is_security_info = matches!(
                    ie.ie_number(),
                    AnyIeType::Type6bit(IEType6bit::MacSecurityInfo)
                        | AnyIeType::Type5bit(ShortIeType::Len0(IEType5bitLen0::MacSecurityInfo))
                );
                if is_security_info {
                    return Ok(Some(buffer.len() - rest.len()..buffer.len()));
                }
            }
        }
    }
}

/// Compute the MIC over `buffer[..len - MIC_LEN]` and write the
/// truncated 5-byte MIC into `buffer[len - MIC_LEN..]`. The caller must
/// have appended a [`MIC_LEN`]-byte placeholder before calling.
pub(crate) fn compute_mic<C: MacCrypto>(
    crypto: &mut C,
    key: &[u8; KEY_LEN],
    buffer: &mut [u8],
) -> Result<(), MacSecurityError<C::Error>> {
    if buffer.len() < MIC_LEN {
        return Err(MacSecurityError::BufferTooShort);
    }
    let mic_start = buffer.len() - MIC_LEN;
    let (data, mic_slot) = buffer.split_at_mut(mic_start);
    let tag = crypto.cmac(key, data).map_err(MacSecurityError::Crypto)?;
    mic_slot.copy_from_slice(&tag[..MIC_LEN]);
    Ok(())
}

/// Recompute the MIC over `buffer[..len - MIC_LEN]` and constant-time
/// compare it against `buffer[len - MIC_LEN..]`. Returns `true` iff
/// the MIC matches.
pub(crate) fn verify_mic<C: MacCrypto>(
    crypto: &mut C,
    key: &[u8; KEY_LEN],
    buffer: &[u8],
) -> Result<bool, MacSecurityError<C::Error>> {
    if buffer.len() < MIC_LEN {
        return Err(MacSecurityError::BufferTooShort);
    }
    let mic_start = buffer.len() - MIC_LEN;
    let (data, mic_in_pdu) = buffer.split_at(mic_start);
    let tag = crypto.cmac(key, data).map_err(MacSecurityError::Crypto)?;
    Ok(bool::from(tag[..MIC_LEN].ct_eq(mic_in_pdu)))
}

/// Common header byte length for each defined MAC header type, per
/// §6.3.3. Returns `None` for unknown / reserved values.
const fn common_header_size(mac_header_type: u8) -> Option<usize> {
    match mac_header_type {
        constants::mac_header_type::DATA_MAC_PDU => Some(2),
        constants::mac_header_type::BEACON => Some(7),
        constants::mac_header_type::UNICAST => Some(10),
        constants::mac_header_type::RD_BROADCAST => Some(6),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// NoCrypto backend (always available)
// ---------------------------------------------------------------------------

/// Error returned by [`NoCrypto`]: a secured PDU was encountered but
/// no crypto backend is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CryptoUnavailable;

/// A [`MacCrypto`] backend for devices that never use MAC security.
///
/// Both operations fail with [`CryptoUnavailable`], so
/// [`crate::mac::pdu::Message::parse`] still parses unsecured
/// PDUs normally (returning
/// [`crate::mac::pdu::ParsedPdu::Unsecured`]) while any secured PDU
/// is rejected with [`MacSecurityError::Crypto`]. This keeps a single
/// receive entry point on systems without security; pass all-zero key
/// arrays, they are never read.
///
/// Always available, independent of the `software-crypto` feature.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NoCrypto;

impl MacCrypto for NoCrypto {
    type Error = CryptoUnavailable;

    fn cmac(&mut self, _key: &[u8; KEY_LEN], _data: &[u8]) -> Result<[u8; 16], Self::Error> {
        Err(CryptoUnavailable)
    }

    fn ctr_apply(
        &mut self,
        _key: &[u8; KEY_LEN],
        _iv: &[u8; 16],
        _data: &mut [u8],
    ) -> Result<(), Self::Error> {
        Err(CryptoUnavailable)
    }
}

// ---------------------------------------------------------------------------
// Software backend (gated behind the `software-crypto` feature, default on)
// ---------------------------------------------------------------------------

#[cfg(feature = "software-crypto")]
pub mod software;

#[cfg(feature = "software-crypto")]
pub use software::SoftwareCrypto;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_iv_layout_matches_table() {
        let iv = build_iv(
            &SecurityContext {
                tx: LongRdId::new(0x11223344).unwrap(),
                rx: LongRdId::new(0x55667788).unwrap(),
                hpc: 0xAABBCCDD,
            },
            SequenceNumber::new(0x123).unwrap(),
        );
        assert_eq!(&iv[0..4], &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(&iv[4..8], &[0x55, 0x66, 0x77, 0x88]);
        assert_eq!(&iv[8..12], &[0xAA, 0xBB, 0xCC, 0xDD]);
        // PSN = 0x123 in bits 96..107; counter = 0 in bits 108..127.
        // Packed BE: PSN(12) | counter(20) = 0x123 << 20 = 0x12300000.
        assert_eq!(&iv[12..16], &[0x12, 0x30, 0x00, 0x00]);
    }

    #[test]
    #[expect(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn cipher_range_not_used_returns_none() {
        // Beacon header type, security = NotUsed.
        let buf = [0b00_00_0001];
        assert_eq!(cipher_range(&buf).unwrap(), None);
    }

    #[test]
    fn cipher_range_used_no_ie_unicast() {
        // Unicast (type=2), security=UsedNoIe (01). The ciphered part
        // starts immediately after the 10-byte common header
        // (Table 6.3.2-1): 1 + 10 = 11.
        let buf = [0x12u8; 32];
        assert_eq!(cipher_range(&buf).unwrap(), Some(11..32));
    }

    #[test]
    fn cipher_range_used_with_ie_beacon() {
        // Beacon (type=1), security=UsedWithIe (10). The ciphered part
        // starts immediately after the MAC Security Info IE
        // (Table 6.3.2-1).
        let mut buf = [0; 32];
        buf[0] = 0x21;
        // Bytes 1..8: 7-byte beacon common header (content irrelevant).
        // Bytes 8..15: MAC Security Info IE, 6-bit type with 8-bit
        // length (MacExt 01): head 0x50, length 5, 5 body bytes.
        buf[8] = 0x50;
        buf[9] = 5;
        // Cipher starts right after the IE: 8 + 2 + 5 = 15.
        assert_eq!(cipher_range(&buf).unwrap(), Some(15..32));
    }

    #[test]
    fn cipher_range_plaintext_ies_before_short_security_info() {
        // Unicast (type=2), security=UsedWithIe (10). Clause 5.9.1.3
        // allows plaintext IEs followed by the payload-length-0 MAC
        // Security Info short IE as the cipher-start marker.
        let mut buf = [0; 32];
        buf[0] = 0x22;
        // Bytes 1..11: 10-byte unicast common header.
        // Bytes 11..15: plaintext 6-bit IE (MacExt 01), type 0x01,
        // length 2, 2 payload bytes.
        buf[11] = 0x41;
        buf[12] = 2;
        // Byte 15: short MAC Security Info IE (MacExt 11, length flag
        // 0, 5-bit type 10000): head 0xD0.
        buf[15] = 0xD0;
        // Cipher starts right after the marker IE: 16.
        assert_eq!(cipher_range(&buf).unwrap(), Some(16..32));
    }

    #[test]
    fn cipher_range_used_with_ie_missing_security_info_errors() {
        // UsedWithIe header but the IE stream ends without a MAC
        // Security Info IE.
        let mut buf = [0; 12];
        buf[0] = 0x22;
        // Byte 11: short Padding IE (head 0xC0), then end of PDU.
        buf[11] = 0xC0;
        assert_eq!(
            cipher_range(&buf).unwrap_err(),
            CipherRangeError::InvalidHeader
        );
    }

    #[test]
    fn cipher_range_truncated_common_header_errors() {
        // UsedNoIe unicast header byte but only 5 of the 10 common
        // header bytes present.
        let buf = [0x12u8; 6];
        assert_eq!(
            cipher_range(&buf).unwrap_err(),
            CipherRangeError::BufferTooShort
        );
    }

    // Software-crypto-specific tests live in `software.rs` next to the
    // backend they exercise.
}
