//! MAC PDU parsing and building.
//!
//! ETSI TS 103 636-4, clauses 6.3.1 through 6.3.4.

use core::marker::PhantomData;

use crate::constants;
use crate::mac::headers::{
    Beacon, DataMacPdu, MacCommonHeader, MacHeaderType, RdBroadcast, Unicast,
};
use crate::mac::ie::{AnyIeType, InformationElement};
use crate::mac::messages::MacSecurityInfoParts;
use crate::security::{self, KEY_LEN, MIC_LEN, MacCrypto, MacSecurityError, SecurityContext};
use crate::types::{
    IEType6bit, KeyIndex, LongRdId, MacExt, MacHeaderTypeKind, MacSecurity, NetworkId24,
    SecurityIvType, SecurityVersion, SequenceNumber, ShortIeType,
};
use crate::{BufferFull, ParsingError};

// ---------------------------------------------------------------------------
// MessageBody trait
// ---------------------------------------------------------------------------

/// A typed MAC message body that knows which 6-bit IE type carries it
/// on the wire.
///
/// Implemented by every `*Parts` struct under [`crate::mac::messages`].
/// Together with [`MacPduBuilder::push_body`] this lets a caller skip
/// the boilerplate of allocating a scratch buffer, calling
/// [`crate::mac::ie::InformationElement::new_6bit_with_length`], and
/// then [`MacPduBuilder::push_ie`]: instead, write the body directly
/// into the PDU buffer in one call.
pub trait MessageBody {
    /// IE type byte that carries this body on the wire.
    const IE_TYPE: IEType6bit;
    /// Number of bytes [`Self::serialize`] will write. Must match the
    /// value reported by the per-type `encoded_len()` method.
    fn encoded_len(&self) -> usize;
    /// Serialize the body into `out`. The output length must match
    /// [`Self::encoded_len`].
    ///
    /// # Errors
    ///
    /// Returns [`SerializationError::BufferTooShort`](crate::SerializationError::BufferTooShort)
    /// if `out` is too short, or
    /// [`ValueOutOfRange`](crate::SerializationError::ValueOutOfRange)
    /// when a list or value cannot be encoded in its on-wire field.
    fn serialize(&self, out: &mut [u8]) -> Result<usize, crate::SerializationError>;
}

/// A typed MAC message body carried by a 5-bit "Short IE" (MAC_Ext = 11).
///
/// Short IEs have a fixed length encoded in the IE type byte itself:
/// 0 or 1 payload byte. Sister trait to [`MessageBody`] for the 6-bit
/// IE registry. Pushed via [`MacPduBuilder::push_short_body`].
pub trait ShortMessageBody {
    /// IE type that carries this body on the wire. The registry it
    /// comes from determines whether 0 or 1 payload byte follows.
    const IE_TYPE: ShortIeType;
    /// Number of payload bytes [`Self::serialize`] will write. Must
    /// equal [`ShortIeType::len`] for `Self::IE_TYPE`. Always 0 or 1.
    fn encoded_len(&self) -> usize;
    /// Serialize the body into `out`. The output length must match
    /// [`Self::encoded_len`].
    ///
    /// # Errors
    ///
    /// Returns [`SerializationError::BufferTooShort`](crate::SerializationError::BufferTooShort)
    /// if `out` is too short. By construction no other errors are
    /// possible.
    fn serialize(&self, out: &mut [u8]) -> Result<usize, crate::SerializationError>;
}

/// Write `gap` bytes of Padding IEs at `pos` in `buf`, using the
/// encodings mandated by clause 6.4.3.8:
/// * `gap == 0`: no-op.
/// * `gap == 1`: one 5-bit Short Padding IE, length 0 (head 0xC0).
/// * `gap == 2`: one 5-bit Short Padding IE, length 1 (head 0xE0).
/// * `gap >= 3`: a 6-bit Padding IE with an 8-bit length field
///   (MAC_Ext 01, head 0x40, length = padding octets, zeroed payload).
///   The 8-bit length caps one such IE at 257 bytes, so larger gaps
///   are filled by chaining Padding IEs.
///
/// Returns the new `pos` after padding, or [`BufferFull`] if the buffer
/// has fewer than `gap` bytes left.
fn write_padding(buf: &mut [u8], pos: usize, gap: usize) -> Result<usize, BufferFull> {
    if buf.len() - pos < gap {
        return Err(BufferFull);
    }
    let mut pos = pos;
    let mut gap = gap;
    while gap > 0 {
        match gap {
            1 => {
                // MAC_Ext=11 (Short) | composite (len=0, type=0b00000) -> 0xC0
                buf[pos] = 0xC0;
                pos += 1;
                gap -= 1;
            }
            2 => {
                // MAC_Ext=11 (Short) | composite (len=1, type=0b00000) -> 0xE0
                buf[pos] = 0xE0;
                buf[pos + 1] = 0x00;
                pos += 2;
                gap -= 2;
            }
            _ => {
                // MAC_Ext=01 (8-bit length) | type=0b000000 -> 0x40,
                // then the length byte and that many arbitrary padding
                // octets (zeroed for determinism).
                let payload = usize::min(gap - 2, 255);
                buf[pos] = 0x40;
                buf[pos + 1] = payload as u8;
                buf[pos + 2..pos + 2 + payload].fill(0);
                pos += 2 + payload;
                gap -= 2 + payload;
            }
        }
    }
    Ok(pos)
}

/// Write a [`ShortMessageBody`] as a Short IE at `pos` in `buf`. The
/// 1-byte head (MAC_Ext=11 + 1-bit length + 5-bit type) is written
/// up-front; the 0 or 1 payload bytes are then serialized in place.
#[inline]
fn write_short_body<B: ShortMessageBody>(
    buf: &mut [u8],
    pos: usize,
    body: &B,
) -> Result<usize, BufferFull> {
    let body_len = body.encoded_len();
    debug_assert_eq!(
        body_len,
        B::IE_TYPE.len() as usize,
        "ShortMessageBody::encoded_len must match IE_TYPE.len()",
    );
    let total = 1 + body_len;
    if buf.len() - pos < total {
        return Err(BufferFull);
    }
    buf[pos] = (MacExt::ShortIe.as_u8() << 6) | B::IE_TYPE.composite();
    if body_len > 0 {
        body.serialize(&mut buf[pos + 1..pos + total])
            .expect("encoded_len matches serialize output for valid bodies");
    }
    Ok(pos + total)
}

/// Write a [`MessageBody`] as an IE at `pos` in `buf`. Computes the IE
/// header (1 head byte + 1 or 2 length bytes) up front so the body is
/// serialized into its final position with no intermediate copy.
#[inline]
fn write_body<B: MessageBody>(buf: &mut [u8], pos: usize, body: &B) -> Result<usize, BufferFull> {
    let body_len = body.encoded_len();
    let (mac_ext, hdr_bytes) = if body_len > u8::MAX as usize {
        (MacExt::Length16Bit, 3)
    } else {
        (MacExt::Length8Bit, 2)
    };
    let total = hdr_bytes + body_len;
    if buf.len() - pos < total {
        return Err(BufferFull);
    }
    buf[pos] = (mac_ext.as_u8() << 6) | (u8::from(B::IE_TYPE) & 0x3F);
    if hdr_bytes == 2 {
        buf[pos + 1] = body_len as u8;
    } else {
        buf[pos + 1..pos + 3].copy_from_slice(&(body_len as u16).to_be_bytes());
    }
    body.serialize(&mut buf[pos + hdr_bytes..pos + total])
        .expect("encoded_len matches serialize output for valid bodies");
    Ok(pos + total)
}

// ---------------------------------------------------------------------------
// Message - parsed MAC PDU
// ---------------------------------------------------------------------------

/// Parsed MAC PDU. Holds the 1-byte header type, the parsed common
/// header, the IE-stream tail (with the MAC security MIC trailer already
/// split off when applicable), and an optional reference to that MIC.
///
/// ETSI TS 103 636-4, clause 6.3.1.
#[derive(Clone, Copy)]
pub struct Message<'a> {
    /// 1-byte header type (clause 6.3.2).
    pub head: MacHeaderType,
    /// Common header, dispatched on `head.mac_header_type()` (clause 6.3.3).
    pub common: MacCommonHeader<'a>,
    /// IE stream bytes. When MAC security is applied, the trailing
    /// [`MIC_LEN`]-byte MIC has already been split off and is available
    /// in [`Self::mic`].
    pub tail: &'a [u8],
    /// `Some(..)` iff `head.mac_security_typed()` indicates security is
    /// applied (`UsedNoIe` or `UsedWithIe`). When set, contains the
    /// 5-byte MIC trailer from the end of the original buffer.
    pub mic: Option<&'a [u8; MIC_LEN]>,
}

impl<'a> Message<'a> {
    /// Split a buffer into MAC header, common header, IE tail, and MIC
    /// trailer WITHOUT any authentication or decryption.
    ///
    /// Nothing about the result is verified: on a secured PDU the
    /// `tail` is still ciphertext and the MIC has not been checked.
    /// Use this only to read plaintext header fields before the real
    /// [`Self::parse`] call, e.g. the transmitter Long RD ID to look
    /// up the peer's keys / [`SecurityContext`], or via
    /// [`Self::peek_security_info`] for the HPC. Never act on the
    /// tail of a secured PDU obtained this way.
    ///
    /// Validates that `head.version() == 0` (the only version
    /// defined). When the MAC security field indicates security is
    /// applied, the last [`MIC_LEN`] bytes are split off the buffer
    /// into [`Self::mic`] and the IE-stream `tail` excludes them.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] if the buffer is too short, if the
    /// version / header type is unrecognized, or if the security field
    /// holds the reserved value `0b11`.
    pub fn parse_unverified(buffer: &'a [u8]) -> Result<Self, ParsingError> {
        let head_byte = *buffer.first().ok_or(ParsingError::Truncated)?;
        let head = MacHeaderType(head_byte);

        if head.version() != constants::MAC_VERSION {
            return Err(ParsingError::InvalidHeader);
        }

        let security = head
            .mac_security_typed()
            .ok_or(ParsingError::ReservedValue)?;
        let security_active = !matches!(security, crate::types::MacSecurity::NotUsed);

        // Carve out the MIC trailer first (if security is active) so that
        // the IE-stream tail and the rest-of-PDU Padding rule see only
        // IE-stream bytes.
        let (working, mic) = if security_active {
            if buffer.len() < 1 + MIC_LEN {
                return Err(ParsingError::Truncated);
            }
            let mic_start = buffer.len() - MIC_LEN;
            let mic_slice: &[u8; MIC_LEN] = buffer[mic_start..]
                .try_into()
                .expect("length checked above");
            (&buffer[..mic_start], Some(mic_slice))
        } else {
            (buffer, None)
        };

        let after_header = &working[1..];

        let kind = MacHeaderTypeKind::try_from_u8(head.mac_header_type())
            .ok_or(ParsingError::InvalidHeader)?;
        let (common, tail) = match kind {
            MacHeaderTypeKind::DataMacPdu => split_common::<2>(after_header)
                .map(|(b, t)| (MacCommonHeader::DataMacPdu(DataMacPdu(b)), t))?,
            MacHeaderTypeKind::Beacon => split_common::<7>(after_header)
                .map(|(b, t)| (MacCommonHeader::Beacon(Beacon(b)), t))?,
            MacHeaderTypeKind::Unicast => split_common::<10>(after_header)
                .map(|(b, t)| (MacCommonHeader::Unicast(Unicast(b)), t))?,
            MacHeaderTypeKind::RdBroadcast => split_common::<6>(after_header)
                .map(|(b, t)| (MacCommonHeader::RdBroadcast(RdBroadcast(b)), t))?,
            MacHeaderTypeKind::Escape => return Err(ParsingError::InvalidHeader),
        };

        Ok(Self {
            head,
            common,
            tail,
            mic,
        })
    }

    /// Iterate IEs from the tail. The items borrow the underlying
    /// receive buffer (`'a`), not this `Message` value, so an
    /// [`InformationElement`] may outlive the `Message` it came from.
    #[inline]
    pub fn tail_items(
        &self,
    ) -> impl Iterator<Item = Result<InformationElement<'a>, ParsingError>> + use<'a> {
        InformationElement::parse_stream(self.tail)
    }

    /// Decrypt + verify + parse a received Mode 1 PDU in one call.
    ///
    /// This is the single receive entry point for secured and
    /// unsecured traffic alike; the returned [`ParsedPdu`] makes the
    /// security status impossible to ignore.
    ///
    /// Steps performed internally:
    /// 1. Peek the MAC header byte to determine the security field.
    /// 2. If `NotUsed`, parse as plaintext and return
    ///    [`ParsedPdu::Unsecured`]. No crypto operation runs; whether
    ///    unsecured PDUs are acceptable is the caller's match arm.
    /// 3. Otherwise read the PSN from the (unciphered) common header,
    ///    compute the cipher range (clause 5.9.1.3), and CTR-decrypt
    ///    that range in place. The header, PSN included, is covered by
    ///    the MIC, so a tampered PSN yields a wrong IV and fails the
    ///    MIC check; the caller never has to supply (and so can never
    ///    mismatch) the PSN.
    /// 4. CMAC-verify the MIC trailer with a constant-time compare.
    /// 5. Parse the now-plaintext buffer and return
    ///    [`ParsedPdu::Secured`].
    ///
    /// Replay protection is the caller's job: after a successful parse,
    /// validate `msg.common.sequence_number()` (and the HPC used in
    /// `ctx`) against your receive window before accepting the payload.
    ///
    /// On systems that never use security, pass
    /// [`crate::security::NoCrypto`] and zeroed keys: unsecured PDUs
    /// parse normally and secured ones are rejected with
    /// [`MacSecurityError::Crypto`].
    ///
    /// # Errors
    ///
    /// Returns [`MacSecurityError`] for short buffer, reserved security
    /// value, MIC mismatch, crypto backend failure, or downstream
    /// [`ParsingError`] from the plaintext parse. The plaintext parse
    /// error is folded into `MacSecurityError::InvalidHeader` for
    /// simplicity in the secured path; if you need to distinguish, use
    /// the low-level helpers and call [`Self::parse_unverified`] directly.
    pub fn parse<C: MacCrypto>(
        buffer: &'a mut [u8],
        crypto: &mut C,
        integrity_key: &[u8; KEY_LEN],
        cipher_key: &[u8; KEY_LEN],
        ctx: &SecurityContext,
    ) -> Result<ParsedPdu<'a>, MacSecurityError<C::Error>> {
        let Some(range) = security::cipher_range(buffer)? else {
            let msg =
                Self::parse_unverified(buffer).map_err(|_| MacSecurityError::InvalidHeader)?;
            return Ok(ParsedPdu::Unsecured(msg));
        };
        // The common header sits before the cipher range, so the
        // on-wire PSN is readable before decryption.
        let psn = Message::parse_unverified(&buffer[..])
            .map_err(|_| MacSecurityError::InvalidHeader)?
            .common
            .sequence_number();
        let iv = security::build_iv(ctx, psn);
        crypto
            .ctr_apply(cipher_key, &iv, &mut buffer[range])
            .map_err(MacSecurityError::Crypto)?;
        let ok = security::verify_mic(crypto, integrity_key, buffer)?;
        if !ok {
            return Err(MacSecurityError::BadMic);
        }
        let msg = Self::parse_unverified(buffer).map_err(|_| MacSecurityError::InvalidHeader)?;
        Ok(ParsedPdu::Secured(msg))
    }

    /// Peek the plaintext MAC Security Info IE of a received (still
    /// encrypted) `UsedWithIe` PDU, without decrypting anything.
    ///
    /// Per Table 6.3.2-1 the IE sits before the ciphered part, so a
    /// receiver can extract the transmitter's HPC (and key index / IV
    /// type) here, update its [`SecurityContext`], and then call
    /// [`Self::parse`]. This is the HPC (re)synchronization
    /// path of clause 5.9.1.1.
    ///
    /// Returns `None` if the header does not indicate `UsedWithIe`, if
    /// no full MAC Security Info IE is found in the plaintext IE
    /// prefix (the zero-length short form carries no HPC), or if the
    /// IE fails to parse.
    #[must_use]
    pub fn peek_security_info(buffer: &[u8]) -> Option<MacSecurityInfoParts> {
        let msg = Message::parse_unverified(buffer).ok()?;
        if msg.head.mac_security_typed() != Some(crate::types::MacSecurity::UsedWithIe) {
            return None;
        }
        for ie in msg.tail_items() {
            // Stop at the first malformed element: everything at and
            // beyond the security IE boundary is ciphertext.
            let ie = ie.ok()?;
            match ie.ie_number() {
                AnyIeType::Type6bit(IEType6bit::MacSecurityInfo) => {
                    return MacSecurityInfoParts::parse(ie.payload()).ok();
                }
                AnyIeType::Type5bit(ShortIeType::Len0(
                    crate::types::IEType5bitLen0::MacSecurityInfo,
                )) => {
                    return None;
                }
                _ => continue,
            }
        }
        None
    }
}

/// Outcome of [`Message::parse`]: a parsed PDU together with
/// its (unforgeable) security status.
///
/// The status is part of the type so a caller cannot accidentally
/// treat attacker-stripped plaintext as verified data: getting at the
/// [`Message`] forces a decision about which arm is acceptable on
/// this link.
#[must_use]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ParsedPdu<'a> {
    /// The PDU was CTR-decrypted in place and its MIC verified.
    Secured(Message<'a>),
    /// The MAC header indicated `NotUsed`: the PDU parsed fine but
    /// carries no integrity protection whatsoever.
    Unsecured(Message<'a>),
}

impl<'a> ParsedPdu<'a> {
    /// True for [`ParsedPdu::Secured`].
    #[must_use]
    #[inline]
    pub const fn is_secured(&self) -> bool {
        matches!(self, ParsedPdu::Secured(_))
    }

    /// The parsed message regardless of security status. Prefer
    /// matching the enum unless your link-layer policy genuinely
    /// accepts both.
    #[must_use]
    #[inline]
    pub fn into_message(self) -> Message<'a> {
        match self {
            ParsedPdu::Secured(m) | ParsedPdu::Unsecured(m) => m,
        }
    }

    /// The parsed message iff the MIC verified, `None` for unsecured
    /// PDUs. The one-liner for security-mandatory receivers.
    #[must_use]
    #[inline]
    pub fn secured(self) -> Option<Message<'a>> {
        match self {
            ParsedPdu::Secured(m) => Some(m),
            ParsedPdu::Unsecured(_) => None,
        }
    }
}

#[inline]
fn split_common<const N: usize>(buffer: &[u8]) -> Result<(&[u8; N], &[u8]), ParsingError> {
    if buffer.len() < N {
        return Err(ParsingError::Truncated);
    }
    let (head, tail) = buffer.split_at(N);
    let head: &[u8; N] = head.try_into().expect("length checked above");
    Ok((head, tail))
}

impl core::fmt::Debug for Message<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Message")
            .field("head", &self.head)
            .field("common", &self.common)
            .field("tail_len", &self.tail.len())
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Message<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "Message {{ sec: {=u8}, ", self.head.mac_security());
        match &self.common {
            MacCommonHeader::DataMacPdu(h) => h.format(f),
            MacCommonHeader::Beacon(h) => h.format(f),
            MacCommonHeader::Unicast(h) => h.format(f),
            MacCommonHeader::RdBroadcast(h) => h.format(f),
        }
        defmt::write!(f, ", tail: {=usize} bytes", self.tail.len());
    }
}

// ---------------------------------------------------------------------------
// MacPduBuilder - write into a caller-provided buffer
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Typestate marker types
// ---------------------------------------------------------------------------

mod sealed {
    pub trait Sealed {}
}

/// Initial builder state: no MAC header has been written. Transitions
/// to one of the `Header*` states via a `push_*_header` call.
pub enum NoHeader {}
/// Builder state after a header with `MacSecurity::NotUsed`. Allows
/// [`MacPduBuilder::push_ie`] and [`MacPduBuilder::finish_without_security`].
pub enum HeaderUnsecured {}
/// Builder state after a header with `MacSecurity::UsedWithIe`. The
/// only legal next call is [`MacPduBuilder::push_mac_security_info`].
pub enum HeaderSecuredAwait {}
/// Builder state once the MAC Security Info IE has been pushed (under
/// `UsedWithIe`) or immediately after a `UsedNoIe` header. Allows
/// [`MacPduBuilder::push_ie`] and
/// [`MacPduBuilder::finish_with_security`].
pub enum HeaderSecuredReady {}

/// Marker type passed to `push_*_header` to select
/// [`MacSecurity::NotUsed`] at the type level.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotUsed;
/// Marker for [`MacSecurity::UsedNoIe`].
#[derive(Debug, Clone, Copy, Default)]
pub struct UsedNoIe;
/// Marker for [`MacSecurity::UsedWithIe`].
#[derive(Debug, Clone, Copy, Default)]
pub struct UsedWithIe;

impl sealed::Sealed for NotUsed {}
impl sealed::Sealed for UsedNoIe {}
impl sealed::Sealed for UsedWithIe {}

/// Sealed trait that maps a security-mode marker
/// ([`NotUsed`] / [`UsedNoIe`] / [`UsedWithIe`]) to the resulting
/// builder state and the on-wire 2-bit security field value.
pub trait SecurityMode: sealed::Sealed {
    /// Typestate after `push_<header>` runs.
    type AfterHeader;
    /// Raw 2-bit MAC Security field value.
    const FIELD: u8;
}

/// Security modes legal for a Beacon header. Clause 5.9.1.3 requires
/// secured beacons to carry the MAC Security Info IE (a beacon header
/// has no PSN for the receiver to synchronize on), so [`UsedNoIe`] is
/// excluded at the type level.
pub trait BeaconSecurityMode: SecurityMode {}
impl BeaconSecurityMode for NotUsed {}
impl BeaconSecurityMode for UsedWithIe {}

impl SecurityMode for NotUsed {
    type AfterHeader = HeaderUnsecured;
    const FIELD: u8 = constants::mac_security::NOT_USED;
}
impl SecurityMode for UsedNoIe {
    type AfterHeader = HeaderSecuredReady;
    const FIELD: u8 = constants::mac_security::USED_NO_IE;
}
impl SecurityMode for UsedWithIe {
    type AfterHeader = HeaderSecuredAwait;
    const FIELD: u8 = constants::mac_security::USED_WITH_IE;
}

// ---------------------------------------------------------------------------
// MacPduBuilder
// ---------------------------------------------------------------------------

/// Typestate builder that writes a MAC PDU into a caller-provided
/// buffer. The state parameter (default [`NoHeader`]) tracks which
/// operations are valid at the type level - each state type documents
/// what it permits.
pub struct MacPduBuilder<'a, State = NoHeader> {
    buf: &'a mut [u8],
    pos: usize,
    /// Position in `buf` where the cipher range starts. For
    /// `HeaderSecuredReady` this is `1 + common_size` (`UsedNoIe`) or
    /// the offset just past the MAC Security Info IE (`UsedWithIe`),
    /// per Table 6.3.2-1. Unused in `NoHeader`, `HeaderUnsecured`,
    /// and `HeaderSecuredAwait`.
    cipher_start: usize,
    /// Byte offset of the 4-byte HPC field inside the pushed MAC
    /// Security Info IE, or 0 when none was pushed.
    /// `finish_with_security` patches `ctx.hpc` into this slot so the
    /// on-air HPC and the IV HPC cannot diverge.
    hpc_offset: usize,
    /// Sequence number captured from the just-pushed common header.
    /// Beacon headers have no PSN; the spec uses PSN=0 in the IV
    /// derivation, so we initialise the builder with PSN=0 and
    /// overwrite it when a non-beacon header is pushed.
    psn: SequenceNumber,
    _state: PhantomData<State>,
}

// ---- NoHeader state: new + push_*_header ----------------------------------

impl<'a> MacPduBuilder<'a, NoHeader> {
    /// Create a builder over `buf` starting at offset 0.
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            cipher_start: 0,
            hpc_offset: 0,
            psn: const { SequenceNumber::try_from_u16(0).expect("0 is a valid SequenceNumber") },
            _state: PhantomData,
        }
    }

    /// Remaining capacity in the buffer.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Write a Data MAC PDU header (Figure 6.3.2-1 + 6.3.3.1-1).
    #[inline]
    pub fn push_data_mac_pdu<S: SecurityMode>(
        self,
        _security: S,
        reset: bool,
        sequence_number: SequenceNumber,
    ) -> Result<MacPduBuilder<'a, S::AfterHeader>, BufferFull> {
        let common = DataMacPdu::new_owned(reset, sequence_number);
        self.write_header_then::<S>(
            constants::mac_header_type::DATA_MAC_PDU,
            &common,
            sequence_number,
        )
    }

    /// Write a Beacon header (Figure 6.3.2-1 + 6.3.3.2-1).
    ///
    /// The security marker is [`BeaconSecurityMode`]-bounded:
    /// [`UsedNoIe`] does not compile here, see the trait docs.
    #[inline]
    pub fn push_beacon<S: BeaconSecurityMode>(
        self,
        _security: S,
        network_id: NetworkId24,
        transmitter: LongRdId,
    ) -> Result<MacPduBuilder<'a, S::AfterHeader>, BufferFull> {
        let common = Beacon::new_owned(network_id, transmitter);
        // Beacon has no PSN in its common header; the spec uses PSN = 0
        // for the IV derivation.
        self.write_header_then::<S>(
            constants::mac_header_type::BEACON,
            &common,
            const { SequenceNumber::try_from_u16(0).expect("0 is a valid SequenceNumber") },
        )
    }

    /// Write a Unicast header (Figure 6.3.2-1 + 6.3.3.3-1).
    #[inline]
    pub fn push_unicast<S: SecurityMode>(
        self,
        _security: S,
        reset: bool,
        sequence_number: SequenceNumber,
        receiver: LongRdId,
        transmitter: LongRdId,
    ) -> Result<MacPduBuilder<'a, S::AfterHeader>, BufferFull> {
        let common = Unicast::new_owned(reset, sequence_number, receiver, transmitter);
        self.write_header_then::<S>(
            constants::mac_header_type::UNICAST,
            &common,
            sequence_number,
        )
    }

    /// Write an RD Broadcasting header (Figure 6.3.2-1 + 6.3.3.4-1).
    #[inline]
    pub fn push_rd_broadcast<S: SecurityMode>(
        self,
        _security: S,
        reset: bool,
        sequence_number: SequenceNumber,
        transmitter: LongRdId,
    ) -> Result<MacPduBuilder<'a, S::AfterHeader>, BufferFull> {
        let common = RdBroadcast::new_owned(reset, sequence_number, transmitter);
        self.write_header_then::<S>(
            constants::mac_header_type::RD_BROADCAST,
            &common,
            sequence_number,
        )
    }

    #[inline]
    fn write_header_then<S: SecurityMode>(
        mut self,
        mac_header_type: u8,
        common_bytes: &[u8],
        psn: SequenceNumber,
    ) -> Result<MacPduBuilder<'a, S::AfterHeader>, BufferFull> {
        let need = 1 + common_bytes.len();
        if self.remaining() < need {
            return Err(BufferFull);
        }
        let security = MacSecurity::try_from_u8(S::FIELD).expect("SecurityMode is valid");
        let head = MacHeaderType::new(security, mac_header_type).expect("type fits in 4 bits");
        self.buf[self.pos] = head.0;
        self.pos += 1;
        self.buf[self.pos..self.pos + common_bytes.len()].copy_from_slice(common_bytes);
        self.pos += common_bytes.len();
        // For UsedNoIe the ciphered part starts immediately after the
        // MAC Common header (Table 6.3.2-1); for UsedWithIe it will be
        // set by push_mac_security_info; for NotUsed it's unused.
        let cipher_start = if S::FIELD == constants::mac_security::USED_NO_IE {
            self.pos
        } else {
            0
        };
        Ok(MacPduBuilder {
            buf: self.buf,
            pos: self.pos,
            cipher_start,
            hpc_offset: 0,
            psn,
            _state: PhantomData,
        })
    }
}

// ---- Helpers shared by states that can write IEs --------------------------

#[inline]
fn write_ie(buf: &mut [u8], pos: usize, ie: &InformationElement<'_>) -> Result<usize, BufferFull> {
    let extra = ie.encoded_len();
    if buf.len() - pos < extra {
        return Err(BufferFull);
    }
    let head = ie.head();
    let payload = ie.payload();
    let dst = &mut buf[pos..pos + extra];
    dst[0] = head;
    let body_off = match MacExt::try_from_u8(head >> 6) {
        Some(MacExt::Length8Bit) => {
            dst[1] = payload.len() as u8;
            2
        }
        Some(MacExt::Length16Bit) => {
            dst[1..3].copy_from_slice(&(payload.len() as u16).to_be_bytes());
            3
        }
        _ => 1,
    };
    dst[body_off..].copy_from_slice(payload);
    Ok(pos + extra)
}

// ---- HeaderUnsecured state ------------------------------------------------

impl<'a> MacPduBuilder<'a, HeaderUnsecured> {
    /// Append an IE to the tail. Takes `self` by value and returns
    /// `Self` so callers can chain into [`Self::finish_without_security`].
    /// In a loop, reassign: `b = b.push_ie(ie)?`.
    #[inline]
    pub fn push_ie(mut self, ie: &InformationElement<'_>) -> Result<Self, BufferFull> {
        self.pos = write_ie(self.buf, self.pos, ie)?;
        Ok(self)
    }

    /// Append a typed message body as an IE in one call. The body's
    /// [`MessageBody::IE_TYPE`] is used for the IE header; no temporary
    /// scratch buffer is needed because the body is serialized directly
    /// into the builder buffer.
    #[inline]
    pub fn push_body<B: MessageBody>(mut self, body: &B) -> Result<Self, BufferFull> {
        self.pos = write_body(self.buf, self.pos, body)?;
        Ok(self)
    }

    /// Append a typed 5-bit "Short IE" message body in one call. The
    /// 1-byte head encodes both the IE type and its length flag (0 or 1
    /// payload byte). See [`ShortMessageBody`].
    #[inline]
    pub fn push_short_body<B: ShortMessageBody>(mut self, body: &B) -> Result<Self, BufferFull> {
        self.pos = write_short_body(self.buf, self.pos, body)?;
        Ok(self)
    }

    /// Remaining capacity in the buffer.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Finalize an unsecured PDU. Returns the borrowed slice of bytes
    /// written. No MIC is appended, no encryption performed.
    #[inline]
    pub fn finish_without_security(self) -> &'a [u8] {
        let MacPduBuilder { buf, pos, .. } = self;
        &buf[..pos]
    }

    /// Finalize an unsecured PDU at exactly `target_len` bytes by
    /// appending Padding IEs to fill any gap. Picks the smallest
    /// encoding that matches the remaining gap (5-bit Short Padding for
    /// 1 or 2 bytes; 6-bit Padding with an 8-bit length for >= 3 bytes).
    ///
    /// Useful when the PHY layer's transport block size (see
    /// [`crate::subslot::compute_tbs`]) requires the PDU to be a
    /// specific length.
    ///
    /// # Errors
    ///
    /// Returns [`BufferFull`] if `target_len` is less than the bytes
    /// already written, or if it exceeds the buffer capacity.
    #[inline]
    pub fn finish_without_security_padded(self, target_len: usize) -> Result<&'a [u8], BufferFull> {
        let MacPduBuilder { buf, pos, .. } = self;
        if target_len < pos {
            return Err(BufferFull);
        }
        let gap = target_len - pos;
        let end = write_padding(buf, pos, gap)?;
        Ok(&buf[..end])
    }
}

// ---- HeaderSecuredAwait state: only push_mac_security_info ---------------

impl<'a> MacPduBuilder<'a, HeaderSecuredAwait> {
    /// Append the MAC Security Info IE as the first IE in the stream
    /// and transition to [`HeaderSecuredReady`]. The ciphered part
    /// starts immediately after this IE (Table 6.3.2-1), so the IE,
    /// and the HPC it carries for IV synchronization, stays plaintext
    /// on the wire.
    ///
    /// The HPC is not a parameter: `finish_with_security` writes
    /// `ctx.hpc` into the IE, so the on-air HPC and the one used in
    /// the IV derivation cannot diverge.
    #[inline]
    pub fn push_mac_security_info(
        mut self,
        version: SecurityVersion,
        key_index: KeyIndex,
        iv_type: SecurityIvType,
    ) -> Result<MacPduBuilder<'a, HeaderSecuredReady>, BufferFull> {
        let parts = MacSecurityInfoParts {
            version,
            key_index,
            iv_type,
            // Placeholder: patched with ctx.hpc by finish_with_security.
            hpc: 0,
        };
        let mut body_bytes = [0; 5];
        parts
            .serialize(&mut body_bytes)
            .expect("MacSecurityInfoParts always fits in 5 bytes");
        let ie = InformationElement::new_6bit_with_length(IEType6bit::MacSecurityInfo, &body_bytes)
            .expect("5 bytes fits in u16 length");
        // HPC at +3: IE head (1) + 8-bit length (1) + version/key/iv (1).
        let hpc_offset = self.pos + 3;
        self.pos = write_ie(self.buf, self.pos, &ie)?;
        let cipher_start = self.pos;
        Ok(MacPduBuilder {
            buf: self.buf,
            pos: self.pos,
            cipher_start,
            hpc_offset,
            psn: self.psn,
            _state: PhantomData,
        })
    }
}

// ---- HeaderSecuredReady state: push_ie + finish_with_security -------------

impl<'a> MacPduBuilder<'a, HeaderSecuredReady> {
    /// Append an IE to the tail. Takes `self` by value and returns
    /// `Self` so callers can chain into [`Self::finish_with_security`].
    /// In a loop, reassign: `b = b.push_ie(ie)?`.
    #[inline]
    pub fn push_ie(mut self, ie: &InformationElement<'_>) -> Result<Self, BufferFull> {
        self.pos = write_ie(self.buf, self.pos, ie)?;
        Ok(self)
    }

    /// Append a typed message body as an IE in one call. See
    /// [`MacPduBuilder::<HeaderUnsecured>::push_body`] for details.
    #[inline]
    pub fn push_body<B: MessageBody>(mut self, body: &B) -> Result<Self, BufferFull> {
        self.pos = write_body(self.buf, self.pos, body)?;
        Ok(self)
    }

    /// Append a typed 5-bit "Short IE" message body in one call. See
    /// [`MacPduBuilder::<HeaderUnsecured>::push_short_body`] for details.
    #[inline]
    pub fn push_short_body<B: ShortMessageBody>(mut self, body: &B) -> Result<Self, BufferFull> {
        self.pos = write_short_body(self.buf, self.pos, body)?;
        Ok(self)
    }

    /// Remaining capacity in the buffer.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Finalize a secured PDU: writes `ctx.hpc` into the MAC Security
    /// Info IE (when one was pushed), appends a [`MIC_LEN`]-byte MIC
    /// computed with `integrity_key` over the whole PDU, then encrypts
    /// the spec-defined range (clause 5.9.1.3) with `cipher_key` and
    /// an IV derived from `ctx` and the PSN extracted from the common
    /// header.
    pub fn finish_with_security<C: MacCrypto>(
        self,
        crypto: &mut C,
        integrity_key: &[u8; KEY_LEN],
        cipher_key: &[u8; KEY_LEN],
        ctx: &SecurityContext,
    ) -> Result<&'a [u8], MacSecurityError<C::Error>> {
        let MacPduBuilder {
            buf,
            mut pos,
            cipher_start,
            hpc_offset,
            psn,
            ..
        } = self;
        // Patch the IV's HPC into the (still plaintext, MIC-covered)
        // MAC Security Info IE so the two cannot diverge.
        if hpc_offset != 0 {
            buf[hpc_offset..hpc_offset + 4].copy_from_slice(&ctx.hpc.to_be_bytes());
        }
        // Append 5-byte MIC placeholder (zeros).
        if buf.len() - pos < MIC_LEN {
            return Err(MacSecurityError::BufferTooShort);
        }
        for slot in &mut buf[pos..pos + MIC_LEN] {
            *slot = 0;
        }
        pos += MIC_LEN;
        // Compute MIC over buf[..pos - MIC_LEN], write into buf[pos - MIC_LEN..pos].
        security::compute_mic(crypto, integrity_key, &mut buf[..pos])?;
        // Build IV from the supplied SecurityContext and the PSN
        // captured from the common header when it was pushed: this
        // makes a builder-side / context-side PSN mismatch unrepresentable.
        let iv = security::build_iv(ctx, psn);
        crypto
            .ctr_apply(cipher_key, &iv, &mut buf[cipher_start..pos])
            .map_err(MacSecurityError::Crypto)?;
        Ok(&buf[..pos])
    }

    /// Finalize a secured PDU at exactly `target_len` bytes (including
    /// the 5-byte MIC trailer) by appending Padding IEs to fill the gap
    /// before computing the MIC and encrypting. See
    /// [`MacPduBuilder::<HeaderUnsecured>::finish_without_security_padded`]
    /// for the encoding-choice rules; the padding is inside the cipher
    /// range and is covered by the MIC.
    ///
    /// `target_len` is the FINAL on-wire PDU length including the MIC.
    ///
    /// # Errors
    ///
    /// Returns [`MacSecurityError::BufferTooShort`] if the IE stream
    /// already exceeds `target_len - MIC_LEN`, if `target_len` is less
    /// than [`MIC_LEN`], or if the buffer is too small to hold the
    /// padded + MIC-trailed PDU. Crypto backend errors bubble through
    /// `MacSecurityError::Crypto`.
    pub fn finish_with_security_padded<C: MacCrypto>(
        mut self,
        target_len: usize,
        crypto: &mut C,
        integrity_key: &[u8; KEY_LEN],
        cipher_key: &[u8; KEY_LEN],
        ctx: &SecurityContext,
    ) -> Result<&'a [u8], MacSecurityError<C::Error>> {
        if target_len < MIC_LEN {
            return Err(MacSecurityError::BufferTooShort);
        }
        let ie_target = target_len - MIC_LEN;
        if ie_target < self.pos {
            return Err(MacSecurityError::BufferTooShort);
        }
        let gap = ie_target - self.pos;
        self.pos = write_padding(self.buf, self.pos, gap)?;
        self.finish_with_security(crypto, integrity_key, cipher_key, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beacon_buffer() -> [u8; 8] {
        // MAC header type byte: version=00, sec=00, type=0001 (Beacon) = 0x01
        // Beacon header: net 0x123456, tx 0xAABBCCDD
        [
            0x01, 0x12, 0x34, 0x56, // network id
            0xAA, 0xBB, 0xCC, 0xDD, // transmitter
        ]
    }

    #[test]
    fn parse_beacon() {
        let buf = beacon_buffer();
        let msg = Message::parse_unverified(&buf[..]).unwrap();
        assert_eq!(
            msg.head.mac_header_type(),
            constants::mac_header_type::BEACON
        );
        match msg.common {
            MacCommonHeader::Beacon(b) => {
                assert_eq!(b.network_id(), 0x123456);
                assert_eq!(b.transmitter_address(), 0xAABBCCDD);
            }
            _ => panic!("expected beacon"),
        }
        assert!(msg.tail.is_empty());
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn parse_version_mismatch() {
        let buf = [0b01_00_0001]; // version=01
        assert!(Message::parse_unverified(&buf[..]).is_err());
    }

    #[test]
    fn parse_escape_header_type() {
        // type=0xF (Escape) is not yet supported -> error
        let buf = [0x0F, 0, 0, 0, 0, 0, 0, 0];
        assert!(Message::parse_unverified(&buf[..]).is_err());
    }

    #[test]
    fn builder_writes_beacon() {
        // Byte-level check: pushing a Beacon header through the typed
        // builder produces the same bytes as a hand-constructed Beacon
        // common header.
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let b = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap();
        assert_eq!(b.finish_without_security(), &beacon_buffer());
    }

    #[test]
    fn builder_push_beacon_round_trips_typed_args() {
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let b = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap();
        let written = b.finish_without_security();

        let msg = Message::parse_unverified(written).unwrap();
        assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::NotUsed));
        match msg.common {
            MacCommonHeader::Beacon(beacon) => {
                assert_eq!(beacon.network_id_typed().unwrap(), net);
                assert_eq!(beacon.transmitter().unwrap(), tx);
            }
            _ => panic!("expected beacon"),
        }
    }

    #[test]
    fn builder_push_unicast() {
        let mut buf = [0; 64];
        let seq = SequenceNumber::try_from_u16(0xABC).unwrap();
        let rx = LongRdId::try_from_u32(0x11223344).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let b = MacPduBuilder::new(&mut buf)
            .push_unicast(NotUsed, true, seq, rx, tx)
            .unwrap();
        let written = b.finish_without_security();
        let msg = Message::parse_unverified(written).unwrap();
        assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::NotUsed));
        match msg.common {
            MacCommonHeader::Unicast(u) => {
                assert!(u.reset());
                assert_eq!(u.sequence_number(), seq);
                assert_eq!(u.receiver().unwrap(), rx);
                assert_eq!(u.transmitter().unwrap(), tx);
            }
            _ => panic!("expected unicast"),
        }
    }

    #[test]
    fn builder_appends_ie() {
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let payload: &[u8] = &[1, 2, 3, 4, 5];
        let ie =
            InformationElement::new_6bit_with_length(IEType6bit::ClusterBeacon, payload).unwrap();
        let b = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .push_ie(&ie)
            .unwrap();
        let written = b.finish_without_security();
        let msg = Message::parse_unverified(written).unwrap();
        assert_eq!(msg.tail.len(), 1 + 1 + 5);
        let mut iter = msg.tail_items();
        let parsed_ie = iter.next().unwrap().unwrap();
        assert_eq!(
            parsed_ie.ie_number(),
            crate::mac::ie::AnyIeType::Type6bit(IEType6bit::ClusterBeacon)
        );
        assert_eq!(parsed_ie.payload(), payload);
    }

    #[test]
    fn builder_finish_padded_zero_gap_is_a_no_op() {
        // No bytes to pad: returned slice has the same length as the
        // bare finish_without_security would.
        let mut buf_a = [0; 32];
        let mut buf_b = [0; 32];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let bare_len = MacPduBuilder::new(&mut buf_a)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security()
            .len();
        let padded_len = MacPduBuilder::new(&mut buf_b)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security_padded(bare_len)
            .unwrap()
            .len();
        assert_eq!(padded_len, bare_len);
    }

    #[test]
    fn builder_finish_padded_one_byte_uses_short_padding() {
        let mut buf = [0; 32];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let bare_len = 8; // 1 header byte + 7 beacon header bytes
        let pdu = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security_padded(bare_len + 1)
            .unwrap();
        assert_eq!(pdu.len(), bare_len + 1);
        // 5-bit Short Padding, len=0 head = 0xC0
        assert_eq!(pdu[bare_len], 0xC0);
    }

    #[test]
    fn builder_finish_padded_two_bytes_uses_short_padding_with_payload() {
        let mut buf = [0; 32];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let bare_len = 8;
        let pdu = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security_padded(bare_len + 2)
            .unwrap();
        assert_eq!(pdu.len(), bare_len + 2);
        // 5-bit Short Padding, len=1 head = 0xE0, plus a zero payload byte
        assert_eq!(pdu[bare_len], 0xE0);
        assert_eq!(pdu[bare_len + 1], 0x00);
    }

    #[test]
    fn builder_finish_padded_large_gap_uses_8bit_length_padding() {
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let bare_len = 8;
        let target = bare_len + 10;
        let pdu = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security_padded(target)
            .unwrap();
        assert_eq!(pdu.len(), target);
        // Clause 6.4.3.8: MAC_Ext 01 + type 000000 (head 0x40), length
        // = number of padding octets, then zeroed padding payload.
        assert_eq!(pdu[bare_len], 0x40);
        assert_eq!(pdu[bare_len + 1], 8);
        for &b in &pdu[bare_len + 2..target] {
            assert_eq!(b, 0x00);
        }
        // Parses as a single Padding IE covering the rest of the PDU.
        let msg = Message::parse_unverified(pdu).unwrap();
        let ie = msg.tail_items().next().unwrap().unwrap();
        assert_eq!(
            ie.ie_number(),
            crate::mac::ie::AnyIeType::Type6bit(IEType6bit::Padding)
        );
        assert_eq!(ie.payload().len(), 8);
    }

    #[test]
    fn builder_finish_padded_gap_three_boundary() {
        // gap = 3 is the smallest gap on the 8-bit-length branch:
        // 0x40, length 1, one zero byte.
        let mut buf = [0; 16];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let bare_len = 8;
        let pdu = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .finish_without_security_padded(bare_len + 3)
            .unwrap();
        assert_eq!(&pdu[bare_len..], &[0x40, 1, 0x00]);
    }

    #[test]
    fn write_padding_chains_when_gap_exceeds_one_ie() {
        // 8-bit length caps one Padding IE at 257 bytes; a 258-byte gap
        // must chain (257-byte IE + 1-byte short Padding IE).
        let mut buf = [0xFFu8; 300];
        let end = write_padding(&mut buf, 0, 258).unwrap();
        assert_eq!(end, 258);
        assert_eq!(buf[0], 0x40);
        assert_eq!(buf[1], 255);
        assert!(buf[2..257].iter().all(|&b| b == 0));
        assert_eq!(buf[257], 0xC0);
    }

    #[test]
    fn builder_finish_padded_rejects_smaller_than_current() {
        let mut buf = [0; 32];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        // Beacon header is 8 bytes already; ask for 4.
        assert!(
            MacPduBuilder::new(&mut buf)
                .push_beacon(NotUsed, net, tx)
                .unwrap()
                .finish_without_security_padded(4)
                .is_err()
        );
    }

    #[test]
    fn builder_push_short_body_writes_short_ie() {
        // Push a 5-bit Short IE body via push_short_body and confirm it
        // parses back as the same IE type with the expected payload.
        use crate::mac::messages::RdCapabilityShortParts;
        use crate::types::{HarqFeedbackDelay, IEType5bitLen1};
        let body = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(3).unwrap(),
            dwa: false,
        };
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let written = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .push_short_body(&body)
            .unwrap()
            .finish_without_security()
            .len();
        let msg = Message::parse_unverified(&buf[..written]).unwrap();
        let ie = msg.tail_items().next().unwrap().unwrap();
        assert_eq!(
            ie.ie_number(),
            crate::mac::ie::AnyIeType::Type5bit(ShortIeType::Len1(
                IEType5bitLen1::RdCapabilityShort
            ))
        );
        // The 1-byte payload round-trips through parse.
        let parsed = RdCapabilityShortParts::parse(ie.payload()).unwrap();
        assert!(parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 3);
        assert!(!parsed.dwa);
    }

    #[test]
    fn builder_push_body_works_on_mu_bearing_body() {
        // ResourceAllocationParts carries Mu internally so it fits the
        // parameter-less MessageBody trait. Confirm push_body works and
        // the result round-trips through parse.
        use crate::mac::messages::{ResourceAllocationKind, ResourceAllocationParts};
        use crate::types::Mu;
        let body = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::ReleaseAll,
        };
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let written = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .push_body(&body)
            .unwrap()
            .finish_without_security()
            .len();
        let msg = Message::parse_unverified(&buf[..written]).unwrap();
        let ie = msg.tail_items().next().unwrap().unwrap();
        assert_eq!(
            ie.ie_number(),
            crate::mac::ie::AnyIeType::Type6bit(IEType6bit::ResourceAllocation)
        );
        let parsed = ResourceAllocationParts::parse(ie.payload(), Mu::M1).unwrap();
        assert!(matches!(parsed.kind, ResourceAllocationKind::ReleaseAll));
    }

    #[test]
    fn builder_push_body_works_on_lifetime_bound_body() {
        // GroupAssignmentParts<'a> borrows a slice; confirm push_body
        // still works with a generic argument that carries a lifetime.
        use crate::mac::messages::{GroupAssignmentParts, GroupResourceTagEntry};
        use crate::types::{GroupId, ResourceTag};
        let tags = [GroupResourceTagEntry::new(
            false,
            ResourceTag::try_from_u8(0x20).unwrap(),
        )];
        let body = GroupAssignmentParts {
            single: true,
            group_id: GroupId::try_from_u8(0x01).unwrap(),
            tags: &tags,
        };
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let written = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .push_body(&body)
            .unwrap()
            .finish_without_security()
            .len();
        let msg = Message::parse_unverified(&buf[..written]).unwrap();
        let ie = msg.tail_items().next().unwrap().unwrap();
        assert_eq!(
            ie.ie_number(),
            crate::mac::ie::AnyIeType::Type6bit(IEType6bit::GroupAssignment)
        );
    }

    #[test]
    fn builder_push_body_writes_ie_directly() {
        // Round-trip a body via push_body and confirm it parses as the
        // same IE type with the same payload bytes.
        use crate::mac::messages::JoiningInformationParts;
        use crate::types::EndpointProtocol;
        let mut buf = [0; 64];
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        let endpoints =
            heapless::Vec::from_slice(&[EndpointProtocol(0x1111), EndpointProtocol(0x2222)])
                .unwrap();
        let body = JoiningInformationParts { endpoints };
        let written = MacPduBuilder::new(&mut buf)
            .push_beacon(NotUsed, net, tx)
            .unwrap()
            .push_body(&body)
            .unwrap()
            .finish_without_security()
            .len();
        let msg = Message::parse_unverified(&buf[..written]).unwrap();
        let mut iter = msg.tail_items();
        let ie = iter.next().unwrap().unwrap();
        assert_eq!(
            ie.ie_number(),
            crate::mac::ie::AnyIeType::Type6bit(IEType6bit::JoiningInformation)
        );
        // The body parses back identically.
        let parsed = JoiningInformationParts::parse(ie.payload()).unwrap();
        assert_eq!(parsed.endpoints.len(), 2);
        assert_eq!(parsed.endpoints[0].0, 0x1111);
        assert_eq!(parsed.endpoints[1].0, 0x2222);
    }

    #[test]
    fn builder_buffer_full_on_header() {
        let mut buf = [0; 5]; // too small for beacon (8 bytes)
        let net = NetworkId24::try_from_u32(0x123456).unwrap();
        let tx = LongRdId::try_from_u32(0xAABBCCDD).unwrap();
        assert!(
            MacPduBuilder::new(&mut buf)
                .push_beacon(NotUsed, net, tx)
                .is_err()
        );
    }

    // ---- Message::parse_unverified MIC trailer handling -------------------------------

    fn make_unicast_pdu_unsecured(payload_after_common: &[u8]) -> [u8; 64] {
        let mut buf = [0; 64];
        let pdu_len = {
            let b = MacPduBuilder::new(&mut buf)
                .push_unicast(
                    NotUsed,
                    false,
                    SequenceNumber::try_from_u16(1).unwrap(),
                    LongRdId::try_from_u32(0xCAFEBABE).unwrap(),
                    LongRdId::try_from_u32(0xDEADBEEF).unwrap(),
                )
                .unwrap();
            b.finish_without_security().len()
        };
        buf[pdu_len..pdu_len + payload_after_common.len()].copy_from_slice(payload_after_common);
        buf
    }

    /// Patch the MAC header byte to advertise a different security
    /// field after the body was built. Used to test
    /// `Message::parse_unverified`'s MIC-trailer split semantics without going
    /// through the secured builder.
    fn make_unicast_synthetic_secured_pdu(
        security: MacSecurity,
        payload_after_common: &[u8],
    ) -> [u8; 64] {
        let mut buf = make_unicast_pdu_unsecured(payload_after_common);
        buf[0] = (buf[0] & !0x30) | ((security.as_u8() & 0x03) << 4);
        buf
    }

    #[test]
    fn parse_without_security_does_not_extract_mic() {
        // 11-byte header + 3-byte synthetic tail, no security.
        let buf = make_unicast_synthetic_secured_pdu(MacSecurity::NotUsed, &[0xAA, 0xBB, 0xCC]);
        let msg = Message::parse_unverified(&buf[..14]).unwrap();
        assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::NotUsed));
        assert!(msg.mic.is_none());
        assert_eq!(msg.tail, &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn parse_with_security_used_no_ie_splits_5_byte_mic() {
        // 11-byte header + 3-byte tail + 5-byte synthetic MIC = 19 bytes.
        let buf = make_unicast_synthetic_secured_pdu(
            MacSecurity::UsedNoIe,
            &[0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33, 0x44, 0x55],
        );
        let msg = Message::parse_unverified(&buf[..19]).unwrap();
        assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::UsedNoIe));
        // tail excludes the last 5 bytes
        assert_eq!(msg.tail, &[0xAA, 0xBB, 0xCC]);
        // mic is exactly the last 5 bytes
        assert_eq!(msg.mic.unwrap(), &[0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn parse_with_security_used_with_ie_splits_5_byte_mic() {
        let buf = make_unicast_synthetic_secured_pdu(
            MacSecurity::UsedWithIe,
            &[0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33, 0x44, 0x55],
        );
        let msg = Message::parse_unverified(&buf[..19]).unwrap();
        assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::UsedWithIe));
        assert_eq!(msg.tail, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(msg.mic.unwrap(), &[0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn parse_with_security_rejects_buffer_too_short_for_mic() {
        // Header (1) + Unicast common (10) = 11 bytes, no room for the 5-byte MIC.
        let buf = make_unicast_synthetic_secured_pdu(MacSecurity::UsedNoIe, &[]);
        assert!(Message::parse_unverified(&buf[..11]).is_err());
    }

    #[test]
    fn parse_with_security_tail_can_be_empty() {
        // 11 (header) + 5 (MIC) = 16 bytes, zero IE-stream tail.
        let buf = make_unicast_synthetic_secured_pdu(
            MacSecurity::UsedNoIe,
            &[0x11, 0x22, 0x33, 0x44, 0x55],
        );
        let msg = Message::parse_unverified(&buf[..16]).unwrap();
        assert!(msg.tail.is_empty());
        assert_eq!(msg.mic.unwrap(), &[0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    // ---- End-to-end secured-builder + parse round-trips -------------
    //
    // Bundled into a single feature-gated submodule so the whole secured
    // flow (helper + imports + tests) lives behind one `#[cfg]`.
    #[cfg(feature = "software-crypto")]
    mod secured {
        use super::*;

        use crate::mac::messages::MacSecurityInfoParts;
        use crate::security::{NoCrypto, SecurityContext, SoftwareCrypto};

        #[test]
        fn secured_unicast_used_no_ie_round_trip() {
            let mut crypto = SoftwareCrypto;
            let int_key = [0x11u8; 16];
            let cipher_key = [0x22u8; 16];
            let psn = SequenceNumber::try_from_u16(7).unwrap();
            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(0xCAFEBABE).unwrap(),
                rx: LongRdId::try_from_u32(0xDEADBEEF).unwrap(),
                hpc: 0x1234,
            };

            // Build a secured PDU (UsedNoIe).
            let mut tx_buf = [0; 128];
            let payload: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD];
            let payload_ie =
                InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, payload)
                    .unwrap();
            let ciphertext_len = MacPduBuilder::new(&mut tx_buf)
                .push_unicast(UsedNoIe, false, psn, ctx.rx, ctx.tx)
                .unwrap()
                .push_ie(&payload_ie)
                .unwrap()
                .finish_with_security(&mut crypto, &int_key, &cipher_key, &ctx)
                .unwrap()
                .len();

            // Decrypt + verify + parse in one call on the receiver side.
            let mut rx_buf = tx_buf;
            let msg = Message::parse(
                &mut rx_buf[..ciphertext_len],
                &mut crypto,
                &int_key,
                &cipher_key,
                &ctx,
            )
            .unwrap()
            .secured()
            .expect("PDU was built secured");
            assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::UsedNoIe));
            // First IE in the decrypted tail is the user-plane payload.
            let mut iter = msg.tail_items();
            let first = iter.next().unwrap().unwrap();
            assert_eq!(
                first.ie_number(),
                crate::mac::ie::AnyIeType::Type6bit(IEType6bit::UserPlaneDataFlow1)
            );
            assert_eq!(first.payload(), payload);
        }

        #[test]
        fn secured_finish_padded_round_trip() {
            // Build a secured PDU and pad it to exactly TARGET bytes
            // including the 5-byte MIC. Confirm the on-wire length is
            // TARGET and the decrypted IE stream still parses.
            const TARGET: usize = 40;
            let mut crypto = SoftwareCrypto;
            let int_key = [0x77u8; 16];
            let cipher_key = [0x88u8; 16];
            let psn = SequenceNumber::try_from_u16(1).unwrap();
            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(0xAABBCCDD).unwrap(),
                rx: LongRdId::try_from_u32(0x11223344).unwrap(),
                hpc: 0x1000,
            };
            let mut tx_buf = [0; 128];
            let payload: &[u8] = &[0xDE, 0xAD];
            let ie =
                InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, payload)
                    .unwrap();
            let len = MacPduBuilder::new(&mut tx_buf)
                .push_unicast(UsedNoIe, false, psn, ctx.rx, ctx.tx)
                .unwrap()
                .push_ie(&ie)
                .unwrap()
                .finish_with_security_padded(TARGET, &mut crypto, &int_key, &cipher_key, &ctx)
                .unwrap()
                .len();
            assert_eq!(len, TARGET);

            // RX side: decrypt + verify; the original IE is still there
            // alongside whatever Padding IE was appended.
            let mut rx_buf = tx_buf;
            let msg = Message::parse(&mut rx_buf[..len], &mut crypto, &int_key, &cipher_key, &ctx)
                .unwrap()
                .secured()
                .expect("PDU was built secured");
            let mut iter = msg.tail_items();
            let first = iter.next().unwrap().unwrap();
            assert_eq!(
                first.ie_number(),
                crate::mac::ie::AnyIeType::Type6bit(IEType6bit::UserPlaneDataFlow1)
            );
            assert_eq!(first.payload(), payload);
        }

        #[test]
        fn secured_beacon_used_with_ie_round_trip() {
            let mut crypto = SoftwareCrypto;
            let int_key = [0x33u8; 16];
            let cipher_key = [0x44u8; 16];
            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(0x01020304).unwrap(),
                rx: LongRdId::BROADCAST,
                hpc: 0xABCD_1234,
            };

            let mut tx_buf = [0; 128];
            let net = NetworkId24::try_from_u32(0x123456).unwrap();
            let payload_ie = InformationElement::new_6bit_with_length(
                IEType6bit::ClusterBeacon,
                &[1, 2, 3, 4, 5],
            )
            .unwrap();
            let ciphertext_len = MacPduBuilder::new(&mut tx_buf)
                .push_beacon(UsedWithIe, net, ctx.tx)
                .unwrap()
                .push_mac_security_info(
                    SecurityVersion::Mode1,
                    KeyIndex::try_from_u8(0).unwrap(),
                    SecurityIvType::OneTimeHpc,
                )
                .unwrap()
                .push_ie(&payload_ie)
                .unwrap()
                .finish_with_security(&mut crypto, &int_key, &cipher_key, &ctx)
                .unwrap()
                .len();

            // Before decrypting: the Security Info IE is plaintext, so
            // the receiver can peek the HPC for IV synchronization. The
            // on-air HPC must equal ctx.hpc (patched at finish time).
            let peeked = Message::peek_security_info(&tx_buf[..ciphertext_len])
                .expect("security info IE is plaintext");
            assert_eq!(peeked.hpc, ctx.hpc);
            assert_eq!(peeked.version, SecurityVersion::Mode1);

            let mut rx_buf = tx_buf;
            let msg = Message::parse(
                &mut rx_buf[..ciphertext_len],
                &mut crypto,
                &int_key,
                &cipher_key,
                &ctx,
            )
            .unwrap()
            .secured()
            .expect("PDU was built secured");
            assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::UsedWithIe));
            // First IE should be the MAC Security Info IE we pushed,
            // carrying the patched HPC.
            let mut iter = msg.tail_items();
            let first = iter.next().unwrap().unwrap();
            assert_eq!(
                first.ie_number(),
                crate::mac::ie::AnyIeType::Type6bit(IEType6bit::MacSecurityInfo)
            );
            let on_air = MacSecurityInfoParts::parse(first.payload()).unwrap();
            assert_eq!(on_air.hpc, ctx.hpc);
            let second = iter.next().unwrap().unwrap();
            assert_eq!(
                second.ie_number(),
                crate::mac::ie::AnyIeType::Type6bit(IEType6bit::ClusterBeacon)
            );
        }

        #[test]
        fn parse_returns_unsecured_for_not_used() {
            // A NotUsed PDU flows through the same entry point but is
            // explicitly marked Unsecured.
            let mut crypto = SoftwareCrypto;
            let keys = [0; 16];
            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(1).unwrap(),
                rx: LongRdId::try_from_u32(2).unwrap(),
                hpc: 0,
            };
            let mut buf = [0; 64];
            let len = MacPduBuilder::new(&mut buf)
                .push_beacon(
                    NotUsed,
                    NetworkId24::try_from_u32(0x123456).unwrap(),
                    ctx.tx,
                )
                .unwrap()
                .finish_without_security()
                .len();
            let parsed = Message::parse(&mut buf[..len], &mut crypto, &keys, &keys, &ctx).unwrap();
            assert!(!parsed.is_secured());
            let msg = parsed.into_message();
            assert_eq!(msg.head.mac_security_typed(), Some(MacSecurity::NotUsed));
        }

        #[test]
        fn no_crypto_parses_unsecured_and_rejects_secured() {
            use crate::security::MacSecurityError;

            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(0x11111111).unwrap(),
                rx: LongRdId::try_from_u32(0x22222222).unwrap(),
                hpc: 0,
            };
            let keys = [0; 16];

            // Unsecured PDU: parses through the single entry point.
            let mut crypto = NoCrypto;
            let mut buf = [0; 64];
            let len = MacPduBuilder::new(&mut buf)
                .push_beacon(
                    NotUsed,
                    NetworkId24::try_from_u32(0x123456).unwrap(),
                    ctx.tx,
                )
                .unwrap()
                .finish_without_security()
                .len();
            let parsed = Message::parse(&mut buf[..len], &mut crypto, &keys, &keys, &ctx).unwrap();
            assert!(!parsed.is_secured());

            // Secured PDU: rejected with a Crypto error, not parsed.
            let mut soft = SoftwareCrypto;
            let mut tx_buf = [0; 64];
            let psn = SequenceNumber::try_from_u16(3).unwrap();
            let len = MacPduBuilder::new(&mut tx_buf)
                .push_unicast(UsedNoIe, false, psn, ctx.rx, ctx.tx)
                .unwrap()
                .finish_with_security(&mut soft, &keys, &keys, &ctx)
                .unwrap()
                .len();
            let err =
                Message::parse(&mut tx_buf[..len], &mut crypto, &keys, &keys, &ctx).unwrap_err();
            assert!(matches!(err, MacSecurityError::Crypto(_)));
        }

        #[test]
        fn parse_rejects_tampered_mic() {
            let mut crypto = SoftwareCrypto;
            let int_key = [0x55u8; 16];
            let cipher_key = [0x66u8; 16];
            let ctx = SecurityContext {
                tx: LongRdId::try_from_u32(0x11111111).unwrap(),
                rx: LongRdId::try_from_u32(0x22222222).unwrap(),
                hpc: 1,
            };

            let mut tx_buf = [0; 64];
            let len = {
                let b = MacPduBuilder::new(&mut tx_buf)
                    .push_unicast(
                        UsedNoIe,
                        false,
                        const { SequenceNumber::try_from_u16(0).unwrap() },
                        ctx.rx,
                        ctx.tx,
                    )
                    .unwrap();
                b.finish_with_security(&mut crypto, &int_key, &cipher_key, &ctx)
                    .unwrap()
                    .len()
            };

            // Tamper a ciphertext byte (somewhere in the IE-stream / MIC region).
            let mut rx_buf = tx_buf;
            rx_buf[len - 1] ^= 0x01;
            let err = Message::parse(&mut rx_buf[..len], &mut crypto, &int_key, &cipher_key, &ctx)
                .unwrap_err();
            assert!(matches!(err, MacSecurityError::BadMic));
        }
    } // mod secured
}
