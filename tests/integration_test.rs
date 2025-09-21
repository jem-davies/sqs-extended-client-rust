use std::collections::HashMap;
use std::str::from_utf8;

use aws_config::{BehaviorVersion, Region, meta::region::RegionProviderChain};
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::operation::list_buckets::ListBucketsOutput;
use aws_sdk_sqs::operation::list_queues::ListQueuesOutput;
use aws_sdk_sqs::operation::receive_message::ReceiveMessageOutput;
use aws_sdk_sqs::types::Message;
use aws_sdk_sqs::{
    self,
    operation::{
        receive_message::builders::ReceiveMessageFluentBuilder,
        send_message::builders::SendMessageFluentBuilder,
    },
    types::MessageAttributeValue,
};
use sqs_extended_client::{SqsExtendedClient, SqsExtendedClientBuilder};
use testcontainers_modules::{
    localstack::{self, LocalStack},
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};

#[tokio::test]
async fn send_message_always_through_s3() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let (node, _endpoint_url, queue_url, s3_client, sqs_client) =
        create_localstack_with_bucket_and_queue().await?;

    let send_clone_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();
    let receive_clone_sqs_client: aws_sdk_sqs::Client = sqs_client.clone();
    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client.clone(), sqs_client)
            .with_s3_bucket_name("sqs-extended-client-bucket".to_string())
            .with_always_through_s3(true)
            .build();

    let msg_input: SendMessageFluentBuilder = send_clone_sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body("hello SQS! with love from the sqs-extended-client-rust 😊");

    sqs_extended_client.send_message(msg_input).await?;

    let receive_msg: ReceiveMessageFluentBuilder = receive_clone_sqs_client
        .receive_message()
        .queue_url(&queue_url);

    let response: ReceiveMessageOutput = sqs_extended_client.receive_message(receive_msg).await?;

    let msgs: Vec<Message> = response.messages.clone().unwrap_or_default();

    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].body.as_ref().unwrap(),
        "hello SQS! with love from the sqs-extended-client-rust 😊"
    );

    let reserved_attribute = MessageAttributeValue::builder()
        .data_type("Number")
        .string_value("59")
        .build()?;

    let attributes: HashMap<String, MessageAttributeValue> =
        msgs[0].message_attributes.clone().unwrap();

    let expected_attributes: HashMap<String, MessageAttributeValue> =
        HashMap::from([("ExtendedPayloadSize".to_string(), reserved_attribute)]);

    assert_eq!(attributes, expected_attributes);

    // Check the receipt handle - and use the values to check for the S3 Object!
    let receipt_handle = msgs[0].receipt_handle.clone().unwrap();

    let s3_bucket: &str = receipt_handle
        .split("-..s3BucketName..-")
        .nth(1)
        .unwrap()
        .split("-..s3BucketName..-")
        .next()
        .unwrap();
    let s3_key: &str = receipt_handle
        .split("-..s3Key..-")
        .nth(1)
        .unwrap()
        .split("-..s3Key..-")
        .next()
        .unwrap();

    // get the file contents from s3 as a string:
    let list_objects_output = s3_client
        .list_objects_v2()
        .bucket("sqs-extended-client-bucket")
        .send()
        .await?;

    let contents = list_objects_output.contents();

    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0].key().unwrap(), s3_key);

    let object: GetObjectOutput = s3_client
        .get_object()
        .bucket(s3_bucket)
        .key(s3_key)
        .send()
        .await?;

    let bytes = object.body.collect().await?.into_bytes();
    let response: &str = from_utf8(&bytes)?;

    assert_eq!(
        response,
        "hello SQS! with love from the sqs-extended-client-rust 😊"
    );

    let _rm = node.rm();

    Ok(())
}

async fn create_localstack_with_bucket_and_queue() -> Result<
    (
        ContainerAsync<LocalStack>,
        String,
        String,
        aws_sdk_s3::Client,
        aws_sdk_sqs::Client,
    ),
    Box<dyn std::error::Error + 'static>,
> {
    let node: ContainerAsync<LocalStack> = localstack::LocalStack::default()
        .with_env_var("SERVICES", "s3,sqs")
        .start()
        .await?;

    let host_ip = node.get_host().await?;
    let host_port: u16 = node.get_host_port_ipv4(4566).await?;
    let region_provider: RegionProviderChain = RegionProviderChain::first_try("us-east-1");
    let sqs_creds: aws_sdk_sqs::config::Credentials =
        aws_sdk_sqs::config::Credentials::new("fake", "fake", None, None, "test");
    let s3_creds: aws_sdk_s3::config::Credentials =
        aws_sdk_s3::config::Credentials::new("fake", "fake", None, None, "test");

    let endpoint_url: String = format!("http://{host_ip}:{host_port}");

    let sqs_config: aws_config::SdkConfig = aws_config::defaults(BehaviorVersion::v2025_01_17())
        .region(region_provider)
        .credentials_provider(sqs_creds)
        .endpoint_url(&endpoint_url)
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::default()
        .behavior_version(BehaviorVersion::v2025_01_17())
        .region(Region::new("us-east-1"))
        .credentials_provider(s3_creds)
        .endpoint_url(&endpoint_url)
        .force_path_style(true) // required to connect to localstack
        .build();

    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::from_conf(s3_config);

    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&sqs_config);

    sqs_client
        .create_queue()
        .queue_name("sqs-extended-client-queue")
        .send()
        .await?;

    s3_client
        .create_bucket()
        .bucket("sqs-extended-client-bucket")
        .send()
        .await?;

    let list_result: ListQueuesOutput =
        sqs_client.list_queues().send().await?;
    assert_eq!(list_result.queue_urls().len(), 1);

    let bucket_list: ListBucketsOutput =
        s3_client.list_buckets().send().await?;
    assert_eq!(bucket_list.buckets().len(), 1);

    let queue_url = list_result
        .queue_urls()
        .first()
        .expect("Queue URL should exist")
        .to_string();

    Ok((node, endpoint_url, queue_url, s3_client, sqs_client))
}
