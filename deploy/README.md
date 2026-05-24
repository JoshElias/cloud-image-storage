# Deployment

This deployment is intentionally small:

- One AWS EC2 instance.
- One Elastic IP for a stable WireGuard endpoint.
- One encrypted EBS data volume.
- One private S3 bucket with lifecycle rules for originals.
- Public HTTP/HTTPS ingress for the photo portal.
- One WireGuard UDP ingress rule for host administration.
- Host-native WireGuard through `wg-quick@wg0`.
- Podman Quadlet units for Caddy, the app, worker, Postgres, and backup job.

No public SSH or Postgres ingress is created.

## Flow

1. Generate a WireGuard keypair on the laptop.
2. Generate a WireGuard keypair for the server and expose both keys through ignored local environment variables.
3. Generate an admin password hash with `cis auth hash-password --password "replace-me"`.
4. Set `portal_domain_name` to the public subdomain you will use.
5. Apply the infrastructure.
6. Point the portal domain at the Elastic IP output.
7. Let Caddy issue the TLS certificate and browse the portal over HTTPS.

The Elastic IP keeps both the public HTTPS portal and WireGuard endpoint stable when the EC2 host is rebuilt.

The app and worker containers pull `ghcr.io/joshelias/cloud-image-storage:latest` by default. The package must be public, or the host will need registry credentials.

## Local WireGuard Client

Use the `client.conf.tmpl` output as a starting point:

```sh
sudo wg-quick up ~/.config/cloud-image-storage/wireguard/photo-archive.conf
```

When done:

```sh
sudo wg-quick down ~/.config/cloud-image-storage/wireguard/photo-archive.conf
```

## Verification

Run before applying:

```sh
tofu -chdir=deploy/opentofu fmt -check
tofu -chdir=deploy/opentofu validate
tofu -chdir=deploy/opentofu plan
```

Review the plan for exactly three public ingress rules: HTTP 80, HTTPS 443, and WireGuard UDP on the configured port.

After applying, verify:

```sh
sudo wg-quick up ~/.config/cloud-image-storage/wireguard/photo-archive.conf
curl http://10.44.0.1:8080/healthz
curl https://photos.example.com/healthz
```
