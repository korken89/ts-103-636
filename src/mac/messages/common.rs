//! Shared building blocks reused by multiple message bodies.
//!
//! Currently: [`AllocationPair`], used by Resource Allocation
//! (clause 6.4.3.3) and Random Access Resource (clause 6.4.3.4); the
//! wire codecs live in the generated modules.

use crate::types::{PacketLengthType, RaLength};

/// A (start_subslot, length_type, length) triple used by Resource
/// Allocation and Random Access Resource IEs. The start_subslot is
/// 8-bit or 9-bit depending on the PHY subcarrier scaling factor
/// [`Mu`](crate::types::Mu) (8-bit when `mu <= 4`, otherwise 9-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct AllocationPair {
    /// First subslot of the allocation in the indicated frame.
    /// On the wire this is an 8-bit field for `mu <= 4` and a 9-bit
    /// field for `mu > 4`; valid range therefore depends on `mu` and is
    /// checked at serialize / parse time.
    pub start_subslot: u16,
    pub length_type: PacketLengthType,
    pub length: RaLength,
}
