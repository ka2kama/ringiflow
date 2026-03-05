provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "ringiflow"
      Environment = "dev"
      ManagedBy   = "terraform"
    }
  }
}
