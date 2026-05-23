variable "aws_region" {
  description = "AWS region for the archive."
  type        = string
  default     = "us-east-1"
}

variable "name" {
  description = "Name prefix for resources."
  type        = string
  default     = "cloud-image-storage"
}

variable "bucket_name" {
  description = "Globally unique S3 bucket name for image storage."
  type        = string
}

variable "instance_type" {
  description = "EC2 instance type for the Podman host."
  type        = string
  default     = "t4g.small"
}

variable "volume_size_gb" {
  description = "Encrypted EBS volume size for Postgres and app state."
  type        = number
  default     = 40
}

variable "wireguard_port" {
  description = "Public UDP port for WireGuard."
  type        = number
  default     = 51820
}

variable "wireguard_client_public_key" {
  description = "Laptop WireGuard public key."
  type        = string
  sensitive   = true
}

variable "wireguard_server_vpn_ip" {
  description = "WireGuard VPN IP assigned to the server."
  type        = string
  default     = "10.44.0.1"
}

variable "wireguard_client_vpn_ip" {
  description = "WireGuard VPN IP assigned to the laptop."
  type        = string
  default     = "10.44.0.2"
}

variable "allowed_wireguard_cidr" {
  description = "CIDR allowed to reach the WireGuard UDP port."
  type        = string
  default     = "0.0.0.0/0"
}
