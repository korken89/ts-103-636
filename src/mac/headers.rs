//! MAC header type and common headers.
//!
//! ETSI TS 103 636-4, clauses 6.3.2 and 6.3.3.
//!
//! All field layouts are big-endian (MSB-first within bytes), matching the
//! over-the-air order in the spec.

use core::fmt;

use crate::types::{LongRdId, MacSecurity, NetworkId24, SequenceNumber};
use crate::{ExcessiveBitsSet, constants};

// ---------------------------------------------------------------------------
// MacHeaderType - 1-byte MAC header type
// ETSI TS 103 636-4, clause 6.3.2, Figure 6.3.2-1, Tables 6.3.2-1 and 6.3.2-2
// ---------------------------------------------------------------------------

/// Thin wrapper over the 1-byte MAC header type.
///
/// Layout (Figure 6.3.2-1):
/// ```text
///    0   1   2   3   4   5   6   7
/// +-------+-------+---+---+---+---+
/// |Version|MAC Sec| MAC Header Type |
/// +-------+-------+---------------+
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MacHeaderType(pub u8);

impl MacHeaderType {
    /// Construct from individual fields. Version is fixed to 0 (the only
    /// value defined by §6.3.2).
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if `mac_header_type` does not fit in
    /// 4 bits.
    #[inline]
    pub const fn new(
        mac_security: MacSecurity,
        mac_header_type: u8,
    ) -> Result<Self, ExcessiveBitsSet> {
        if mac_header_type & !0x0F != 0 {
            return Err(ExcessiveBitsSet);
        }
        let security = mac_security as u8;
        Ok(Self(
            (constants::MAC_VERSION << 6) | (security << 4) | mac_header_type,
        ))
    }

    /// 2-bit Version field. Only `0` is defined.
    #[must_use]
    #[inline]
    pub const fn version(self) -> u8 {
        self.0 >> 6
    }

    /// 2-bit MAC Security field (Table 6.3.2-1).
    #[must_use]
    #[inline]
    pub const fn mac_security(self) -> u8 {
        (self.0 >> 4) & 0x03
    }

    /// Typed view of the 2-bit MAC Security field. Returns `None` if the
    /// reserved value 0b11 is in the header.
    #[must_use]
    #[inline]
    pub const fn mac_security_typed(self) -> Option<MacSecurity> {
        MacSecurity::try_from_u8(self.mac_security())
    }

    /// 4-bit MAC Header Type field (Table 6.3.2-2).
    #[must_use]
    #[inline]
    pub const fn mac_header_type(self) -> u8 {
        self.0 & 0x0F
    }
}

impl fmt::Debug for MacHeaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MacHeaderType")
            .field("version", &self.version())
            .field("mac_security", &self.mac_security())
            .field("mac_header_type", &self.mac_header_type())
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for MacHeaderType {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "MacHeaderType {{ v: {=u8}, sec: {=u8}, type: {=u8} }}",
            self.version(),
            self.mac_security(),
            self.mac_header_type()
        );
    }
}

// ---------------------------------------------------------------------------
// DataMacPdu - 2-byte header
// ETSI TS 103 636-4, clause 6.3.3.1, Figure 6.3.3.1-1
// ---------------------------------------------------------------------------

/// 2-byte DATA MAC PDU common header.
///
/// Layout (Figure 6.3.3.1-1):
/// ```text
///    0   1   2   3   4   5   6   7     0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+ +---+---+---+---+---+---+---+---+
/// |  Spare  |Reset| Sequence Number | |         Sequence Number       |
/// +---------+-----+-----------------+ +-------------------------------+
/// |              Spare                |
/// +-----------------------------------+
/// ```
#[derive(Clone, Copy)]
pub struct DataMacPdu<'a>(pub &'a [u8; 2]);

impl DataMacPdu<'_> {
    /// `Reset` bit (clears MAC sequence-number state when set).
    #[must_use]
    #[inline]
    pub const fn reset(self) -> bool {
        ((self.0[0] >> 4) & 1) != 0
    }

    /// 12-bit MAC sequence number.
    #[must_use]
    #[inline]
    pub const fn sequence_number(self) -> SequenceNumber {
        let raw = (((self.0[0] as u16) & 0x0F) << 8) | (self.0[1] as u16);
        // The 0x0F mask above guarantees the 12-bit fit, so the
        // SequenceNumber invariant always holds.
        match SequenceNumber::new(raw) {
            Some(s) => s,
            None => unreachable!(),
        }
    }

    /// Build a 2-byte Data MAC PDU common header (Figure 6.3.3.1-1).
    /// Spare bits are written as zero.
    #[must_use]
    #[inline]
    pub const fn new_owned(reset: bool, sequence_number: SequenceNumber) -> [u8; 2] {
        let seq = sequence_number.as_u16();
        let reset_bit: u8 = if reset { 0x10 } else { 0x00 };
        [reset_bit | ((seq >> 8) as u8 & 0x0F), (seq & 0xFF) as u8]
    }
}

impl fmt::Debug for DataMacPdu<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataMacPdu")
            .field("reset", &self.reset())
            .field("sequence_number", &self.sequence_number())
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for DataMacPdu<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "DataMacPdu {{ reset: {=bool}, seq: {=u16} }}",
            self.reset(),
            self.sequence_number().as_u16()
        );
    }
}

// ---------------------------------------------------------------------------
// Beacon - 7-byte header
// ETSI TS 103 636-4, clause 6.3.3.2, Figure 6.3.3.2-1
// ---------------------------------------------------------------------------

/// 7-byte Beacon common header.
///
/// Layout (Figure 6.3.3.2-1):
/// ```text
///    0   1   2   3   4   5   6
/// +-------------------------------+
/// |       Network ID (24)         |
/// +-------------------------------+
/// |     Transmitter Address (32)  |
/// +-------------------------------+
/// ```
#[derive(Clone, Copy)]
pub struct Beacon<'a>(pub &'a [u8; 7]);

impl Beacon<'_> {
    /// 24-bit Network ID, right-aligned in the returned u32 (the high 8 bits
    /// are zero, matching the convention used by [`NetworkId24`]).
    #[must_use]
    #[inline]
    pub const fn network_id(self) -> u32 {
        u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]])
    }

    /// Typed view of the network ID. Returns `None` if the 24-bit value is zero.
    #[must_use]
    #[inline]
    pub fn network_id_typed(self) -> Option<NetworkId24> {
        NetworkId24::new(self.network_id())
    }

    /// 32-bit Transmitter Address (Long RD ID, clause 4.2.3.2).
    #[must_use]
    #[inline]
    pub fn transmitter_address(self) -> u32 {
        u32::from_be_bytes(*<&[u8; 4]>::try_from(&self.0[3..7]).expect("slice len is 4"))
    }

    /// Transmitter Long RD ID (typed). `None` if the raw value is reserved (0).
    #[must_use]
    #[inline]
    pub fn transmitter(self) -> Option<LongRdId> {
        LongRdId::new(self.transmitter_address())
    }

    /// Build a 7-byte Beacon common header (Figure 6.3.3.2-1).
    #[must_use]
    #[inline]
    pub fn new_owned(network_id: NetworkId24, transmitter: LongRdId) -> [u8; 7] {
        let nid = network_id.as_u32().to_be_bytes(); // [0, n_hi, n_mid, n_lo]
        let tx = transmitter.as_u32().to_be_bytes();
        [nid[1], nid[2], nid[3], tx[0], tx[1], tx[2], tx[3]]
    }
}

impl fmt::Debug for Beacon<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Beacon")
            .field("network_id", &format_args!("0x{:06x}", self.network_id()))
            .field(
                "transmitter_address",
                &format_args!("0x{:08x}", self.transmitter_address()),
            )
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Beacon<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "Beacon {{ net: 0x{=u32:06x}, tx: 0x{=u32:08x} }}",
            self.network_id(),
            self.transmitter_address()
        );
    }
}

// ---------------------------------------------------------------------------
// Unicast - 10-byte header
// ETSI TS 103 636-4, clause 6.3.3.3, Figure 6.3.3.3-1
// ---------------------------------------------------------------------------

/// 10-byte Unicast common header.
///
/// Layout (Figure 6.3.3.3-1):
/// ```text
///    0   1   2   3   4   5   6   7     0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+ +-------------------------------+
/// |  Spare  |Reset| Sequence Number | |    Receiver Address (32)     |
/// +---------+-----+-----------------+ +-------------------------------+
/// |         Sequence Number          |    Transmitter Address (32)  |
/// +-----------------------------------+-------------------------------+
/// ```
#[derive(Clone, Copy)]
pub struct Unicast<'a>(pub &'a [u8; 10]);

impl Unicast<'_> {
    /// `Reset` bit (clears MAC sequence-number state when set).
    #[must_use]
    #[inline]
    pub const fn reset(self) -> bool {
        ((self.0[0] >> 4) & 1) != 0
    }

    /// 12-bit MAC sequence number.
    #[must_use]
    #[inline]
    pub const fn sequence_number(self) -> SequenceNumber {
        let raw = (((self.0[0] as u16) & 0x0F) << 8) | (self.0[1] as u16);
        match SequenceNumber::new(raw) {
            Some(s) => s,
            None => unreachable!(),
        }
    }

    /// 32-bit Receiver Address (Long RD ID).
    #[must_use]
    #[inline]
    pub fn receiver_address(self) -> u32 {
        u32::from_be_bytes(*<&[u8; 4]>::try_from(&self.0[2..6]).expect("slice len is 4"))
    }

    /// 32-bit Transmitter Address (Long RD ID).
    #[must_use]
    #[inline]
    pub fn transmitter_address(self) -> u32 {
        u32::from_be_bytes(*<&[u8; 4]>::try_from(&self.0[6..10]).expect("slice len is 4"))
    }

    /// Receiver Long RD ID (typed). `None` if the raw value is reserved (0).
    #[must_use]
    #[inline]
    pub fn receiver(self) -> Option<LongRdId> {
        LongRdId::new(self.receiver_address())
    }

    /// Transmitter Long RD ID (typed). `None` if the raw value is reserved (0).
    #[must_use]
    #[inline]
    pub fn transmitter(self) -> Option<LongRdId> {
        LongRdId::new(self.transmitter_address())
    }

    /// Build a 10-byte Unicast common header (Figure 6.3.3.3-1).
    #[must_use]
    #[inline]
    pub fn new_owned(
        reset: bool,
        sequence_number: SequenceNumber,
        receiver: LongRdId,
        transmitter: LongRdId,
    ) -> [u8; 10] {
        let seq = sequence_number.as_u16();
        let reset_bit: u8 = if reset { 0x10 } else { 0x00 };
        let rx = receiver.as_u32().to_be_bytes();
        let tx = transmitter.as_u32().to_be_bytes();
        [
            reset_bit | ((seq >> 8) as u8 & 0x0F),
            (seq & 0xFF) as u8,
            rx[0],
            rx[1],
            rx[2],
            rx[3],
            tx[0],
            tx[1],
            tx[2],
            tx[3],
        ]
    }
}

impl fmt::Debug for Unicast<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unicast")
            .field("reset", &self.reset())
            .field("sequence_number", &self.sequence_number())
            .field(
                "receiver_address",
                &format_args!("0x{:08x}", self.receiver_address()),
            )
            .field(
                "transmitter_address",
                &format_args!("0x{:08x}", self.transmitter_address()),
            )
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Unicast<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "Unicast {{ reset: {=bool}, seq: {=u16}, rx: 0x{=u32:08x}, tx: 0x{=u32:08x} }}",
            self.reset(),
            self.sequence_number().as_u16(),
            self.receiver_address(),
            self.transmitter_address()
        );
    }
}

// ---------------------------------------------------------------------------
// RdBroadcast - 6-byte header
// ETSI TS 103 636-4, clause 6.3.3.4, Figure 6.3.3.4-1
// ---------------------------------------------------------------------------

/// 6-byte RD Broadcasting common header.
///
/// Layout (Figure 6.3.3.4-1):
/// ```text
///    0   1   2   3   4   5   6   7     0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+ +-------------------------------+
/// |  Spare  |Reset| Sequence Number | |  Transmitter Address (32)    |
/// +---------+-----+-----------------+ +-------------------------------+
/// |         Sequence Number          |
/// +-----------------------------------+
/// ```
#[derive(Clone, Copy)]
pub struct RdBroadcast<'a>(pub &'a [u8; 6]);

impl RdBroadcast<'_> {
    /// `Reset` bit (clears MAC sequence-number state when set).
    #[must_use]
    #[inline]
    pub const fn reset(self) -> bool {
        ((self.0[0] >> 4) & 1) != 0
    }

    /// 12-bit MAC sequence number.
    #[must_use]
    #[inline]
    pub const fn sequence_number(self) -> SequenceNumber {
        let raw = (((self.0[0] as u16) & 0x0F) << 8) | (self.0[1] as u16);
        match SequenceNumber::new(raw) {
            Some(s) => s,
            None => unreachable!(),
        }
    }

    /// 32-bit Transmitter Address (Long RD ID).
    #[must_use]
    #[inline]
    pub fn transmitter_address(self) -> u32 {
        u32::from_be_bytes(*<&[u8; 4]>::try_from(&self.0[2..6]).expect("slice len is 4"))
    }

    /// Transmitter Long RD ID (typed). `None` if the raw value is reserved (0).
    #[must_use]
    #[inline]
    pub fn transmitter(self) -> Option<LongRdId> {
        LongRdId::new(self.transmitter_address())
    }

    /// Build a 6-byte RD Broadcasting common header (Figure 6.3.3.4-1).
    #[must_use]
    #[inline]
    pub fn new_owned(
        reset: bool,
        sequence_number: SequenceNumber,
        transmitter: LongRdId,
    ) -> [u8; 6] {
        let seq = sequence_number.as_u16();
        let reset_bit: u8 = if reset { 0x10 } else { 0x00 };
        let tx = transmitter.as_u32().to_be_bytes();
        [
            reset_bit | ((seq >> 8) as u8 & 0x0F),
            (seq & 0xFF) as u8,
            tx[0],
            tx[1],
            tx[2],
            tx[3],
        ]
    }
}

impl fmt::Debug for RdBroadcast<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RdBroadcast")
            .field("reset", &self.reset())
            .field("sequence_number", &self.sequence_number())
            .field(
                "transmitter_address",
                &format_args!("0x{:08x}", self.transmitter_address()),
            )
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for RdBroadcast<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "RdBroadcast {{ reset: {=bool}, seq: {=u16}, tx: 0x{=u32:08x} }}",
            self.reset(),
            self.sequence_number().as_u16(),
            self.transmitter_address()
        );
    }
}

// ---------------------------------------------------------------------------
// MacCommonHeader
// ---------------------------------------------------------------------------

/// Union of all four common header types (clauses 6.3.3.1 through 6.3.3.4).
#[derive(Clone, Copy)]
pub enum MacCommonHeader<'a> {
    /// DATA MAC PDU header, clause 6.3.3.1.
    DataMacPdu(DataMacPdu<'a>),
    /// Beacon Header, clause 6.3.3.2.
    Beacon(Beacon<'a>),
    /// Unicast Header, clause 6.3.3.3.
    Unicast(Unicast<'a>),
    /// RD Broadcasting Header, clause 6.3.3.4.
    RdBroadcast(RdBroadcast<'a>),
}

impl MacCommonHeader<'_> {
    /// Number of bytes this header occupies in the PDU.
    #[must_use]
    #[inline]
    pub const fn size(&self) -> usize {
        match self {
            MacCommonHeader::DataMacPdu(_) => 2,
            MacCommonHeader::Beacon(_) => 7,
            MacCommonHeader::Unicast(_) => 10,
            MacCommonHeader::RdBroadcast(_) => 6,
        }
    }

    /// 12-bit MAC sequence number (PSN) carried by this header.
    ///
    /// The Beacon header has no PSN field; clause 5.9.1.2 mandates
    /// PSN = 0 in the IV derivation for beacons, which is what this
    /// returns for [`MacCommonHeader::Beacon`].
    #[must_use]
    #[inline]
    pub const fn sequence_number(&self) -> SequenceNumber {
        match self {
            MacCommonHeader::DataMacPdu(h) => h.sequence_number(),
            MacCommonHeader::Beacon(_) => {
                const { SequenceNumber::new(0).expect("0 is a valid SequenceNumber") }
            }
            MacCommonHeader::Unicast(h) => h.sequence_number(),
            MacCommonHeader::RdBroadcast(h) => h.sequence_number(),
        }
    }
}

impl fmt::Debug for MacCommonHeader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacCommonHeader::DataMacPdu(h) => f.debug_tuple("DataMacPdu").field(h).finish(),
            MacCommonHeader::Beacon(h) => f.debug_tuple("Beacon").field(h).finish(),
            MacCommonHeader::Unicast(h) => f.debug_tuple("Unicast").field(h).finish(),
            MacCommonHeader::RdBroadcast(h) => f.debug_tuple("RdBroadcast").field(h).finish(),
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for MacCommonHeader<'_> {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            MacCommonHeader::DataMacPdu(h) => h.format(f),
            MacCommonHeader::Beacon(h) => h.format(f),
            MacCommonHeader::Unicast(h) => h.format(f),
            MacCommonHeader::RdBroadcast(h) => h.format(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_header_type_decoding() {
        let h = MacHeaderType(0b0001_0010);
        assert_eq!(h.version(), 0b00);
        assert_eq!(h.mac_security(), 0b01);
        assert_eq!(h.mac_header_type(), 0b0010);
    }

    #[test]
    fn data_mac_pdu_layout() {
        // byte 0: 0b0010_0000 -> spare=001, reset=0, seq_hi=0
        //         bit 4 of 0x20 is 0, so reset=false
        //         low 4 bits of 0x20 are 0
        // byte 1: 0x42 -> seq_lo = 0x42
        let bytes: [u8; 2] = [0x20, 0x42];
        let h = DataMacPdu(&bytes);
        assert!(!h.reset());
        assert_eq!(h.sequence_number().as_u16(), 0x042);

        // Now with reset=1
        let bytes: [u8; 2] = [0x30, 0x42];
        let h = DataMacPdu(&bytes);
        assert!(h.reset());
        assert_eq!(h.sequence_number().as_u16(), 0x042);
    }

    #[test]
    fn beacon_layout() {
        // network_id = 0x123456, tx = 0xAABBCCDD
        let bytes: [u8; 7] = [0x12, 0x34, 0x56, 0xAA, 0xBB, 0xCC, 0xDD];
        let h = Beacon(&bytes);
        assert_eq!(h.network_id(), 0x123456);
        assert_eq!(h.transmitter_address(), 0xAABBCCDD);
        assert_eq!(h.transmitter().unwrap().as_u32(), 0xAABBCCDD);
    }

    #[test]
    fn unicast_layout() {
        let bytes: [u8; 10] = [
            0x20, 0x00, // spare(3), reset(1=0), seq(12=0)
            0x11, 0x22, 0x33, 0x44, // receiver
            0xAA, 0xBB, 0xCC, 0xDD, // transmitter
        ];
        let h = Unicast(&bytes);
        assert!(!h.reset());
        assert_eq!(h.sequence_number().as_u16(), 0);
        assert_eq!(h.receiver_address(), 0x11223344);
        assert_eq!(h.transmitter_address(), 0xAABBCCDD);
    }

    #[test]
    fn rdbroadcast_layout() {
        let bytes: [u8; 6] = [
            0x20, 0x00, // spare(3), reset(1=0), seq(12=0)
            0xAA, 0xBB, 0xCC, 0xDD, // transmitter
        ];
        let h = RdBroadcast(&bytes);
        assert!(!h.reset());
        assert_eq!(h.sequence_number().as_u16(), 0);
        assert_eq!(h.transmitter_address(), 0xAABBCCDD);
    }

    #[test]
    fn common_header_size() {
        let data = [0u8; 2];
        let beacon = [0u8; 7];
        let unicast = [0u8; 10];
        let rdb = [0u8; 6];
        assert_eq!(MacCommonHeader::DataMacPdu(DataMacPdu(&data)).size(), 2);
        assert_eq!(MacCommonHeader::Beacon(Beacon(&beacon)).size(), 7);
        assert_eq!(MacCommonHeader::Unicast(Unicast(&unicast)).size(), 10);
        assert_eq!(MacCommonHeader::RdBroadcast(RdBroadcast(&rdb)).size(), 6);
    }
}
