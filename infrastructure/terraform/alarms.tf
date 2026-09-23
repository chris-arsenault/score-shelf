data "aws_ssm_parameter" "alarm_topic_arn" {
  name = "/ahara/alarms/sns-topic-arn"
}

locals {
  lambda_alarm_functions = {
    api = module.api.function_names["api"]
  }
}

resource "aws_cloudwatch_metric_alarm" "lambda_errors" {
  for_each = local.lambda_alarm_functions

  alarm_name          = "${local.prefix}-${each.key}-errors"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  evaluation_periods  = 1
  metric_name         = "Errors"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Sum"
  threshold           = 1
  treat_missing_data  = "notBreaching"
  alarm_actions       = [nonsensitive(data.aws_ssm_parameter.alarm_topic_arn.value)]
  ok_actions          = [nonsensitive(data.aws_ssm_parameter.alarm_topic_arn.value)]

  dimensions = {
    FunctionName = each.value
  }
}

resource "aws_cloudwatch_metric_alarm" "lambda_throttles" {
  for_each = local.lambda_alarm_functions

  alarm_name          = "${local.prefix}-${each.key}-throttles"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  evaluation_periods  = 1
  metric_name         = "Throttles"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Sum"
  threshold           = 1
  treat_missing_data  = "notBreaching"
  alarm_actions       = [nonsensitive(data.aws_ssm_parameter.alarm_topic_arn.value)]
  ok_actions          = [nonsensitive(data.aws_ssm_parameter.alarm_topic_arn.value)]

  dimensions = {
    FunctionName = each.value
  }
}
