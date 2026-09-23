data "aws_ssm_parameter" "db_username" {
  name = "/ahara/db/score-shelf/username"
}

data "aws_ssm_parameter" "db_password" {
  name = "/ahara/db/score-shelf/password"
}

data "aws_ssm_parameter" "db_database" {
  name = "/ahara/db/score-shelf/database"
}

data "aws_ssm_parameter" "observability_otlp_http_endpoint" {
  name = "/ahara/observability/otlp-http-endpoint"
}

# Written by ahara-infra (services/score-shelf-publisher.tf).
data "aws_ssm_parameter" "publisher_client_id" {
  name = "/ahara/score-shelf/publisher-client-id"
}

data "aws_ssm_parameter" "publisher_scope" {
  name = "/ahara/score-shelf/publisher-scope"
}
