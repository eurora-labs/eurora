#!/usr/bin/env bash
#
# Print the Mac's IPv4 on the active default-route interface.
#
# Used by `just ios-device` to bake a LAN-reachable host into the mobile
# binary so a physical iPhone on the same Wi-Fi can reach this Mac's dev
# servers — `localhost` references resolve to the iPhone itself on a
# real device, so every URL the binary embeds has to point at a real
# LAN IP instead.
#
# Bails with an actionable hint if the only IPv4 we can find is in the
# CLAT46 range (192.0.0.0/29). That address space is RFC 7335 — a
# customer-side translator synthesizes it locally on the Mac for
# outgoing IPv4-over-IPv6 traffic, which means it exists only on this
# machine and is unreachable from any other device. The classic trigger
# is iPhone Personal Hotspot on a 5G carrier with an IPv6-only uplink.

set -euo pipefail

iface=$(route -n get default 2>/dev/null | awk '/interface:/ {print $2; exit}')
if [ -z "${iface:-}" ]; then
    echo "lan-ip: no default route — connect to a network the iPhone can also reach." >&2
    exit 1
fi

ip=$(ipconfig getifaddr "$iface" 2>/dev/null || true)
if [ -z "$ip" ]; then
    echo "lan-ip: $iface has no IPv4 address — join a Wi-Fi network the iPhone can also reach." >&2
    exit 1
fi

case "$ip" in
    192.0.0.*)
        cat >&2 <<EOF
lan-ip: detected CLAT46 address ($ip on $iface).

The upstream link is IPv6-only — typically iPhone Personal Hotspot on a
5G carrier. $ip is synthesized locally on this Mac for outgoing IPv4
and is not reachable from the iPhone (or any other device).

Fix: switch off the hotspot and join a normal Wi-Fi network that both
the Mac and the iPhone can connect to.
EOF
        exit 1
        ;;
esac

echo "$ip"
