# Deployment

This deployment is intentionally small:

- One AWS EC2 instance.
- One encrypted EBS data volume.
- One private S3 bucket with lifecycle rules for originals.
- One WireGuard UDP ingress rule.
- Podman Quadlet units for the app, worker, Postgres, backup job, and WireGuard setup.

No public HTTP, HTTPS, SSH, or Postgres ingress is created.

## Flow

1. Generate a WireGuard keypair on the laptop.
2. Pass the laptop public key into OpenTofu.
3. Apply the infrastructure.
4. Copy `deploy/quadlet/*.container` and `*.network` units to the host.
5. Enable the units with systemd.
6. Connect WireGuard from the laptop and browse the app at the server VPN address.

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
