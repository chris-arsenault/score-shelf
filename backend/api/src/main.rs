use std::sync::Arc;

use lambda_http::{service_fn, Error, Request};

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let telemetry = ahara_lambda_telemetry::TelemetryConfig::new(api::SERVICE_NAME);
    ahara_lambda_telemetry::init_lambda_logging(&telemetry);
    tracing::info!("api starting");
    let state = Arc::new(
        api::ApiState::from_env()
            .await
            .map_err(|err| lambda_http::Error::from(err.to_string()))?,
    );
    ahara_lambda_telemetry::run_http_lambda(
        telemetry,
        service_fn(move |request: Request| {
            let state = Arc::clone(&state);
            async move { Ok::<_, Error>(api::handle_request(request, state).await) }
        }),
    )
    .await
}
