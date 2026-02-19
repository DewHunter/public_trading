terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "6.32.1"
    }
  }
}

provider "aws" {
  region  = "us-west-2"
  profile = "jorgedev"
  default_tags {
    tags = {
      Stack = "public-dev"
    }
  }
}

resource "aws_cloudwatch_log_group" "public_service_log_group" {
  name              = "public_trading/service"
  retention_in_days = 180
}

resource "aws_cloudwatch_log_stream" "public_lg_stream_dellxps" {
  name           = "dellxpslaptop_server"
  log_group_name = aws_cloudwatch_log_group.public_service_log_group.name
}
