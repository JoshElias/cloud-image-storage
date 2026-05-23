data "aws_ami" "al2023_arm" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-*-arm64"]
  }

  filter {
    name   = "architecture"
    values = ["arm64"]
  }
}

resource "random_password" "postgres" {
  length  = 32
  special = false
}

locals {
  database_url = "postgres://cis:${random_password.postgres.result}@cis-postgres:5432/cis"

  app_config = <<-TOML
    [server]
    bind_address = "0.0.0.0:8080"
    public_base_url = "http://${var.wireguard_server_vpn_ip}:8080"

    [storage]
    bucket = "${var.bucket_name}"
    region = "${var.aws_region}"
    originals_prefix = "originals/"
    previews_prefix = "previews/"
    metadata_prefix = "metadata/raw/"

    [database]
    url = "${local.database_url}"
  TOML

  postgres_env = <<-ENV
    POSTGRES_PASSWORD=${random_password.postgres.result}
    DATABASE_URL=${local.database_url}
    CIS_BUCKET=${var.bucket_name}
  ENV

  wireguard_config = <<-CONF
    [Interface]
    Address = ${var.wireguard_server_vpn_ip}/24
    ListenPort = ${var.wireguard_port}
    PrivateKey = ${var.wireguard_server_private_key}

    [Peer]
    PublicKey = ${var.wireguard_client_public_key}
    AllowedIPs = ${var.wireguard_client_vpn_ip}/32
  CONF

  app_container = replace(
    file("${path.module}/../quadlet/app.container"),
    "ghcr.io/joshelias/cloud-image-storage:latest",
    var.container_image,
  )

  worker_container = replace(
    file("${path.module}/../quadlet/worker.container"),
    "ghcr.io/joshelias/cloud-image-storage:latest",
    var.container_image,
  )
}

resource "aws_s3_bucket" "archive" {
  bucket = var.bucket_name
}

resource "aws_s3_bucket_public_access_block" "archive" {
  bucket                  = aws_s3_bucket.archive.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "archive" {
  bucket = aws_s3_bucket.archive.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "archive" {
  bucket = aws_s3_bucket.archive.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "archive" {
  bucket = aws_s3_bucket.archive.id

  rule {
    id     = "originals-cold-storage"
    status = "Enabled"

    filter {
      and {
        prefix = "originals/"

        tags = {
          keep_hot = "false"
        }
      }
    }

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    transition {
      days          = 365
      storage_class = "DEEP_ARCHIVE"
    }
  }
}

resource "aws_iam_role" "host" {
  name = "${var.name}-host"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "host_archive" {
  name = "${var.name}-archive"
  role = aws_iam_role.host.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:RestoreObject",
          "s3:ListBucket",
          "s3:GetObjectTagging",
          "s3:PutObjectTagging"
        ]
        Resource = [
          aws_s3_bucket.archive.arn,
          "${aws_s3_bucket.archive.arn}/*"
        ]
      }
    ]
  })
}

resource "aws_iam_instance_profile" "host" {
  name = "${var.name}-host"
  role = aws_iam_role.host.name
}

resource "aws_security_group" "host" {
  name        = "${var.name}-host"
  description = "WireGuard-only ingress for cloud image storage"

  ingress {
    description = "WireGuard"
    from_port   = var.wireguard_port
    to_port     = var.wireguard_port
    protocol    = "udp"
    cidr_blocks = [var.allowed_wireguard_cidr]
  }

  egress {
    description = "Outbound internet access"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_instance" "host" {
  ami                         = data.aws_ami.al2023_arm.id
  instance_type               = var.instance_type
  iam_instance_profile        = aws_iam_instance_profile.host.name
  vpc_security_group_ids      = [aws_security_group.host.id]
  associate_public_ip_address = true
  user_data_replace_on_change = true
  user_data = templatefile("${path.module}/cloud-init.yaml.tftpl", {
    app_config                = local.app_config
    app_container             = local.app_container
    cis_network               = file("${path.module}/../quadlet/cis.network")
    postgres_backup_container = file("${path.module}/../quadlet/postgres-backup.container")
    postgres_backup_timer     = file("${path.module}/../quadlet/postgres-backup.timer")
    postgres_container        = file("${path.module}/../quadlet/postgres.container")
    postgres_env              = local.postgres_env
    wireguard_config          = local.wireguard_config
    worker_container          = local.worker_container
  })

  root_block_device {
    encrypted   = true
    volume_size = 30
  }

  tags = {
    Name = var.name
  }
}

resource "aws_ebs_volume" "data" {
  availability_zone = aws_instance.host.availability_zone
  encrypted         = true
  size              = var.volume_size_gb

  tags = {
    Name = "${var.name}-data"
  }
}

resource "aws_volume_attachment" "data" {
  device_name = "/dev/sdf"
  instance_id = aws_instance.host.id
  volume_id   = aws_ebs_volume.data.id
}
