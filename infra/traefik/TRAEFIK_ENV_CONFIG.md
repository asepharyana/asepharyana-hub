# Traefik Environment Configuration

This document describes environment variables used to configure Traefik certificate and config paths in production deployments.

## Certificate Path Environment Variables

All certificate paths support environment variable substitution with safe fallback defaults. This allows flexible certificate management across different deployment environments without modifying compose files.

### Configuration Variables

| Variable                              | Description                                                     | Default Path                                  | Purpose                                                 |
| ------------------------------------- | --------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------- |
| `TRAEFIK_CONFIG_PATH`                 | Directory containing dynamic Traefik configuration files (YAML) | `/root/asepharyana-hub/infra/traefik/dynamic` | Location of middleware, router, and service definitions |
| `TRAEFIK_CERT_ASEPHARYANA_MY_ID_PEM`  | Certificate file for asepharyana.my.id                          | `/root/asepharyana.my.id.pem`                 | SSL/TLS certificate for asepharyana.my.id domain        |
| `TRAEFIK_CERT_ASEPHARYANA_MY_ID_KEY`  | Key file for asepharyana.my.id                                  | `/root/asepharyana.my.id.key`                 | SSL/TLS private key for asepharyana.my.id domain        |
| `TRAEFIK_CERT_ASEPHARYANA_WEB_ID_PEM` | Certificate file for asepharyana.web.id                         | `/root/asepharyana.web.id.pem`                | SSL/TLS certificate for asepharyana.web.id domain       |
| `TRAEFIK_CERT_ASEPHARYANA_WEB_ID_KEY` | Key file for asepharyana.web.id                                 | `/root/asepharyana.web.id.key`                | SSL/TLS private key for asepharyana.web.id domain       |

## Usage

### Default Behavior (Production)

If no environment variables are set, Traefik will use the default paths shown above. This is suitable for production deployments where certificates are installed at these standard locations.

```bash
docker compose -f infra/compose/traefik.yml up -d
```

### Custom Paths (Custom Deployments)

To override paths for a custom deployment, set environment variables before starting services:

```bash
export TRAEFIK_CONFIG_PATH=/etc/traefik/custom-dynamic

docker compose -f infra/compose/traefik.yml up -d
```

### Via .env File

Create or update your `.env` file in the deployment directory:

```env
TRAEFIK_CONFIG_PATH=/root/asepharyana-hub/infra/traefik/dynamic
TRAEFIK_CERT_ASEPHARYANA_MY_ID_PEM=/root/asepharyana.my.id.pem
TRAEFIK_CERT_ASEPHARYANA_MY_ID_KEY=/root/asepharyana.my.id.key
TRAEFIK_CERT_ASEPHARYANA_WEB_ID_PEM=/root/asepharyana.web.id.pem
TRAEFIK_CERT_ASEPHARYANA_WEB_ID_KEY=/root/asepharyana.web.id.key
```

Then deploy:

```bash
docker compose --env-file .env -f infra/compose/traefik.yml up -d
```

## Notes

- All certificate paths use read-only mounts (`:ro`) for security
- If a certificate file is missing at the specified path, Docker volume mounting will fail—ensure certificates exist before starting Traefik
- The dynamic configuration directory must contain valid YAML files for Traefik to load properly
