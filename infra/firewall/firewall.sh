#!/bin/bash
# ============================================================
# firewall.sh — deny-by-default firewall untuk orangevps
# Public: 22 (SSH), 80/443 (Caddy), 4013 (hermes dashboard)
#         25565 (Minecraft) — WHITELIST TCPShield proxy only
# Tailscale CGNAT 100.64/10: semua port (imrnes & node lain)
# Localhost: semua
# Sisanya: DROP + log
# ============================================================
# TCPShield proxy ranges (https://tcpshield.com/v4/ + /v4-cf/)
# Update saat TCPShield publish range baru.
TCPSHIELD_V4=(
  198.178.119.0/24
  104.234.6.0/24
)
TCPSHIELD_V4_CF=(
  89.222.122.36/31
  152.233.22.8/31
  89.222.108.246/31
  84.17.55.186/31
  51.79.45.52/31
  5.135.84.92/30
  51.75.35.44/30
  51.161.27.110/31
  152.233.30.16/31
  152.233.30.232/31
  203.205.31.160/31
)
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
# Public: Minecraft (FTB sky) — hanya dari proxy TCPShield
for cidr in "${TCPSHIELD_V4[@]}" "${TCPSHIELD_V4_CF[@]}"; do
  iptables -A INPUT -s "$cidr" -p tcp --dport 25565 -j ACCEPT
done

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
# Minecraft 25565: TCPShield IPv4 only — tidak ada range IPv6 publik
ip6tables -A INPUT -p icmpv6 -j ACCEPT
ip6tables -A INPUT -m limit --limit 5/min --limit-burst 10 -j LOG --log-prefix "FW6-DROP " --log-level 4
ip6tables -A INPUT -j DROP

echo "Firewall applied:"
iptables -L INPUT -n --line-numbers | head -24
