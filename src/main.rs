use aws_config::BehaviorVersion;
use aws_sdk_sqs::operation::send_message::builders::SendMessageFluentBuilder;
use sqs_extended_client::{SqsExtendedClient, SqsExtendedClientBuilder, SqsExtendedClientError};
use std::env; 

#[::tokio::main]
async fn main() -> Result<(), SqsExtendedClientError> {
    let sqs_queue_url: String = env::var("SQS_QUEUE_URL").unwrap();
    let s3_bucket_name: String = env::var("S3_BUCKET_NAME").unwrap();

    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);
    let clone_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();

    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
            .with_s3_bucket_name(s3_bucket_name)
            .with_always_through_s3(false)
            //.with_message_size_threshold(2)
            .build();

    let msg_input: SendMessageFluentBuilder = clone_sqs_client
        .send_message()
        .queue_url(sqs_queue_url)
        .message_body("HELLO SQS FROM RUST! :)");

    sqs_extended_client.send_message(msg_input).await?;
    Ok(())
}
