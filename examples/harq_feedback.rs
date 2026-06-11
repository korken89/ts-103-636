//! HARQ on the nRF9151: PCC Type 2 headers, feedback, and TBS-exact
//! data sizing, structured the way the modem API drives a MAC.
//!
//! DECT NR+ carries HARQ feedback inside the Physical Layer Control
//! Field (PCC) Type 2 (clause 6.2.2), not in the MAC PDU. On the
//! nRF9151 (`nrf_modem_dect_phy`):
//!
//! * TX: the application fills `nrf_modem_dect_phy_tx_params` with a
//!   10-byte `phy_header` (exactly the bytes [`PccType2F000::to_bytes`]
//!   produces) and a `data` buffer whose length MUST exactly match the
//!   TBS implied by (packet_length, MCS) - that is what the
//!   [`subslot`] helpers compute and `finish_without_security_padded`
//!   fills.
//! * RX: the modem raises `NRF_MODEM_DECT_PHY_EVT_PCC` with the raw
//!   header (parse it with [`Pcc::parse`]) before the data (PDC) has
//!   been CRC-checked, and `..._EVT_PDC` with the payload afterwards.
//! * HARQ response: `nrf_modem_dect_phy_tx_harq` must be called
//!   directly from the PCC event (time critical), so the feedback is
//!   ALWAYS generated as NACK - the modem flips it to ACK on its own
//!   once the PDC checksum passes. The application therefore never
//!   sets `ack: true` itself on this path.
//!
//! This example simulates both radios in memory: an FT sending data on
//! one HARQ process and a PT generating the feedback the nRF9151 way.
//!
//! Run with `cargo run --example harq_feedback`.

use ts_103_636::pcc::{Pcc, PccType2F000};
use ts_103_636::prelude::*;
use ts_103_636::subslot::{bits_per_subslots_nrf9151, min_subslots_nrf9151};
use ts_103_636::types::{BufferStatus, Cqi, Feedback, HarqProcess};

const FT_SHORT_ID: u16 = 0x2222;
const PT_SHORT_ID: u16 = 0x1111;
const SHORT_NETWORK_ID: u8 = 0xAB;

/// Everything `nrf_modem_dect_phy_tx()` needs from the MAC: the raw
/// 10-byte PHY header and the TBS-exact data buffer
/// (`nrf_modem_dect_phy_tx_params.phy_header` / `.data`).
struct TxParams {
    phy_header: [u8; 10],
    data_size: usize,
}

/// FT side: build one HARQ (re)transmission of `payload` on `process`.
///
/// Mirrors the nRF9151 TX path: pick the smallest subslot count that
/// carries the MAC PDU at the chosen MCS, signal `count - 1` in the
/// 4-bit packet length field, and pad the MAC PDU to exactly the TBS
/// so `data_size` matches what the modem requires.
fn ft_build_tx(
    buf: &mut [u8],
    payload: &[u8],
    process: u8,
    ndi: bool,
    rv: u8,
    feedback: Feedback,
) -> TxParams {
    let mcs = Mcs::new(2).unwrap();
    let psn = SequenceNumber::new(42).unwrap();
    let ft = LongRdId::new(0x2222_BBBB).unwrap();
    let pt = LongRdId::new(0x1111_AAAA).unwrap();

    // MAC PDU overhead: 1 header type byte + 10-byte unicast header +
    // 2-byte IE header. Compute the bare PDU first, then size the TBS.
    let ie =
        InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, payload).unwrap();
    let bare_len = 1 + 10 + ie.encoded_len();

    let subslots = min_subslots_nrf9151(bare_len, mcs).expect("payload fits some packet length");
    let tbs_bytes = bits_per_subslots_nrf9151(subslots, mcs).unwrap() as usize / 8;

    let pdu = MacPduBuilder::new(buf)
        .push_unicast(NotUsed, false, psn, pt, ft)
        .unwrap()
        .push_ie(&ie)
        .unwrap()
        // The modem requires data_size == TBS: fill the gap with
        // Padding IEs.
        .finish_without_security_padded(tbs_bytes)
        .unwrap();

    let phy_header = PccType2F000 {
        packet_length_type: PacketLengthType::Subslot,
        // On-air field carries subslot count - 1 (clause 6.2.1).
        packet_length: PacketLength::new(subslots - 1).unwrap(),
        short_network_id: NetworkId8::new(SHORT_NETWORK_ID).unwrap(),
        transmitter_identity: ShortRdId::new(FT_SHORT_ID).unwrap(),
        transmit_power: TransmitPower::Dbm10,
        df_mcs: mcs,
        receiver_identity: ShortRdId::new(PT_SHORT_ID).unwrap(),
        spatial_streams: 0,
        df_redundancy_version: rv,
        df_new_data_indication: ndi,
        df_harq_process_number: process,
        feedback,
    }
    .to_bytes();

    TxParams {
        phy_header,
        data_size: pdu.len(),
    }
}

/// PT side: the `NRF_MODEM_DECT_PHY_EVT_PCC` handler.
///
/// Called BEFORE the PDC is decoded, so per the Nordic documentation
/// the HARQ feedback handed to `nrf_modem_dect_phy_tx_harq` is always
/// a NACK; the modem adjusts it to ACK by itself if the PDC checksum
/// later passes. Returns the feedback for the response header.
fn pt_on_pcc_event(phy_header: &[u8]) -> Feedback {
    let Pcc::Type2F000(header) = Pcc::parse(phy_header).expect("well-formed PCC") else {
        panic!("expected a Type 2 F000 header");
    };
    println!(
        "PT: PCC event: from 0x{:04x}, process {}, NDI={}, RV={}, MCS {}",
        u16::from(header.transmitter_identity),
        header.df_harq_process_number,
        u8::from(header.df_new_data_indication),
        header.df_redundancy_version,
        header.df_mcs.as_u8(),
    );

    // Time-critical path: schedule nrf_modem_dect_phy_tx_harq with the
    // prepared response. ack is ALWAYS false here on the nRF9151.
    Feedback::Format1 {
        harq_process: HarqProcess::new(header.df_harq_process_number).unwrap(),
        ack: false,
        // Report the PT's own pending uplink data.
        buffer_status: BufferStatus::UpTo256,
        // Channel estimate from this reception.
        cqi: Cqi::Mcs(Mcs::new(2).unwrap()),
    }
}

/// Simulate the modem behavior between PCC scheduling and the actual
/// HARQ response going out on air: if the PDC CRC passed, the modem
/// flips the NACK to an ACK.
fn modem_adjusts_feedback(scheduled: Feedback, pdc_crc_ok: bool) -> Feedback {
    match scheduled {
        Feedback::Format1 {
            harq_process,
            buffer_status,
            cqi,
            ..
        } if pdc_crc_ok => Feedback::Format1 {
            harq_process,
            ack: true,
            buffer_status,
            cqi,
        },
        other => other,
    }
}

/// FT side: react to the feedback in the PT's response PCC. Returns
/// the (ndi, rv) for the next transmission on that process.
fn ft_on_pcc_event(phy_header: &[u8], ndi: bool) -> (bool, u8) {
    let Pcc::Type2F000(header) = Pcc::parse(phy_header).expect("well-formed PCC") else {
        panic!("expected a Type 2 F000 header");
    };
    match header.feedback {
        Feedback::Format1 {
            harq_process,
            ack,
            buffer_status,
            cqi,
        } => {
            if ack {
                println!(
                    "FT: process {} ACKed (PT buffer {:?}, CQI {:?}) -> new data, toggle NDI",
                    harq_process.as_u8(),
                    buffer_status,
                    cqi,
                );
                (!ndi, 0)
            } else {
                println!(
                    "FT: process {} NACKed (CQI {:?}) -> retransmit, same NDI, next RV",
                    harq_process.as_u8(),
                    cqi,
                );
                (ndi, 2)
            }
        }
        other => panic!("unexpected feedback: {other:?}"),
    }
}

fn main() {
    let payload = [0x42; 40];
    let process = 2;
    let mut ndi = true;
    let mut rv = 0;

    // ---------------------------------------------------------------
    // Round 1: FT transmits, the PT's PDC CRC fails -> NACK on air.
    // ---------------------------------------------------------------
    let mut tx_buf = [0; 256];
    let tx = ft_build_tx(&mut tx_buf, &payload, process, ndi, rv, Feedback::None);
    println!(
        "FT: TX process {process}, NDI={}, RV={rv}, data_size={} (TBS-exact)",
        u8::from(ndi),
        tx.data_size,
    );

    let scheduled = pt_on_pcc_event(&tx.phy_header);
    let on_air = modem_adjusts_feedback(scheduled, false); // PDC CRC failed
    println!("PT: PDC CRC failed -> modem keeps the scheduled NACK");

    // The PT's HARQ response carries the feedback in its own PCC.
    let mut pt_buf = [0; 256];
    let pt_tx = ft_build_tx(&mut pt_buf, &[0; 4], 0, true, 0, on_air);
    (ndi, rv) = ft_on_pcc_event(&pt_tx.phy_header, ndi);

    // ---------------------------------------------------------------
    // Round 2: retransmission (same NDI, next RV); PDC CRC passes and
    // the modem flips the PT's scheduled NACK into an ACK.
    // ---------------------------------------------------------------
    println!();
    let tx = ft_build_tx(&mut tx_buf, &payload, process, ndi, rv, Feedback::None);
    println!(
        "FT: TX process {process}, NDI={}, RV={rv}, data_size={} (TBS-exact)",
        u8::from(ndi),
        tx.data_size,
    );

    let scheduled = pt_on_pcc_event(&tx.phy_header);
    let on_air = modem_adjusts_feedback(scheduled, true); // PDC CRC ok
    println!("PT: PDC CRC ok -> modem flips the scheduled NACK to ACK");

    let pt_tx = ft_build_tx(&mut pt_buf, &[0; 4], 0, true, 0, on_air);
    (ndi, rv) = ft_on_pcc_event(&pt_tx.phy_header, ndi);
    let _ = (ndi, rv);

    // ---------------------------------------------------------------
    // Multi-process: Format 4 ACKs several processes in one go (bit N
    // = process N decoded and not yet ACKed); Format 7 reports buffer
    // status alone when there is no HARQ state to convey.
    // ---------------------------------------------------------------
    println!();
    let bitmap = Feedback::Format4 {
        harq_feedback_bitmap: 0b0000_0101, // processes 0 and 2
        cqi: Cqi::Mcs(Mcs::new(3).unwrap()),
    };
    let pt_tx = ft_build_tx(&mut pt_buf, &[0; 4], 1, false, 0, bitmap);
    let Pcc::Type2F000(header) = Pcc::parse(&pt_tx.phy_header).unwrap() else {
        unreachable!()
    };
    if let Feedback::Format4 {
        harq_feedback_bitmap,
        ..
    } = header.feedback
    {
        let acked: heapless::Vec<u8, 8> = (0..8)
            .filter(|n| harq_feedback_bitmap & (1 << n) != 0)
            .collect();
        println!("FT: bitmap feedback, ACKed processes {acked:?}");
    }

    let bs_only = Feedback::Format7 {
        buffer_status: BufferStatus::Empty,
        cqi: None,
    };
    let pt_tx = ft_build_tx(&mut pt_buf, &[0; 4], 1, false, 0, bs_only);
    let Pcc::Type2F000(header) = Pcc::parse(&pt_tx.phy_header).unwrap() else {
        unreachable!()
    };
    if let Feedback::Format7 {
        buffer_status,
        cqi: None,
    } = header.feedback
    {
        println!("FT: buffer-status-only feedback: {buffer_status:?} (no CQI included)");
    }
}
