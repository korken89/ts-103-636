//! `kani::Arbitrary` impls for the typed field wrappers, compiled
//! only under `cargo kani` (`--cfg kani`).
//!
//! Every impl goes through the type's PUBLIC constructor and assumes
//! the constructor accepted the raw value. This matters: a derived
//! `Arbitrary` would conjure representations a user cannot construct
//! (e.g. an out-of-range `Mcs`), and the serialize-totality proofs in
//! `verify/` would then cover values that cannot exist. With these
//! impls, "any value" means exactly "any value reachable through the
//! public API".
//!
//! Three shapes:
//! - `arb_via!`: fallible one-raw-value constructor (`new` /
//!   `try_from_u8` returning `Option`); draw a symbolic raw and
//!   assume the constructor accepts it.
//! - `arb_raw!`: transparent newtype over a full-range integer with a
//!   `pub` field; every representation is constructible.
//! - `arb_fields!`: composite with all-`pub` fields, built
//!   field-by-field - exactly what a user can do.

use heapless::Vec;

use crate::mac::messages::association_request::FtModeFields;
use crate::mac::messages::association_response::{
    AssociationAcceptParts, AssociationResponseParts, FlowAcceptance, GroupAssignment,
    HarqOverride, MAX_RESPONSE_FLOWS,
};
use crate::mac::messages::broadcast_indication::BroadcastRdId;
use crate::mac::messages::common::AllocationPair;
use crate::mac::messages::group_assignment::GroupResourceTagEntry;
use crate::mac::messages::neighbouring::RadioDeviceClass;
use crate::mac::messages::random_access_resource::RachRepeatPolicy;
use crate::mac::messages::rd_capability::{AdditionalPhyCapability, PhyCapability};
use crate::mac::messages::reconfiguration::HarqConfig;
use crate::mac::messages::resource_allocation::{
    AllocationOptions, RepeatPolicy, ResourceAllocationKind,
};
use crate::types::*;

macro_rules! arb_via {
    ($ty:ty, $raw:ty, $ctor:path) => {
        impl kani::Arbitrary for $ty {
            fn any() -> Self {
                let raw: $raw = kani::any();
                let v = $ctor(raw);
                kani::assume(v.is_some());
                v.unwrap()
            }
        }
    };
}

macro_rules! arb_raw {
    ($ty:ident) => {
        impl kani::Arbitrary for $ty {
            fn any() -> Self {
                $ty(kani::any())
            }
        }
    };
}

macro_rules! arb_fields {
    ($ty:ty { $($field:ident),+ $(,)? }) => {
        impl kani::Arbitrary for $ty {
            fn any() -> Self {
                Self {
                    $($field: kani::any(),)+
                }
            }
        }
    };
}

// Exemplars; the sweep below this line covers every field type used
// by the generated Parts structs and their composite support types.
arb_via!(Mcs, u8, Mcs::new);
arb_raw!(LoadPercentage);

// types::association
arb_via!(DlcServiceType, u8, DlcServiceType::try_from_u8);
arb_via!(HarqFeedbackDelay, u8, HarqFeedbackDelay::new);
arb_via!(HarqProcesses, u8, HarqProcesses::new);
arb_via!(MacSecuritySupport, u8, MacSecuritySupport::try_from_u8);
arb_via!(MaxHarqReTx, u8, MaxHarqReTx::new);
arb_via!(OperatingModes, u8, OperatingModes::try_from_u8);
arb_via!(Release, u8, Release::try_from_u8);
arb_via!(ReleaseCause, u8, ReleaseCause::try_from_u8);
arb_via!(RejectCause, u8, RejectCause::try_from_u8);
arb_via!(RejectTimer, u8, RejectTimer::try_from_u8);
arb_via!(SetupCause, u8, SetupCause::try_from_u8);

// types::beacon
arb_via!(ClusterBeaconPeriod, u8, ClusterBeaconPeriod::try_from_u8);
arb_via!(CountToTrigger, u8, CountToTrigger::new);
arb_via!(NetworkBeaconPeriod, u8, NetworkBeaconPeriod::try_from_u8);
arb_raw!(Sfn);
arb_via!(Quality, u8, Quality::new);

impl kani::Arbitrary for PowerConst {
    fn any() -> Self {
        if kani::any() {
            PowerConst::Unconstrained
        } else {
            PowerConst::Constrained
        }
    }
}

// types::identifiers
arb_via!(LongRdId, u32, LongRdId::new);
arb_via!(ShortRdId, u16, ShortRdId::new);

// types::mac_frame
impl kani::Arbitrary for PacketLengthType {
    fn any() -> Self {
        PacketLengthType::from_bit(kani::any())
    }
}

// types::mac_security
arb_via!(KeyIndex, u8, KeyIndex::new);
arb_via!(SecurityIvType, u8, SecurityIvType::try_from_u8);
arb_via!(SecurityVersion, u8, SecurityVersion::try_from_u8);

// types::measurement
arb_via!(
    BroadcastFeedbackType,
    u8,
    BroadcastFeedbackType::try_from_u8
);
arb_via!(DlDataReception, u8, DlDataReception::try_from_u8);
arb_raw!(EndpointProtocol);
arb_via!(IndicationType, u8, IndicationType::try_from_u8);
arb_via!(
    RadioDeviceStatusDuration,
    u8,
    RadioDeviceStatusDuration::try_from_u8
);
arb_via!(
    RadioDeviceStatusFlag,
    u8,
    RadioDeviceStatusFlag::try_from_u8
);
arb_raw!(Rssi1Measurement);
arb_raw!(Rssi2Measurement);
arb_raw!(SnrMeasurement);
arb_via!(UlPeriod, u8, UlPeriod::try_from_u8);

// types::phy
arb_via!(AbsoluteChannel, u16, AbsoluteChannel::new);
arb_via!(Mu, u8, Mu::new);
arb_via!(NumHarqProcesses, u8, NumHarqProcesses::try_from_u8);
arb_via!(Nss, u8, Nss::try_from_u8);
arb_via!(RdClassBeta, u8, RdClassBeta::try_from_u8);
arb_via!(RdClassMu, u8, RdClassMu::try_from_u8);
arb_via!(RdPowerClass, u8, RdPowerClass::try_from_u8);
arb_via!(RxGain, u8, RxGain::try_from_u8);
arb_via!(SoftBufferSize, u8, SoftBufferSize::try_from_u8);
arb_via!(TransmitPower, u8, TransmitPower::new);

// types::resource
arb_via!(Cwsig, u8, Cwsig::new);
arb_via!(
    DectScheduledResourceFailure,
    u8,
    DectScheduledResourceFailure::try_from_u8
);
arb_via!(MaxRachLength, u8, MaxRachLength::new);
arb_via!(RaLength, u8, RaLength::new);
arb_via!(RachRepeatMode, u8, RachRepeatMode::try_from_u8);
arb_via!(RepeatMode, u8, RepeatMode::try_from_u8);
arb_via!(Repetition, u8, Repetition::new);
arb_via!(ResponseWindow, u8, ResponseWindow::new);
arb_raw!(Validity);

// types::routing_flow
arb_raw!(ApplicationSequenceNumber);
arb_via!(FlowEntry, u8, FlowEntry::try_from_raw);
arb_via!(FlowId, u8, FlowId::new);
arb_via!(RadioResourceChange, u8, RadioResourceChange::try_from_u8);
arb_via!(GroupId, u8, GroupId::new);
arb_via!(Hop, u8, Hop::new);
arb_via!(ResourceTag, u8, ResourceTag::new);
arb_raw!(RouteCost);
arb_via!(
    SourceRoutingValidityTimer,
    u8,
    SourceRoutingValidityTimer::try_from_u8
);

// Handwritten composite message types (all-pub fields).
arb_fields!(AllocationPair {
    start_subslot,
    length_type,
    length
});
arb_fields!(FtModeFields {
    network_beacon_period,
    cluster_beacon_period,
    next_cluster_channel,
    time_to_next,
    current_cluster_channel,
});
arb_fields!(RadioDeviceClass { mu, beta });
arb_fields!(RachRepeatPolicy {
    mode,
    repetition,
    validity
});
arb_fields!(RepeatPolicy {
    mode,
    repetition,
    validity
});
arb_fields!(AllocationOptions {
    add,
    recipient,
    repeat,
    sfn_value,
    channel,
    resource_failure_timer,
});
arb_fields!(PhyCapability {
    rd_power_class,
    max_nss_for_rx,
    rx_for_tx_diversity,
    rx_gain,
    max_mcs,
    soft_buffer_size,
    num_harq_processes,
    harq_feedback_delay,
});
arb_fields!(AdditionalPhyCapability { mu, beta, phy });
arb_fields!(HarqConfig { processes, max_re });
arb_fields!(HarqOverride {
    harq_processes_rx,
    max_harq_re_rx,
    harq_processes_tx,
    max_harq_re_tx,
});
arb_fields!(GroupAssignment {
    group_id,
    resource_tag
});
arb_fields!(AssociationAcceptParts {
    flow_acceptance,
    harq_override,
    group,
});

impl kani::Arbitrary for GroupResourceTagEntry {
    fn any() -> Self {
        GroupResourceTagEntry::new(kani::any(), kani::any())
    }
}

// Handwritten enums: a symbolic selector picks the variant, payloads
// are drawn recursively.
impl kani::Arbitrary for BroadcastRdId {
    fn any() -> Self {
        if kani::any() {
            BroadcastRdId::Short(kani::any())
        } else {
            BroadcastRdId::Long(kani::any())
        }
    }
}

impl kani::Arbitrary for ResourceAllocationKind {
    fn any() -> Self {
        match kani::any::<u8>() {
            0 => Self::ReleaseAll,
            1 => Self::Downlink {
                pair: kani::any(),
                options: kani::any(),
            },
            2 => Self::Uplink {
                pair: kani::any(),
                options: kani::any(),
            },
            _ => Self::Both {
                dl: kani::any(),
                ul: kani::any(),
                options: kani::any(),
            },
        }
    }
}

impl kani::Arbitrary for FlowAcceptance {
    fn any() -> Self {
        if kani::any() {
            FlowAcceptance::All
        } else {
            let n: usize = kani::any();
            kani::assume(n <= MAX_RESPONSE_FLOWS);
            let mut v = Vec::new();
            let mut i = 0;
            while i < n {
                let _ = v.push(kani::any());
                i += 1;
            }
            FlowAcceptance::Specific(v)
        }
    }
}

impl kani::Arbitrary for AssociationResponseParts {
    fn any() -> Self {
        if kani::any() {
            AssociationResponseParts::Reject {
                cause: kani::any(),
                timer: kani::any(),
            }
        } else {
            AssociationResponseParts::Accept(kani::any())
        }
    }
}
