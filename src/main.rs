use aws_config::BehaviorVersion;
use sqs_extended_client_rust::{SqsExtendedClient, SqsExtendedClientBuilder};

#[::tokio::main]
async fn main() {
    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);

    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
            .with_s3_bucket_name("bucket_name".to_string())
            .with_always_through_s3(true)
            .with_message_size_threshold(20)
            .build();

    println!("{}", sqs_extended_client.send_message());
}
