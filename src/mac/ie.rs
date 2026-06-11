//! Information Element (IE) types, parser, and serializer.
//!
//! ETSI TS 103 636-4, clause 6.3.4 (and the tables therein:
//! 6.3.4-1 for MAC_Ext, 6.3.4-2 for 6-bit types, 6.3.4-3 and -4 for 5-bit
//! types).

use crate::types::{IEType6bit, MacExt, ShortIeType};
use crate::{BufferFull, ParsingError};

// ---------------------------------------------------------------------------
// AnyIeType
// ---------------------------------------------------------------------------

/// Unifying enum over the IE type registries. Returned by
/// [`InformationElement::ie_number`].
///
/// Reserved / unrecognized code points surface as the `Unknown*`
/// variants so the IE stream parser can keep skipping such IEs by
/// length instead of failing.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyIeType {
    /// Known 6-bit IE type (used with MAC_Ext 00, 01, 10).
    Type6bit(IEType6bit),
    /// Known 5-bit IE type with its length bit (used with MAC_Ext 11).
    Type5bit(ShortIeType),
    /// Reserved / unrecognized 6-bit code point (raw value).
    Unknown6bit(u8),
    /// Reserved / unrecognized Short IE composite (length bit at
    /// bit 5, 5-bit type in the low bits).
    Unknown5bit(u8),
}

impl AnyIeType {
    /// True if this is a padding IE (6-bit or either 5-bit registry).
    #[must_use]
    #[inline]
    pub const fn is_padding(self) -> bool {
        match self {
            AnyIeType::Type6bit(t) => matches!(t, IEType6bit::Padding),
            AnyIeType::Type5bit(t) => t.is_padding(),
            AnyIeType::Unknown6bit(_) | AnyIeType::Unknown5bit(_) => false,
        }
    }
}

// ---------------------------------------------------------------------------
// InformationElement
// ---------------------------------------------------------------------------

/// A single Information Element of the MAC layer.
///
/// ETSI TS 103 636-4, clause 6.3.4. The `head` byte encodes the
/// [`crate::constants::mac_ext`] field in its top 2 bits; the lower 6 bits hold
/// the IE type (6-bit form) or the composite length+value (5-bit form).
///
/// The `payload` is the IE body. For MAC_Ext 11 (Short IE), the length is
/// 0 or 1 byte as encoded in the head itself. For MAC_Ext 01 or 10, an
/// explicit length field follows. For MAC_Ext 00, the length is defined by
/// the IE type (see [`Self::parse`] for what we know).
#[derive(Clone, Copy)]
pub struct InformationElement<'a> {
    /// First byte of the IE. Top 2 bits are the MAC_Ext field, low 6 bits
    /// hold the IE type (or composite length+value in the short-IE case).
    /// Private so the [`Self::payload`] length invariant can't be violated
    /// from outside this module.
    head: u8,
    /// Payload bytes (zero or more). Private for the same reason.
    payload: &'a [u8],
}

impl<'a> InformationElement<'a> {
    /// Raw first byte of the IE (MAC_Ext in top 2 bits, IE type in low 6).
    #[must_use]
    #[inline]
    pub const fn head(&self) -> u8 {
        self.head
    }

    /// IE type from the head byte, unified over the registries.
    /// Reserved / unrecognized code points come back as
    /// [`AnyIeType::Unknown6bit`] / [`AnyIeType::Unknown5bit`].
    #[inline]
    pub fn ie_number(&self) -> AnyIeType {
        let low6 = self.head & 0x3F;
        match MacExt::try_from_u8(self.head >> 6) {
            Some(MacExt::ShortIe) => match ShortIeType::try_from_composite(low6) {
                Some(t) => AnyIeType::Type5bit(t),
                None => AnyIeType::Unknown5bit(low6),
            },
            _ => match IEType6bit::try_from_u8(low6) {
                Some(t) => AnyIeType::Type6bit(t),
                None => AnyIeType::Unknown6bit(low6),
            },
        }
    }

    /// Payload bytes.
    #[must_use]
    #[inline]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// Number of bytes the IE occupies when serialized: head byte + 0, 1,
    /// or 2 length bytes (depending on MAC_Ext) + payload length.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let length_bytes = match MacExt::try_from_u8(self.head >> 6) {
            Some(MacExt::Length8Bit) => 1,
            Some(MacExt::Length16Bit) => 2,
            _ => 0,
        };
        1 + length_bytes + self.payload.len()
    }

    /// Serialize the IE to a writer. Writes head, then explicit length
    /// (if MAC_Ext 01/10), then payload. Uses `write_all` throughout:
    /// a partial write surfaces as an error instead of a silently
    /// truncated IE.
    pub fn serialize<W: embedded_io::Write>(&self, w: &mut W) -> Result<(), W::Error> {
        w.write_all(&[self.head])?;
        match MacExt::try_from_u8(self.head >> 6) {
            Some(MacExt::Length8Bit) => {
                w.write_all(&[self.payload.len() as u8])?;
            }
            Some(MacExt::Length16Bit) => {
                let len = self.payload.len() as u16;
                w.write_all(&len.to_be_bytes())?;
            }
            _ => {}
        }
        w.write_all(self.payload)?;
        Ok(())
    }
}

impl core::fmt::Debug for InformationElement<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InformationElement")
            .field("head", &format_args!("0x{:02x}", self.head))
            .field("ie_number", &self.ie_number())
            .field("payload", &self.payload)
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for InformationElement<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        if self.ie_number().is_padding() && self.payload.iter().all(|b| *b == 0) {
            defmt::write!(
                f,
                "{}, {} bytes of 0x00",
                self.ie_number(),
                self.payload.len()
            );
            return;
        }
        defmt::write!(
            f,
            "{}, payload: {=[u8]:02x}",
            self.ie_number(),
            self.payload
        );
    }
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl<'a> InformationElement<'a> {
    /// Construct a 6-bit IE with an explicit 8-bit or 16-bit length field
    /// (MAC_Ext 01 or 10). The smaller encoding is chosen automatically.
    ///
    /// # Errors
    ///
    /// Returns [`BufferFull`] if the payload is too large to encode
    /// in a 16-bit length field.
    #[inline]
    pub fn new_6bit_with_length(type_: IEType6bit, payload: &'a [u8]) -> Result<Self, BufferFull> {
        if payload.len() > u16::MAX as usize {
            return Err(BufferFull);
        }
        let mac_ext = if payload.len() <= u8::MAX as usize {
            MacExt::Length8Bit
        } else {
            MacExt::Length16Bit
        };
        let head = (mac_ext.as_u8() << 6) | u8::from(type_);
        Ok(Self { head, payload })
    }

    /// Construct a 5-bit "Short IE" (MAC_Ext 11). The payload length
    /// must match the registry the type comes from (0 or 1 byte).
    ///
    /// # Errors
    ///
    /// Returns [`BufferFull`] if the payload length does not match
    /// `type_.len()`.
    #[inline]
    pub fn new_5bit(type_: ShortIeType, payload: &'a [u8]) -> Result<Self, BufferFull> {
        if payload.len() != type_.len() as usize {
            return Err(BufferFull);
        }
        let head = (MacExt::ShortIe.as_u8() << 6) | type_.composite();
        Ok(Self { head, payload })
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

impl<'a> InformationElement<'a> {
    /// Read a single IE from the front of `data`, advancing the slice to
    /// point past the consumed bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] on length underrun, or if the IE type uses
    /// MAC_Ext 00 (no length field) and the type's length is not in our
    /// built-in table.
    pub fn parse(data: &mut &'a [u8]) -> Result<InformationElement<'a>, ParsingError> {
        let head = *data.first().ok_or(ParsingError::Truncated)?;
        *data = &data[1..];

        // `head >> 6` is always in 0..=3 so try_from_u8 always succeeds.
        let mac_ext = MacExt::try_from_u8(head >> 6).expect("masked to 2 bits");
        let len: usize = match mac_ext {
            MacExt::NoLength => {
                let low6 = head & 0x3F;
                match no_length_payload_len(low6) {
                    PayloadLen::Fixed(n) => n,
                    // Padding under MAC_Ext = 00 consumes all remaining
                    // bytes (§6.4.3.8). `data` at this point excludes the
                    // MAC security MIC trailer (split off by
                    // [`crate::mac::pdu::Message::parse_unverified`]), so taking the
                    // full remainder yields the correct payload.
                    PayloadLen::RestOfPdu => data.len(),
                    PayloadLen::Variable => return Err(ParsingError::BadLength),
                }
            }
            MacExt::Length8Bit => {
                let b = *data.first().ok_or(ParsingError::Truncated)?;
                *data = &data[1..];
                b as usize
            }
            MacExt::Length16Bit => {
                if data.len() < 2 {
                    return Err(ParsingError::Truncated);
                }
                let len = u16::from_be_bytes([data[0], data[1]]) as usize;
                *data = &data[2..];
                len
            }
            MacExt::ShortIe => ((head >> 5) & 1) as usize,
        };

        if data.len() < len {
            // An explicit length field pointing past the end of the
            // PDU is a length contradiction; the implied short-IE /
            // fixed lengths just ran out of input.
            return Err(match mac_ext {
                MacExt::Length8Bit | MacExt::Length16Bit => ParsingError::BadLength,
                _ => ParsingError::Truncated,
            });
        }
        let (payload, rest) = data.split_at(len);
        *data = rest;

        Ok(InformationElement { head, payload })
    }

    /// Iterate IEs over `data` lazily. On a parse error, yields one `Err`
    /// and then `None` for subsequent items.
    pub fn parse_stream(
        mut data: &'a [u8],
    ) -> impl Iterator<Item = Result<InformationElement<'a>, ParsingError>> {
        core::iter::from_fn(move || {
            if data.is_empty() {
                return None;
            }
            match Self::parse(&mut data) {
                Ok(ie) => Some(Ok(ie)),
                Err(e) => {
                    data = &[];
                    Some(Err(e))
                }
            }
        })
    }
}

/// How the body length of an IE encoded with `MAC_Ext = 00` (no
/// explicit length field) is determined.
///
/// ETSI TS 103 636-4, clause 6.3.4 plus the per-IE clauses in §6.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PayloadLen {
    /// Fixed body length defined by the IE type itself (e.g. higher-layer
    /// signalling flows have 0-byte bodies).
    Fixed(usize),
    /// Body consumes the rest of the MAC PDU (minus the MAC security
    /// MIC trailer, which [`crate::mac::pdu::Message::parse_unverified`] strips
    /// before invoking the IE-stream parser). Used by Padding
    /// (§6.4.3.8).
    RestOfPdu,
    /// Body length is not fixed by the IE type - it depends on internal
    /// flag bits within the body itself (e.g. Cluster Beacon, RD
    /// Capability). `MAC_Ext = 00` is not a usable encoding for these
    /// IEs; the sender must use `LENGTH_8BIT` or `LENGTH_16BIT`.
    Variable,
}

/// Body length classification for an IE that uses `MAC_Ext = 00`.
///
/// ETSI TS 103 636-4, clause 6.3.4, Table 6.3.4-2.
#[must_use]
pub const fn no_length_payload_len(ie_type_6bit: u8) -> PayloadLen {
    match ie_type_6bit {
        // Padding: rest of PDU (§6.4.3.8)
        0b000000 => PayloadLen::RestOfPdu,
        // Higher layer signalling flows: 0 bytes
        0b000001 | 0b000010 => PayloadLen::Fixed(0),
        // User plane data flows: 0 bytes
        0b000011..=0b000110 => PayloadLen::Fixed(0),
        // Cluster Beacon, RD Capability, etc.: body length depends on
        // internal bits, so MAC_Ext=00 is not usable.
        _ => PayloadLen::Variable,
    }
}

// Convenience: PartialEq against specific IE types for filtering in users'
// code (`if ie.ie_number() == IEType6bit::ClusterBeacon`).
impl PartialEq<IEType6bit> for AnyIeType {
    fn eq(&self, other: &IEType6bit) -> bool {
        matches!(self, AnyIeType::Type6bit(t) if t == other)
    }
}

impl PartialEq<ShortIeType> for AnyIeType {
    fn eq(&self, other: &ShortIeType) -> bool {
        matches!(self, AnyIeType::Type5bit(t) if t == other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IEType5bitLen0, IEType5bitLen1};

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout"
    )]
    fn parse_short_ie_zero_length() {
        // MAC_Ext=11, len=0, type=0b00000 (PADDING with len=0)
        let buf = [0b11_0_00000];
        let mut rest = &buf[..];
        let ie = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(ie.head(), 0b11_0_00000);
        assert_eq!(ie.payload(), &[] as &[u8]);
        assert!(rest.is_empty());
        assert_eq!(
            ie.ie_number(),
            AnyIeType::Type5bit(ShortIeType::Len0(IEType5bitLen0::Padding))
        );
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout"
    )]
    fn parse_short_ie_one_length() {
        // MAC_Ext=11, len=1, type=0b00001 (RADIO_DEVICE_STATUS)
        let buf = [0b11_1_00001, 0xAB];
        let mut rest = &buf[..];
        let ie = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(ie.payload(), &[0xAB]);
        assert!(rest.is_empty());
    }

    #[test]
    fn parse_8bit_length() {
        // MAC_Ext=01, type=0b001001 (CLUSTER_BEACON), length=5
        let buf = [0b01_001001, 5, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mut rest = &buf[..];
        let ie = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(
            ie.ie_number(),
            AnyIeType::Type6bit(IEType6bit::ClusterBeacon)
        );
        assert_eq!(ie.payload(), &[0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn parse_no_length_zero() {
        // MAC_Ext=00, type=0b000011 (USER_PLANE_FLOW_1) -> 0 bytes
        let buf = [0b00_000011];
        let mut rest = &buf[..];
        let ie = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(
            ie.ie_number(),
            AnyIeType::Type6bit(IEType6bit::UserPlaneDataFlow1)
        );
        assert_eq!(ie.payload().len(), 0);
    }

    #[test]
    fn parse_no_length_unknown_errors() {
        // MAC_Ext=00, type=0b001001 (CLUSTER_BEACON) -> body length is
        // variable (depends on internal bits), MAC_Ext=00 is not the
        // right encoding here.
        let buf = [0b00_001001];
        let mut rest = &buf[..];
        assert!(InformationElement::parse(&mut rest).is_err());
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout"
    )]
    fn parse_no_length_padding_consumes_rest_of_pdu() {
        // MAC_Ext=00, type=0b000000 (Padding) followed by 8 arbitrary bytes.
        // Padding swallows all remaining bytes (§6.4.3.8), so parse should
        // yield a single IE with the 8-byte payload and exhaust the buffer.
        let mut buf = [0b00_000000u8, 1, 2, 3, 4, 5, 6, 7, 8];
        buf[0] = 0b00_000000;
        let mut rest = &buf[..];
        let ie = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(ie.ie_number(), AnyIeType::Type6bit(IEType6bit::Padding));
        assert_eq!(ie.payload(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(rest.is_empty());
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout"
    )]
    fn parse_short_payload_truncated() {
        // Short IE claims 1 byte but buffer has 0
        let buf = [0b11_1_00001];
        let mut rest = &buf[..];
        assert!(InformationElement::parse(&mut rest).is_err());
    }

    #[test]
    fn roundtrip_short_ie() {
        let payload: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
        let ie =
            InformationElement::new_6bit_with_length(IEType6bit::ClusterBeacon, payload).unwrap();
        let mut buf = [0; 64];
        ie.serialize(&mut &mut buf[..]).unwrap();
        let written = &buf[..6];
        let mut rest = written;
        let parsed = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(parsed.payload(), payload);
        assert_eq!(parsed.head, ie.head);
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout"
    )]
    fn roundtrip_5bit_ie() {
        // ASSOCIATION_CONTROL (Table 6.3.4-4): MAC_Ext=11, len=1, value=0b00011 = 0b11_1_00011
        let buf = [0b11_1_00011, 0x42];
        let mut rest = &buf[..];
        let parsed = InformationElement::parse(&mut rest).unwrap();
        assert_eq!(
            parsed.ie_number(),
            AnyIeType::Type5bit(ShortIeType::Len1(IEType5bitLen1::AssociationControl))
        );
        assert_eq!(parsed.payload(), &[0x42]);
    }

    #[test]
    fn hophop_golden_vector() {
        // IE stream golden vector from the hophop project:
        // IE 0: head=0x49 (8-bit len, Cluster Beacon), len=5, payload 5 bytes
        // IE 1: head=0x53 (8-bit len, Random Access Resource), len=7, payload 7 bytes
        // IE 2: head=0x40 (8-bit len, Padding), len=24, payload 24 zero bytes
        let data: [u8; 42] = [
            73, 5, 176, 16, 6, 0, 13, 83, 7, 8, 12, 138, 160, 215, 2, 100, 64, 24, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut iter = InformationElement::parse_stream(&data);
        let ie0 = iter.next().unwrap().unwrap();
        assert_eq!(
            ie0.ie_number(),
            AnyIeType::Type6bit(IEType6bit::ClusterBeacon)
        );
        assert_eq!(ie0.payload(), &[176, 16, 6, 0, 13]);

        let ie1 = iter.next().unwrap().unwrap();
        assert_eq!(
            ie1.ie_number(),
            AnyIeType::Type6bit(IEType6bit::RandomAccessResource)
        );
        assert_eq!(ie1.payload(), &[8, 12, 138, 160, 215, 2, 100]);

        let ie2 = iter.next().unwrap().unwrap();
        assert_eq!(ie2.head, 64);
        assert_eq!(ie2.payload().len(), 24);
        assert!(ie2.payload().iter().all(|b| *b == 0));

        assert!(iter.next().is_none());
    }

    #[test]
    fn parse_stream_yields_err_then_none_on_malformed() {
        // First IE: 0x40 head (8-bit length) + length=10 but only 3 payload bytes follow.
        let data = [0x40u8, 10, 0xAA, 0xBB, 0xCC];
        let mut iter = InformationElement::parse_stream(&data);
        let first = iter.next().expect("expected an item");
        assert!(first.is_err(), "first item should be the parse error");
        assert!(
            iter.next().is_none(),
            "iterator must stop after surfacing the error",
        );
        assert!(iter.next().is_none(), "further calls keep yielding None");
    }
}
