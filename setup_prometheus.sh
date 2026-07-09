#!/bin/bash
set -euo pipefail

PROMETHEUS_VERSION="2.53.0"
TAILSCALE_IP="100.108.1.124"
PROMETHEUS_DIR="/etc/prometheus"
PROMETHEUS_DATA_DIR="/var/lib/prometheus"

echo "Creating prometheus user and group..."
if ! getent group prometheus >/dev/null; then
    groupadd --system prometheus
fi

if ! getent passwd prometheus >/dev/null; then
    useradd --system -g prometheus --no-create-home --shell /usr/sbin/nologin prometheus
fi

echo "Creating directories..."
mkdir -p "$PROMETHEUS_DIR" "$PROMETHEUS_DATA_DIR"
chown prometheus:prometheus "$PROMETHEUS_DIR" "$PROMETHEUS_DATA_DIR"

echo "Downloading Prometheus v${PROMETHEUS_VERSION}..."
cd /tmp
wget -qO prometheus.tar.gz "https://github.com/prometheus/prometheus/releases/download/v${PROMETHEUS_VERSION}/prometheus-${PROMETHEUS_VERSION}.linux-amd64.tar.gz"

echo "Extracting Prometheus..."
tar -xf prometheus.tar.gz
cd "prometheus-${PROMETHEUS_VERSION}.linux-amd64"

echo "Installing binaries..."
install -m 0755 -o prometheus -g prometheus prometheus promtool /usr/local/bin/

echo "Installing consoles and libraries..."
cp -a consoles console_libraries "$PROMETHEUS_DIR/"
chown -R prometheus:prometheus "$PROMETHEUS_DIR/consoles" "$PROMETHEUS_DIR/console_libraries"

echo "Creating configuration file..."
cat <<EOF > "${PROMETHEUS_DIR}/prometheus.yml"
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: "prometheus"
    static_configs:
      - targets: ["${TAILSCALE_IP}:9090"]
EOF
chown prometheus:prometheus "${PROMETHEUS_DIR}/prometheus.yml"
chmod 0644 "${PROMETHEUS_DIR}/prometheus.yml"

echo "Creating systemd unit file..."
cat <<EOF > /etc/systemd/system/prometheus.service
[Unit]
Description=Prometheus Time Series Collection and Processing Server
Wants=network-online.target
After=network-online.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/usr/local/bin/prometheus \\
  --config.file=${PROMETHEUS_DIR}/prometheus.yml \\
  --storage.tsdb.path=${PROMETHEUS_DATA_DIR} \\
  --web.console.templates=${PROMETHEUS_DIR}/consoles \\
  --web.console.libraries=${PROMETHEUS_DIR}/console_libraries \\
  --web.listen-address=${TAILSCALE_IP}:9090 \\
  --web.external-url=http://${TAILSCALE_IP}:9090/
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5s
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
chmod 0644 /etc/systemd/system/prometheus.service

echo "Reloading systemd, enabling and starting prometheus..."
systemctl daemon-reload
systemctl enable prometheus
systemctl restart prometheus

echo "Checking Prometheus status..."
systemctl status prometheus --no-pager || true

echo "Prometheus setup completed successfully!"
