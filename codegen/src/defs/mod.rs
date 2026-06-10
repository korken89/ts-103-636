//! Message layout definitions, one module per message. Each
//! definition mirrors its ETSI spec figure row by row; review these
//! files side by side with the standard.

mod association_control;
mod association_request;
mod association_response;
mod association_release;
mod broadcast_indication;
mod cluster_beacon;
mod group_assignment;
mod joining_beacon;
mod joining_information;
mod load_info;
mod mac_security_info;
mod measurement_report;
mod neighbouring;
mod network_beacon;
mod radio_device_status;
mod random_access_resource;
mod rd_capability;
mod reconfiguration_request;
mod resource_allocation;
mod reconfiguration_response;
mod rd_capability_short;
mod route_info;
mod source_routing;

use crate::ir::MessageDef;

/// Every message the generator emits.
pub fn all() -> Vec<MessageDef> {
    vec![
        association_control::def(),
        association_request::def(),
        association_response::def(),
        association_release::def(),
        broadcast_indication::def(),
        cluster_beacon::def(),
        group_assignment::def(),
        joining_beacon::def(),
        joining_information::def(),
        load_info::def(),
        mac_security_info::def(),
        measurement_report::def(),
        neighbouring::def(),
        network_beacon::def(),
        radio_device_status::def(),
        random_access_resource::def(),
        rd_capability::def(),
        reconfiguration_request::def(),
        resource_allocation::def(),
        reconfiguration_response::def(),
        rd_capability_short::def(),
        route_info::def(),
        source_routing::def(),
    ]
}
