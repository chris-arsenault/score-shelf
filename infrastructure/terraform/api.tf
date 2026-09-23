module "api" {
  source   = "git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/alb-api"
  prefix   = local.prefix
  hostname = local.api_hostname

  vpc     = module.ctx.vpc
  alb     = module.ctx.alb
  cognito = module.ctx.cognito

  environment = local.api_env

  iam_policy = [jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect   = "Allow"
        Action   = ["s3:GetObject", "s3:PutObject"]
        Resource = "${aws_s3_bucket.files.arn}/pieces/*"
      },
      {
        Effect   = "Allow"
        Action   = ["s3:ListBucket"]
        Resource = aws_s3_bucket.files.arn
        Condition = {
          StringLike = { "s3:prefix" = ["pieces/*"] }
        }
      }
    ]
  })]

  lambdas = {
    api = {
      binary = "${path.module}/../../backend/target/lambda/api/bootstrap"
      routes = [
        {
          priority      = local.alb_priorities.api_health
          paths         = ["/health"]
          methods       = ["GET", "HEAD"]
          authenticated = false
        },
        {
          priority      = local.alb_priorities.api_authenticated
          paths         = ["/*"]
          authenticated = true
        }
      ]
      tracing_mode                   = "Active"
      reserved_concurrent_executions = 5
    }
  }
}
