use aws_config::BehaviorVersion;
use sqs_extended_client_rust::{SqsExtendedClient, SqsExtendedClientBuilder};

#[::tokio::main]
async fn main() {
    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);

    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client, "bucket_name".to_string())
            .always_via_s3()
            .build();

    println!("{}", sqs_extended_client.send_message());
    println!("{}", sqs_extended_client.always_s3_message());
}
