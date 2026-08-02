#!/bin/bash
# ============================================================
# firewall.sh — deny-by-default firewall untuk orangevps
# Public: 22 (SSH), 80/443 (Caddy), 4013 (hermes dashboard)
# Tailscale CGNAT 100.64/10: semua port (imrnes & node lain)
# Localhost: semua
# Sisanya: DROP + log
# ============================================================
set -e

### IPv4 ###
iptables -F
iptables -X
iptables -Z

# Policy default DROP
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT ACCEPT

# Loopback
iptables -A INPUT -i lo -j ACCEPT

# Established/related
iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

# Tailscale overlay (100.64.0.0/10) — imrnes & peers
iptables -A INPUT -s 100.64.0.0/10 -j ACCEPT

# Public: SSH, HTTP(S)
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
# Public: hermes dashboard (auth-protected)
iptables -A INPUT -p tcp --dport 4013 -j ACCEPT

# ICMP (ping, PMTU)
iptables -A INPUT -p icmp --icmp-type echo-request -m limit --limit 5/sec --limit-burst 10 -j ACCEPT
iptables -A INPUT -p icmp -m conntrack --ctstate NEW -j ACCEPT
iptables -A INPUT -p icmp -j ACCEPT

# Log dropped (rate-limited, 1 baris/5s)
iptables -A INPUT -m limit --limit 5/min --limit-burst 10 -j LOG --log-prefix "FW-DROP " --log-level 4
iptables -A INPUT -j DROP

# UFW chains (dipanggil dari ts-input) — kosongkan
iptables -F ufw-before-input 2>/dev/null || true
iptables -F ufw-after-input 2>/dev/null || true
iptables -F ufw-before-logging-input 2>/dev/null || true
iptables -F ufw-after-logging-input 2>/dev/null || true
iptables -F ufw-reject-input 2>/dev/null || true
iptables -F ufw-track-input 2>/dev/null || true

### IPv6 ###
ip6tables -F
ip6tables -X
ip6tables -Z
ip6tables -P INPUT DROP
ip6tables -P FORWARD DROP
ip6tables -P OUTPUT ACCEPT
ip6tables -A INPUT -i lo -j ACCEPT
ip6tables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
# Tailscale IPv6 ULA (fd7a:115c::/48)
ip6tables -A INPUT -s fd7a:115c::/48 -j ACCEPT
ip6tables -A INPUT -p tcp --dport 22 -j ACCEPT
ip6tables -A INPUT -p tcp --dport 80 -j ACCEPT
ip6tables -A INPUT -p tcp --dport 443 -j ACCEPT
ip6tables -A INPUT -p tcp --dport 4013 -j ACCEPT
# ICMPv6/MLD: ping + neighbor discovery (NIC multicast ff02::1 = MLDv2 reports
# dari host lain; kena LOG+DROP tiap menit — 1800 baris/6h di journal).
# IPv6 layer-2 discovery WAJIB di-ACCEPT, bukan cuma dropped.
ip6tables -A INPUT -p icmpv6 -j ACCEPT
ip6tables -A INPUT -d ff02::1 -j ACCEPT
ip6tables -A INPUT -d ff02::2 -j ACCEPT
ip6tables -A INPUT -d ff02::fb -j ACCEPT
ip6tables -A INPUT -m limit --limit 5/min --limit-burst 10 -j LOG --log-prefix "FW6-DROP " --log-level 4
ip6tables -A INPUT -j DROP

echo "Firewall applied:"
iptables -L INPUT -n --line-numbers | head -20
