# Deployment

This deployment is intentionally small:

- One AWS EC2 instance.
- One Elastic IP for a stable WireGuard endpoint.
- One encrypted EBS data volume.
- One private S3 bucket with lifecycle rules for originals.
- One WireGuard UDP ingress rule.
- Host-native WireGuard through `wg-quick@wg0`.
- Podman Quadlet units for the app, worker, Postgres, and backup job.

No public HTTP, HTTPS, SSH, or Postgres ingress is created.

## Flow

1. Generate a WireGuard keypair on the laptop.
2. Generate a WireGuard keypair for the server and expose both keys through ignored local environment variables.
3. Apply the infrastructure.
4. Let cloud-init install packages, configure WireGuard, mount the data volume, and enable the Quadlet units.
5. Connect WireGuard from the laptop and browse the app at the server VPN address.

The Elastic IP keeps the WireGuard endpoint stable when the EC2 host is rebuilt.

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

Review the plan for exactly one public ingress rule: WireGuard UDP on the configured port.

After applying, verify:

```sh
sudo wg-quick up ~/.config/cloud-image-storage/wireguard/photo-archive.conf
curl http://10.44.0.1:8080/healthz
```
