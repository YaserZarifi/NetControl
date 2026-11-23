# Monitoring limitations

Interface totals measure all received/transmitted octets on one selected adapter, including local traffic. They are not ISP billing measurements, router-wide usage, or historical consumption from before collection began. Browser preview cannot measure Windows traffic or change firewall settings.

Automatic mode selects one connected interface and prefers hardware interfaces. It avoids summing physical and virtual traffic. Select a VPN interface manually when its coverage is wanted. Multiple simultaneously active physical interfaces are not fully covered by one selected interface.

The first reading establishes a baseline. Intervals spanning resets, interface changes, pause/reset generations, sleep gaps longer than five seconds, local midnight, or UTC-offset changes are discarded. Missing coverage is not estimated. Writes are batched every five samples; unexpected shutdown can lose the most recent seconds. Storage failures are reported, and pending memory is capped at 60 samples. Retention cleanup runs at startup and hourly.

Application accounting observes Windows kernel TCP/UDP send/receive events, including IPv4/IPv6 opcodes. Event sizes provide byte counts. Other protocols, unsupported or lost events, and differences between interface octets and transport-event semantics limit coverage. Retransmit events are not added separately. Application totals cover all interfaces while tracing is enabled and are stored separately from selected-interface totals.

System / Unattributed represents observed events with unresolved process identities. It does not estimate every unobserved byte. Protected or exited processes may hide executable paths. Creation time is compared with event time to reject reused PIDs. Decoding failures are reported; kernel event loss can still cause undercounting. No packet contents, destinations, or browsing history are stored.

Administrator permission is required for tracing and firewall changes. Denying it leaves interface monitoring available. Outbound restrictions apply to internet and local destinations. NetControl reports its own rule state; Windows Firewall must be enabled, and other policies can affect enforcement. Removing a NetControl rule does not guarantee other rules permit access. Rules persist after exit and uninstall. Remove unwanted restrictions before uninstalling or through Windows Firewall, using the `NetControl.Outbound.v1.` rule prefix.
