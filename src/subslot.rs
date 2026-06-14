//! Subslot capacity helpers.
//!
//! Two paths are exposed, side by side, because they answer different
//! (but overlapping) questions:
//!
//! **The `*_nrf9151` family** ([`min_subslots_nrf9151`],
//! [`bits_per_subslots_nrf9151`], [`max_subslots_nrf9151`]) is a fast
//! lookup against `SUBSLOT_BITS_NRF9151`, the table published in
//! `nrf_modem/doc/dect/dectphy.rst`. The table is keyed by the 4-bit
//! `packet_length` field (0..=15 → 1..=16 subslots) for MCS 0..=4 at
//! the nRF9151's fixed configuration (`mu=1`, `beta=1`, `N_SS=1`). The
//! table encodes **both** the spec-derived TBS and the modem's
//! supported-combination matrix: cells past
//! [`MAX_SUBSLOTS_NRF9151`]`[mcs]` correspond to packet lengths the
//! spec allows but the modem rejects.
//!
//! **The general path** ([`compute_tbs`], [`min_subslots`]) implements
//! the TS 103 636-3 clause 5.3 TBS formula directly for any valid
//! `(subslot_count, mcs, beta, mu, n_ss)`. It is independent of any
//! particular modem's supported set; it returns whatever the spec
//! arithmetic produces.
//!
//! Where the two paths overlap (i.e. at `mu=1, beta=1, N_SS=1`,
//! `mcs <= 4`, and `subslot_count` within the modem's supported range)
//! they agree exactly - this is locked in by the
//! `general_formula_matches_nrf9151_table` test.
//!
//! **Which one should I call?**
//!
//! * Targeting the nRF9151 → use the `_nrf9151` flavor. You get the
//!   modem's hardware limits enforced automatically (`None` for
//!   unsupported `(mcs, subslot_count)` pairs).
//! * Targeting a different DECT NR+ implementation, or doing
//!   spec-conformance analysis → use [`compute_tbs`] /
//!   [`min_subslots`]. They have no hardware-specific knowledge and
//!   accept any spec-valid configuration.
//! * Validating the table against the spec → use [`compute_tbs`] as
//!   the oracle; that's what the in-crate test does.

use crate::constants;
use crate::types::{Beta, Mcs, Mu};

// ---------------------------------------------------------------------------
// nRF9151 fast path (mu=1, beta=1, N_SS=1)
// Source: nrf_modem/doc/dect/dectphy.rst, "Bits per subslot index with
// given modulation scheme".
// ---------------------------------------------------------------------------

/// Transport block size in bits for the nRF9151 (mu=1, beta=1, N_SS=1).
///
/// Indexed as `[mcs as usize][subslot_index as usize]` where
/// `subslot_index` is the 4-bit packet-length field value (0..=15)
/// carrying subslot counts 1..=16. Sourced from
/// `nrf_modem/doc/dect/dectphy.rst`, "Bits per subslot index with given
/// modulation scheme".
///
/// A `0` entry has two meanings:
/// * `[mcs 0][0] = 0`: the spec formula genuinely returns TBS = 0 for
///   1 subslot at MCS 0 (clause 5.3 Note 2).
/// * any entry at index >= [`MAX_SUBSLOTS_NRF9151`]`[mcs]`: the modem
///   does not support that packet length; the spec formula would
///   return a valid value but the hardware rejects it. Disambiguate
///   via [`MAX_SUBSLOTS_NRF9151`].
const SUBSLOT_BITS_NRF9151: [[u32; 16]; 5] = [
    // MCS 0
    [
        0, 136, 264, 400, 536, 664, 792, 920, 1064, 1192, 1320, 1448, 1576, 1704, 1864, 1992,
    ],
    // MCS 1
    [
        32, 296, 552, 824, 1096, 1352, 1608, 1864, 2104, 2360, 2616, 2872, 3128, 3384, 3704, 3960,
    ],
    // MCS 2
    [
        56, 456, 856, 1256, 1640, 2024, 2360, 2744, 3192, 3576, 3960, 4320, 4768, 5152, 5536, 0,
    ],
    // MCS 3
    [
        88, 616, 1128, 1672, 2168, 2680, 3192, 3704, 4256, 4768, 5280, 0, 0, 0, 0, 0,
    ],
    // MCS 4
    [
        144, 936, 1736, 2488, 3256, 4024, 4832, 5600, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
];

/// Maximum supported subslot count per MCS on the nRF9151 (true
/// counts, not packet-length field values): the table's zero cutoffs
/// at higher packet lengths reflect the modem's TBS cap.
///
/// Note: Nordic's `dectphy.rst` table lists valid TBS values at
/// subslot index 15 (= 16 subslots) for MCS 0 and 1, while its prose
/// says "a transmission can take up to 15 sub-slots". This constant
/// follows the table (and the 4-bit packet_length field, which
/// encodes counts 1..=16); to be confirmed on hardware.
pub const MAX_SUBSLOTS_NRF9151: [u8; 5] = [16, 16, 15, 11, 8];

/// Smallest subslot count on the nRF9151 that can carry `payload_bytes`
/// bytes at `mcs`.
///
/// # Returns
///
/// `Some(n)` with `n` in `1..=16` and at most [`MAX_SUBSLOTS_NRF9151`]
/// for the given MCS. `None` if the payload does not fit in the
/// supported subslot counts, or if `mcs > 4` (out of nRF9151 range).
///
/// # Spec note
///
/// The on-air 4-bit packet_length field carries `n - 1` (a signalled
/// value plus one, clause 6.2.1); convert via
/// [`crate::types::PacketLength`].
pub fn min_subslots_nrf9151(payload_bytes: usize, mcs: Mcs) -> Option<u8> {
    let mcs_idx = mcs.as_u8() as usize;
    if mcs_idx >= SUBSLOT_BITS_NRF9151.len() {
        return None;
    }
    let payload_bits = u32::try_from(payload_bytes.checked_mul(8)?).ok()?;
    let row = &SUBSLOT_BITS_NRF9151[mcs_idx];
    for n in 1..=MAX_SUBSLOTS_NRF9151[mcs_idx] {
        let bits = row[(n - 1) as usize];
        if bits != 0 && bits >= payload_bits {
            return Some(n);
        }
    }
    None
}

/// Number of bits carried by exactly `subslot_count` subslots at `mcs`
/// on the nRF9151.
///
/// # Returns
///
/// `Some(bits)` for `subslot_count` in `1..=MAX_SUBSLOTS_NRF9151[mcs]`
/// (note `Some(0)` for 1 subslot at MCS 0: clause 5.3 NOTE 2, the TBS
/// is genuinely zero). `None` if `subslot_count` is 0 or exceeds the
/// modem's limit for this MCS, or if `mcs > 4`.
pub fn bits_per_subslots_nrf9151(subslot_count: u8, mcs: Mcs) -> Option<u32> {
    let mcs_idx = mcs.as_u8() as usize;
    if mcs_idx >= SUBSLOT_BITS_NRF9151.len() {
        return None;
    }
    if subslot_count == 0 || subslot_count > MAX_SUBSLOTS_NRF9151[mcs_idx] {
        return None;
    }
    Some(SUBSLOT_BITS_NRF9151[mcs_idx][(subslot_count - 1) as usize])
}

/// Maximum subslot count the nRF9151 supports for `mcs`.
///
/// # Returns
///
/// `Some(max)` for MCS 0..=4, `None` for higher MCS.
pub fn max_subslots_nrf9151(mcs: Mcs) -> Option<u8> {
    let mcs_idx = mcs.as_u8() as usize;
    MAX_SUBSLOTS_NRF9151.get(mcs_idx).copied()
}

// ---------------------------------------------------------------------------
// General formula
// ETSI TS 103 636-3, clause 5.3 (Transport block size) and the surrounding
// sections on PDC resource element computation.
// ---------------------------------------------------------------------------

/// Compute the transport block size in bits for `subslot_count` subslots
/// with the given PHY configuration.
///
/// Implements the resource-element formula (clause 5.2.5) and the TBS
/// quantization step (clause 5.3) of ETSI TS 103 636-3, including
/// segmentation when N_M exceeds the maximum turbo encoder code block
/// size Z.
///
/// Returns `Some(0)` for configurations where Note 2 of clause 5.3
/// applies (beta=1, MCS=0, packet length=0, packet length type=0): the
/// PDC is left empty. The return type is `u32`: high-beta/mu/MCS
/// configurations legitimately exceed 65535 bits (e.g. Annex C lists
/// 530432 bits for mu=8, beta=16, MCS 11).
pub const fn compute_tbs(subslot_count: u8, mcs: Mcs, beta: Beta, mu: Mu, n_ss: u8) -> Option<u32> {
    if subslot_count == 0 || subslot_count > constants::MAX_SUBSLOTS {
        return None;
    }
    if n_ss == 0 {
        return None;
    }

    // Clause 5.2.5: PDC resource elements.
    let n_packet_sym = (subslot_count as u32) * (constants::N_SYM_SUBSLOT as u32);
    let n_df_sym = n_packet_sym.saturating_sub(mu.n_gi_plus_stf_sym() as u32);
    if n_df_sym == 0 {
        return None;
    }

    // N_DRS_re = ceil(N_PACKET_sym / N_step) * N_eff_TX * N_OCC[beta] / 4.
    // N_step = 5 for N_eff_TX <= 2, 10 for N_eff_TX >= 4. We treat N_SS as
    // N_eff_TX (single-codeword case).
    let drs_step = if n_ss <= 2 {
        constants::DRS_TIME_STEP_DEFAULT as u32
    } else {
        constants::DRS_TIME_STEP_4PLUS as u32
    };
    let n_drs_re = n_packet_sym.div_ceil(drs_step) * (n_ss as u32) * (beta.n_occ() as u32) / 4;
    let n_re_pdc = n_df_sym
        .saturating_mul(beta.n_occ() as u32)
        .saturating_sub(n_drs_re)
        .saturating_sub(constants::N_PCC_RE as u32);

    // Clause 5.3: N_PDC_bits = N_SS * N_PDC_re * N_bps * R.
    let n_bps = mcs.n_bps() as u32;
    let (rate_num, rate_den) = mcs.code_rate();
    let n_pdc_bits = (n_ss as u32)
        .saturating_mul(n_re_pdc)
        .saturating_mul(n_bps)
        .saturating_mul(rate_num as u32)
        / (rate_den as u32);

    // Clause 5.3: quantization step M.
    let m: u32 = if n_pdc_bits <= 512 {
        8
    } else if n_pdc_bits <= 1024 {
        16
    } else if n_pdc_bits <= 2048 {
        32
    } else {
        64
    };
    // N_M = largest multiple of M not greater than N_PDC_bits.
    let n_m = (n_pdc_bits / m) * m;

    let l = constants::CRC_L as u32; // 24
    let z = constants::Z as u32; // 2048

    // Clause 5.3 NOTE 2: if N_M < L the PDC is left empty (TBS = 0).
    let tbs_bits = if n_m < l {
        0
    } else if n_m <= z {
        n_m - l
    } else {
        // Segmentation: C = ceil(N_M / Z) code blocks, each protected by
        // its own L-bit CRC, plus one L-bit transport-block CRC.
        let c = n_m.div_ceil(z);
        n_m.saturating_sub((c + 1) * l)
    };

    Some(tbs_bits)
}

/// Smallest subslot count whose [`compute_tbs`] fits `payload_bytes`
/// bytes, or `None` if the payload does not fit within
/// `MAX_SUBSLOTS`.
pub const fn min_subslots(
    payload_bytes: usize,
    mcs: Mcs,
    beta: Beta,
    mu: Mu,
    n_ss: u8,
) -> Option<u8> {
    let payload_bits = match payload_bytes.checked_mul(8) {
        // Explicit range check instead of `as`: on 64-bit hosts a huge
        // payload would otherwise truncate and "fit".
        Some(p) if p <= u32::MAX as usize => p as u32,
        _ => return None,
    };
    let mut n = 1;
    while n <= constants::MAX_SUBSLOTS {
        if let Some(tbs) = compute_tbs(n, mcs, beta, mu, n_ss)
            && tbs >= payload_bits
        {
            return Some(n);
        }
        n += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Beta, Mcs, Mu};

    fn mcsi(n: u8) -> Mcs {
        Mcs::try_from_u8(n).unwrap()
    }

    #[test]
    fn nrf_table_mcs0_matches_doc() {
        // dectphy.rst worked example: 17 bytes (136 bits) at MCS 0
        // takes exactly 2 subslots; 33 bytes (264 bits) takes 3.
        assert_eq!(bits_per_subslots_nrf9151(1, mcsi(0)).unwrap(), 0);
        assert_eq!(bits_per_subslots_nrf9151(2, mcsi(0)).unwrap(), 136);
        assert_eq!(bits_per_subslots_nrf9151(3, mcsi(0)).unwrap(), 264);
        assert_eq!(bits_per_subslots_nrf9151(16, mcsi(0)).unwrap(), 1992);
        assert!(bits_per_subslots_nrf9151(0, mcsi(0)).is_none());
    }

    #[test]
    fn nrf_table_mcs4_max() {
        assert_eq!(max_subslots_nrf9151(mcsi(4)).unwrap(), 8);
        assert_eq!(bits_per_subslots_nrf9151(8, mcsi(4)).unwrap(), 5600);
        assert!(bits_per_subslots_nrf9151(9, mcsi(4)).is_none());
    }

    #[test]
    fn nrf_table_unsupported_subslots() {
        // MCS 3: max 11 subslots
        assert!(bits_per_subslots_nrf9151(12, mcsi(3)).is_none());
        // MCS 2: max 15 subslots
        assert!(bits_per_subslots_nrf9151(16, mcsi(2)).is_none());
    }

    #[test]
    fn nrf_min_subslots_basic() {
        // dectphy.rst worked example: 17 bytes at MCS 0 -> 2 subslots.
        assert_eq!(min_subslots_nrf9151(17, mcsi(0)).unwrap(), 2);
        // 33 bytes = 264 bits -> 3 subslots
        assert_eq!(min_subslots_nrf9151(33, mcsi(0)).unwrap(), 3);
        // 249 bytes = 1992 bits -> 16 subslots
        assert_eq!(min_subslots_nrf9151(249, mcsi(0)).unwrap(), 16);
        // 1 byte at MCS 1 fits the 1-subslot TBS (32 bits).
        assert_eq!(min_subslots_nrf9151(1, mcsi(1)).unwrap(), 1);
    }

    #[test]
    fn nrf_min_subslots_agrees_with_general_formula() {
        // The module doc promises the two paths agree on the overlap.
        for mcs in 0..=4 {
            for payload in [1, 4, 17, 33, 100, 249, 495, 700] {
                let general = min_subslots(payload, mcsi(mcs), Beta::B1, Mu::M1, 1)
                    .filter(|&n| n <= MAX_SUBSLOTS_NRF9151[mcs as usize]);
                let nrf = min_subslots_nrf9151(payload, mcsi(mcs));
                assert_eq!(
                    nrf, general,
                    "payload {payload} bytes at MCS {mcs}: nrf {nrf:?} vs general {general:?}"
                );
            }
        }
    }

    #[test]
    fn nrf_min_subslots_rejects_oversize() {
        // 250 bytes > 1992 bits -> doesn't fit
        assert!(min_subslots_nrf9151(250, mcsi(0)).is_none());
        // MCS 4 caps at 8 subslots = 5600 bits = 700 bytes
        assert!(min_subslots_nrf9151(701, mcsi(4)).is_none());
        // Huge payload must not truncate into "fits" on 64-bit hosts.
        assert!(min_subslots_nrf9151(0x2000_0000, mcsi(1)).is_none());
    }

    #[test]
    fn nrf_min_subslots_rejects_high_mcs() {
        assert!(min_subslots_nrf9151(10, mcsi(5)).is_none());
        assert!(min_subslots_nrf9151(10, mcsi(11)).is_none());
    }

    #[test]
    fn general_formula_clause_5_3_note_2_returns_zero() {
        // β = 1, MCS = 0, packet length = 0 (1 subslot) -> PDC empty.
        assert_eq!(compute_tbs(1, mcsi(0), Beta::B1, Mu::M1, 1), Some(0));
    }

    /// Walk the nRF9151 TBS table and verify the general-formula
    /// [`compute_tbs`] reproduces every supported entry exactly.
    /// `[mcs][idx]` carries TBS for `subslot_count = idx + 1` at the
    /// nRF default config (mu=1, beta=1, N_SS=1).
    #[test]
    fn general_formula_matches_nrf9151_table() {
        for (mcs_idx, row) in SUBSLOT_BITS_NRF9151.iter().enumerate() {
            let mcs = mcsi(mcs_idx as u8);
            let max_count = MAX_SUBSLOTS_NRF9151[mcs_idx] as usize;
            for (idx, &expected) in row.iter().take(max_count).enumerate() {
                let subslot_count = (idx + 1) as u8;
                let got = compute_tbs(subslot_count, mcs, Beta::B1, Mu::M1, 1)
                    .expect("supported entry should compute");
                assert_eq!(
                    got, expected,
                    "MCS {mcs_idx}, subslot_index {idx} (subslot_count {subslot_count}): \
                     formula {got} vs nRF table {expected}"
                );
            }
        }
    }
}
