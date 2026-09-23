locals {
  aws_region  = "us-east-1"
  prefix      = "score-shelf"
  domain_name = "ahara.io"

  frontend_hostname = "${local.prefix}.${local.domain_name}"
  api_hostname      = "api.${local.prefix}.${local.domain_name}"

  files_bucket = "${local.prefix}-files"

  db_env = {
    DB_HOST     = module.ctx.rds.address
    DB_PORT     = module.ctx.rds.port
    DB_NAME     = nonsensitive(data.aws_ssm_parameter.db_database.value)
    DB_USERNAME = nonsensitive(data.aws_ssm_parameter.db_username.value)
    DB_PASSWORD = nonsensitive(data.aws_ssm_parameter.db_password.value)
  }

  otel_env = {
    OTEL_EXPORTER_OTLP_ENDPOINT = nonsensitive(data.aws_ssm_parameter.observability_otlp_http_endpoint.value)
    OTEL_LOGS_EXPORTER          = "otlp"
    OTEL_METRICS_EXPORTER       = "otlp"
    OTEL_TRACES_EXPORTER        = "otlp"
  }

  api_env = merge(local.db_env, local.otel_env, {
    COGNITO_CLIENT_ID      = module.cognito_app.client_id
    PUBLISHER_CLIENT_ID    = nonsensitive(data.aws_ssm_parameter.publisher_client_id.value)
    PUBLISHER_SCOPE        = nonsensitive(data.aws_ssm_parameter.publisher_scope.value)
    FILES_BUCKET           = aws_s3_bucket.files.id
    DEPLOYMENT_ENVIRONMENT = "production"
  })

  # Registered in ahara/INTEGRATION.md's listener priority table.
  alb_priorities = {
    api_health        = 260
    api_authenticated = 261
  }

  default_tags = {
    Project   = local.prefix
    ManagedBy = "Terraform"
  }
}
