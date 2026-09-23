module "frontend" {
  source = "git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/website"

  prefix         = local.prefix
  hostname       = local.frontend_hostname
  site_directory = "${path.module}/../../frontend/dist"

  runtime_config = {
    apiBaseUrl        = "https://${local.api_hostname}"
    cognitoUserPoolId = module.ctx.cognito.user_pool_id
    cognitoClientId   = module.cognito_app.client_id
  }
}
