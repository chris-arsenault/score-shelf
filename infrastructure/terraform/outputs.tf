output "frontend_url" {
  value = module.frontend.url
}

output "api_url" {
  value = "https://${local.api_hostname}"
}

output "reserved_alb_priorities" {
  value = local.alb_priorities
}

output "files_bucket_name" {
  value = aws_s3_bucket.files.id
}

output "cognito_client_id" {
  value = module.cognito_app.client_id
}

output "api_function_name" {
  value = module.api.function_names["api"]
}

output "frontend_bucket_name" {
  value = module.frontend.bucket_name
}

output "frontend_distribution_id" {
  value = module.frontend.distribution_id
}
