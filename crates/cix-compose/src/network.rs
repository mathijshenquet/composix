//! Pod namespaces, egress leases, and host publication surfaces.

use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    directories::stable_hash,
    resolve::{CheckResult, CheckedService},
    unit_path,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedPod {
    pub path: String,
    pub egress: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedPublish {
    pub name: String,
    pub service: String,
    pub surface: String,
    pub address: SocketAddr,
    pub pod: String,
    pub kind: PublishKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublishKind {
    Listener,
    Port { target: u16 },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct PodLease {
    pub host: u16,
}

pub(crate) fn render_socket_unit(
    listener: &str,
    service: &str,
    target: &str,
    address: &SocketAddr,
) -> String {
    format!(
        "[Unit]\nDescription=cix compose listener: {listener} for {service}\nPartOf={target}\nBefore={service}\n\n[Socket]\nListenStream={address}\nFileDescriptorName={listener}\nService={service}\n"
    )
}

pub(crate) fn publish_socket_name(composite: &str, published: &CheckedPublish) -> String {
    format!(
        "cix-{composite}-publish-{}.socket",
        unit_path(&published.name)
    )
}

pub(crate) fn render_proxy_unit(
    name: &str,
    socket: &str,
    netns: &str,
    namespace: &str,
    target: &str,
    slice: &str,
    port: u16,
) -> String {
    format!(
        "[Unit]\nDescription=cix compose published port proxy: {name}\nPartOf={target}\nRequires={netns} {socket}\nAfter={netns} {socket}\nJoinsNamespaceOf={netns}\n\n[Service]\nType=notify\nSlice={slice}\nNetworkNamespacePath=/run/netns/{namespace}\nExecStart=/run/current-system/systemd/lib/systemd/systemd-socket-proxyd 127.0.0.1:{port}\nNoNewPrivileges=yes\nPrivateTmp=yes\nProtectSystem=strict\nProtectHome=yes\nRestrictAddressFamilies=AF_INET AF_INET6\n"
    )
}

pub(crate) fn render_netns_unit(
    path: &str,
    namespace: &str,
    target: &str,
    members: &[String],
    lease: Option<PodLease>,
    composite: &str,
) -> String {
    let description = if path.is_empty() { "root" } else { path };
    let before = if members.is_empty() {
        String::new()
    } else {
        format!("Before={}\n", members.join(" "))
    };
    let mut start = format!(
        "/run/current-system/sw/bin/mkdir -p /run/netns; if ! /run/current-system/sw/bin/ip netns list | /run/current-system/sw/bin/grep -q \"^{} \"; then /run/current-system/sw/bin/ip netns add {}; fi; /run/current-system/sw/bin/ip -n {} link set lo up",
        namespace, namespace, namespace
    );
    let mut after = String::new();
    let mut requires = String::new();
    if let Some(lease) = lease {
        let host = veth_name(composite, path, 'h');
        let peer = veth_name(composite, path, 'p');
        let address = pod_address(lease);
        start.push_str(&format!(
            "; n=0; while ! /run/current-system/sw/bin/ip link show cix0 >/dev/null 2>&1; do n=$((n + 1)); test $n -lt 100; /run/current-system/sw/bin/sleep 0.1; done; /run/current-system/sw/bin/ip link delete {host} 2>/dev/null || true; /run/current-system/sw/bin/ip link add {host} type veth peer name {peer}; /run/current-system/sw/bin/ip link set {peer} netns {namespace}; /run/current-system/sw/bin/ip -n {namespace} link set {peer} addrgenmode none; /run/current-system/sw/bin/ip -n {namespace} address replace {address}/16 dev {peer}; /run/current-system/sw/bin/ip -n {namespace} link set {peer} up; /run/current-system/sw/bin/ip link set {host} master cix0; /run/current-system/sw/bin/ip link set {host} up; /run/current-system/sw/bin/ip -n {namespace} route replace default via 10.231.0.1"
        ));
        after.push_str("After=systemd-networkd.service\n");
        requires.push_str("Requires=systemd-networkd.service\n");
    }
    format!(
        "[Unit]\nDescription=cix compose network namespace: {description}\nPartOf={target}\n{before}{requires}{after}\n[Service]\nType=oneshot\nRemainAfterExit=yes\nTimeoutStopSec=10s\nSlice={}.slice\nExecStart=/bin/sh -ec '{start}'\nExecStop=/bin/sh -ec '/run/current-system/sw/bin/ip link delete {} 2>/dev/null || true; /run/current-system/sw/bin/ip netns delete {namespace}'\n",
        target.trim_end_matches(".target"),
        veth_name(composite, path, 'h')
    )
}

pub(crate) fn namespace_name(composite: &str, path: &str) -> String {
    if path.is_empty() {
        format!("cix-{composite}-netns")
    } else {
        format!("cix-{composite}-{}-netns", filesystem_segment(path))
    }
}

pub(crate) fn netns_unit_name(composite: &str, path: &str) -> String {
    if path.is_empty() {
        format!("cix-{composite}-netns.service")
    } else {
        format!("cix-{composite}-{}-netns.service", unit_path(path))
    }
}

pub(crate) fn veth_name(composite: &str, path: &str, side: char) -> String {
    format!("cx{:08x}{side}", stable_hash(composite, path) as u32)
}

pub(crate) fn pod_address(lease: PodLease) -> std::net::Ipv4Addr {
    let [high, low] = lease.host.to_be_bytes();
    std::net::Ipv4Addr::new(10, 231, high, low)
}

pub(crate) fn default_leases(checked: &CheckResult) -> BTreeMap<String, PodLease> {
    checked
        .pods
        .iter()
        .filter(|(_, pod)| pod.egress)
        .enumerate()
        .map(|(index, (path, _))| {
            (
                path.clone(),
                PodLease {
                    host: u16::try_from(index + 2).expect("compose pod count fits IPv4 IPAM"),
                },
            )
        })
        .collect()
}

pub(crate) fn parent_path(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

pub(crate) fn filesystem_segment(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("_{byte:02x}"));
        }
        encoded
    })
}

pub(crate) fn parse_publish_binding(path: &str, name: &str, value: &str) -> Result<SocketAddr> {
    value.parse::<SocketAddr>().with_context(|| {
        format!(
            "children.{path}.bind.{name}: published binding must be an IP address and port such as 127.0.0.1:8080"
        )
    })
}

pub(crate) fn validate_collisions(
    services: &BTreeMap<String, CheckedService>,
    publishes: &[CheckedPublish],
) -> Result<()> {
    let mut ports: BTreeMap<(Option<&str>, u16), (&str, &str)> = BTreeMap::new();
    for (service_name, checked) in services {
        for (port_name, port) in &checked.config.ports {
            if let Some((other_service, other_port)) =
                ports.insert((checked.pod.as_deref(), *port), (service_name, port_name))
            {
                if other_service != service_name {
                    bail!(
                        "services.{service_name}: host port {port} for {port_name:?} collides with services.{other_service} port {other_port:?}"
                    );
                }
            }
        }
    }

    let mut bindings: Vec<(&str, &str, SocketAddr)> = Vec::new();
    for (service_name, checked) in services {
        for (listener, address) in &checked.config.listeners {
            for (other_service, other_listener, other_address) in &bindings {
                if service_name != other_service && addresses_collide(*address, *other_address) {
                    bail!(
                        "services.{service_name}.bind.{listener}: {address} collides with services.{other_service}.bind.{other_listener} ({other_address})"
                    );
                }
            }
            if let Some((other_service, other_port)) = ports.get(&(None, address.port())) {
                if *other_service != service_name {
                    bail!(
                        "services.{service_name}.bind.{listener}: {address} collides with services.{other_service} port {other_port:?}"
                    );
                }
            }
            bindings.push((service_name, listener, *address));
        }
    }
    for published in publishes {
        for (other_service, other_listener, other_address) in &bindings {
            if addresses_collide(published.address, *other_address) {
                bail!(
                    "publish.{}: {} collides with services.{other_service}.bind.{other_listener} ({other_address})",
                    published.name,
                    published.address
                );
            }
        }
        bindings.push((&published.service, &published.surface, published.address));
    }
    Ok(())
}

fn addresses_collide(left: SocketAddr, right: SocketAddr) -> bool {
    if left.port() != right.port() {
        return false;
    }
    match (left.ip(), right.ip()) {
        (IpAddr::V4(left), IpAddr::V4(right)) => {
            left == right || left.is_unspecified() || right.is_unspecified()
        }
        (IpAddr::V6(left), IpAddr::V6(right)) => {
            left == right || left.is_unspecified() || right.is_unspecified()
        }
        (left, right) => left.is_unspecified() || right.is_unspecified(),
    }
}
