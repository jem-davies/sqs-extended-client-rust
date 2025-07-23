use aws_config::BehaviorVersion;
use aws_sdk_sqs::operation::send_message::builders::SendMessageFluentBuilder;
use sqs_extended_client_rust::{SqsExtendedClient, SqsExtendedClientBuilder};

#[::tokio::main]
async fn main() -> Result<(), aws_sdk_sqs::Error> {
    // what should this return be? -> guess it won't matter because this will go anyway...
    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);
    let clone_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();

    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
            .with_s3_bucket_name("".to_string())
            .with_always_through_s3(false)
            .with_message_size_threshold(200000)
            .build();

    let msg_input: SendMessageFluentBuilder = clone_sqs_client
        .send_message()
        .queue_url("")
        .message_body("HELLO SQS FROM RUST! :)");

    println!("{}", "HELLO FROM MAIN");

    sqs_extended_client.send_message(msg_input).await?;
    Ok(())
}
