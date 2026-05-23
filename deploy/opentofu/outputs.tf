output "bucket_name" {
  value = aws_s3_bucket.archive.bucket
}

output "host_public_ip" {
  value = aws_instance.host.public_ip
}

output "wireguard_endpoint" {
  value = "${aws_instance.host.public_ip}:${var.wireguard_port}"
}

output "wireguard_server_vpn_ip" {
  value = var.wireguard_server_vpn_ip
}

output "wireguard_client_config_template" {
  sensitive = true
  value = templatefile("${path.module}/../wireguard/client.conf.tmpl", {
    client_private_key = "<fill-client-private-key>"
    client_vpn_ip      = var.wireguard_client_vpn_ip
    server_public_key  = "<fill-server-public-key>"
    server_endpoint    = "${aws_instance.host.public_ip}:${var.wireguard_port}"
    server_vpn_ip      = var.wireguard_server_vpn_ip
  })
}
