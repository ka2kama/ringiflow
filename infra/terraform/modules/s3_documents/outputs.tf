output "bucket_name" {
  description = "S3 バケット名"
  value       = aws_s3_bucket.documents.bucket
}

output "bucket_arn" {
  description = "S3 バケット ARN"
  value       = aws_s3_bucket.documents.arn
}
