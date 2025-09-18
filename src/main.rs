use aws_config::BehaviorVersion;
use aws_sdk_sqs::operation::{receive_message::builders::ReceiveMessageFluentBuilder, send_message::builders::SendMessageFluentBuilder};
use sqs_extended_client::{SqsExtendedClient, SqsExtendedClientBuilder, SqsExtendedClientError};
use std::env;

#[::tokio::main]
async fn main() -> Result<(), SqsExtendedClientError> {
    let sqs_queue_url: String = env::var("SQS_QUEUE_URL").unwrap();
    let s3_bucket_name: String = env::var("S3_BUCKET_NAME").unwrap();

    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);

    // TODO - get rid of this sqs_client cloning ...
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);
    let sender_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();
    let receiver_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();

    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
            .with_s3_bucket_name(s3_bucket_name)
            .with_always_through_s3(true)
            .with_message_size_threshold(2)
            .build();

    // let large_message: String = "X".repeat(12 * 1024 * 1024);
    let large_message: String = "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string();
    let msg_input: SendMessageFluentBuilder = sender_sqs_client
        .send_message()
        .queue_url(&sqs_queue_url)
        .message_body(large_message);

    sqs_extended_client.send_message(msg_input).await?;

    let receive_msg: ReceiveMessageFluentBuilder = receiver_sqs_client.receive_message().queue_url(sqs_queue_url);

    let rcv:aws_sdk_sqs::operation::receive_message::ReceiveMessageOutput = sqs_extended_client.receive_message(receive_msg).await?;

    for message in rcv.messages.unwrap_or_default() {
        println!("Got the message: {:#?}", message);
    }

    Ok(())
}
