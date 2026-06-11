//! Property: no IE payload may panic any of the typed message body
//! parsers. This is the main attack surface: every parser here runs
//! over raw radio input.
//!
//! The input is treated as a MAC IE stream; every recognized IE's
//! payload is dispatched into its `*Parts::parse`. Mu-dependent
//! bodies are parsed under both an 8-bit and a 16-bit width
//! configuration.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ts_103_636::mac::messages::*;
use ts_103_636::prelude::*;

fn dispatch(ie: &InformationElement<'_>) {
    let p = ie.payload();
    match ie.ie_number() {
        AnyIeType::Type6bit(t) => match t {
            IEType6bit::NetworkBeacon => {
                let _ = NetworkBeaconParts::parse(p);
            }
            IEType6bit::ClusterBeacon => {
                let _ = ClusterBeaconParts::parse(p, Mu::M1);
                let _ = ClusterBeaconParts::parse(p, Mu::M8);
            }
            IEType6bit::AssociationRequest => {
                let _ = AssociationRequestParts::parse(p);
            }
            IEType6bit::AssociationResponse => {
                let _ = AssociationResponseParts::parse(p);
            }
            IEType6bit::AssociationRelease => {
                let _ = AssociationReleaseParts::parse(p);
            }
            IEType6bit::ReconfigurationRequest => {
                let _ = ReconfigurationRequestParts::parse(p);
            }
            IEType6bit::ReconfigurationResponse => {
                let _ = ReconfigurationResponseParts::parse(p);
            }
            IEType6bit::MacSecurityInfo => {
                let _ = MacSecurityInfoParts::parse(p);
            }
            IEType6bit::RouteInfo => {
                let _ = RouteInfoParts::parse(p);
            }
            IEType6bit::ResourceAllocation => {
                let _ = ResourceAllocationParts::parse(p, Mu::M1);
                let _ = ResourceAllocationParts::parse(p, Mu::M8);
            }
            IEType6bit::RandomAccessResource => {
                let _ = RandomAccessResourceParts::parse(p, Mu::M1);
                let _ = RandomAccessResourceParts::parse(p, Mu::M8);
            }
            IEType6bit::RdCapability => {
                let _ = RdCapabilityParts::parse(p);
            }
            IEType6bit::Neighbouring => {
                let _ = NeighbouringParts::parse(p);
            }
            IEType6bit::BroadcastIndication => {
                let _ = BroadcastIndicationParts::parse(p);
            }
            IEType6bit::GroupAssignment => {
                let _ = GroupAssignmentParts::parse(p);
            }
            IEType6bit::LoadInfo => {
                let _ = LoadInfoParts::parse(p);
            }
            IEType6bit::MeasurementReport => {
                let _ = MeasurementReportParts::parse(p);
            }
            IEType6bit::SourceRouting => {
                let _ = SourceRoutingParts::parse(p);
            }
            IEType6bit::JoiningBeacon => {
                let _ = JoiningBeaconParts::parse(p);
            }
            IEType6bit::JoiningInformation => {
                let _ = JoiningInformationParts::parse(p);
            }
            // Opaque payloads / no body parser.
            IEType6bit::Padding
            | IEType6bit::HigherLayerSignallingFlow1
            | IEType6bit::HigherLayerSignallingFlow2
            | IEType6bit::UserPlaneDataFlow1
            | IEType6bit::UserPlaneDataFlow2
            | IEType6bit::UserPlaneDataFlow3
            | IEType6bit::UserPlaneDataFlow4
            | IEType6bit::AdditionalMacMessages
            | IEType6bit::Escape
            | IEType6bit::IeTypeExtension => {}
        },
        AnyIeType::Type5bit(t) => match t {
            ShortIeType::Len1(IEType5bitLen1::RadioDeviceStatus) => {
                let _ = RadioDeviceStatusParts::parse(p);
            }
            ShortIeType::Len1(IEType5bitLen1::RdCapabilityShort) => {
                let _ = RdCapabilityShortParts::parse(p);
            }
            ShortIeType::Len1(IEType5bitLen1::AssociationControl) => {
                let _ = AssociationControlParts::parse(p);
            }
            ShortIeType::Len0(IEType5bitLen0::MacSecurityInfo) => {
                let _ = MacSecurityInfoParts::parse(p);
            }
            _ => {}
        },
        AnyIeType::Unknown6bit(_) | AnyIeType::Unknown5bit(_) => {}
    }
}

fuzz_target!(|data: &[u8]| {
    for ie in InformationElement::parse_stream(data) {
        match ie {
            Ok(ie) => dispatch(&ie),
            Err(_) => break,
        }
    }
});
