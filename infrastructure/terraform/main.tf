terraform {
  required_version = ">= 1.12"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }

  backend "s3" {
    region       = "us-east-1"
    key          = "projects/score-shelf.tfstate"
    encrypt      = true
    use_lockfile = true
  }
}

provider "aws" {
  region = local.aws_region

  default_tags {
    tags = local.default_tags
  }
}
