#![doc = include_str!("../README.md")]
#![no_std]
#![deny(missing_docs)]

pub mod constants;
pub mod mac;
pub mod pcc;
pub mod prelude;
pub mod security;
pub mod subslot;
pub mod types;

/// Error returned when constructing a type from a raw value that contains bits
/// outside the type's valid range (e.g. reserved or excessive bits set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExcessiveBitsSet;

/// Error returned when parsed data violates the specification.
///
/// Fieldless (1 byte, no offsets): the kind alone is usually enough to
/// bisect an on-device parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ParsingError {
    /// The input ended before the structure was complete (short
    /// buffer, truncated optional field, missing length byte, ...).
    Truncated,
    /// A field carries a value the spec marks reserved, or an enum
    /// discriminant outside the defined value set.
    ReservedValue,
    /// A length field contradicts the surrounding structure (e.g. an
    /// IE length running past the end of the PDU).
    BadLength,
    /// A header / format discriminant is unknown or unsupported (MAC
    /// version, MAC header type, PCC header format, ...).
    InvalidHeader,
}

/// Error returned when writing to a buffer and there is not enough space
/// for the requested operation (also raised for invalid input lengths
/// that would exceed the encoding bound).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BufferFull;
