use core::panic;
use std::collections::HashMap;

use aws_sdk_s3;
use aws_sdk_sqs;
use aws_sdk_sqs::operation::send_message::builders::SendMessageFluentBuilder;
use aws_sdk_sqs::operation::send_message::{SendMessageError, SendMessageOutput};
use aws_sdk_sqs::types::MessageAttributeValue;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use aws_smithy_runtime_api::client::result::SdkError;
use aws_smithy_types::byte_stream::ByteStream;
use uuid::Uuid;

const MAX_MESSAGE_SIZE_IN_BYTES: usize = 262144;
static DEFAULT_POINTER_CLASS: &str = "software.amazon.payloadoffloading.PayloadS3Pointer";
static LEGACY_RESERVED_ATTRIBUTE_NAME: &str = "SQSLargePayloadSize";

pub struct SqsExtendedClient {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    // logger here in Go implementation
    bucket_name: String,
    message_size_threshold: usize,
    batch_messages_size_threshold: usize,
    always_through_s3: bool,
    pointer_class: String,
    reserved_attributes: Vec<String>,
    object_prefix: String,
    base_s3_pointer_size: usize,
    base_attribute_size: usize,
}

impl SqsExtendedClient {
    pub async fn send_message(
        &self,
        msg_input: SendMessageFluentBuilder,
    ) -> Result<SendMessageOutput, SdkError<SendMessageError, HttpResponse>> {
        println!("{}", "Sending message!");
        if self.bucket_name == "" {
            panic!("you gotta set bucket_name for send message bro")
        }

        if self.always_through_s3
            || self.message_exceeds_threshold(
                &msg_input.get_message_body(),
                &msg_input.get_message_attributes(),
            )
        {
            // TODO refactor this block
            let s3_key: String = self.s3_key(Uuid::new_v4().to_string());

            let _ = self
                .s3_client
                .put_object()
                .bucket(&self.bucket_name)
                .key(&s3_key)
                .body(self.convert_string_byte_stream(msg_input.get_message_body().clone()))
                .send()
                .await;

            let new_msg: S3Pointer = S3Pointer {
                s3_bucket_name: self.bucket_name.clone(),
                s3_key: s3_key,
                class: self.pointer_class.clone(),
            };

            let xxx = &msg_input.get_message_body();
            let ss: usize;
            match xxx {
                // TODO can this be a if let?
                None => panic!(), // TODO error handling
                Some(string) => {
                    ss = string.len();
                }
            }

            let reserved_attribute = MessageAttributeValue::builder()
                .data_type("Number")
                .string_value(ss.to_string())
                .build()
                .expect("Failed to build MessageAttributeValue");

            msg_input
                .message_body(new_msg.marshall_json())
                .message_attributes(self.reserved_attributes[0].clone(), reserved_attribute)
                .send()
                .await
        } else {
            msg_input.send().await
        }
    }

    pub fn receive_message(&self) {
        panic!("NOT IMPLEMENTED");
    }

    fn message_exceeds_threshold(
        &self,
        body: &Option<String>,
        attributes: &Option<HashMap<String, MessageAttributeValue>>,
    ) -> bool {
        match body {
            None => false, // TODO -> error return
            Some(b) => self.message_size(b, attributes).total() > self.message_size_threshold,
        }
    }

    fn message_size(
        &self,
        body: &String,
        attributes: &Option<HashMap<String, MessageAttributeValue>>,
    ) -> MessageSize {
        MessageSize {
            body_size: body.len(),
            attribute_size: self.attribute_size(attributes),
        }
    }

    fn attribute_size(&self, attributes: &Option<HashMap<String, MessageAttributeValue>>) -> usize {
        match attributes {
            None => 0,
            Some(hash_map) => self.calc_attribute_size(hash_map),
        }
    }

    fn calc_attribute_size(&self, attributes: &HashMap<String, MessageAttributeValue>) -> usize {
        let mut sum: usize = 0;
        for (k, v) in attributes {
            sum = sum + k.len();
            print!("sum + k.len(): {}", sum);

            match &v.binary_value {
                None => {}
                Some(blob) => {
                    sum = sum + blob.as_ref().len();
                }
            }
            match &v.string_value {
                None => {}
                Some(string) => sum = sum + string.len(),
            }
            sum = sum + v.data_type.len();
        }
        sum
    }

    fn convert_string_byte_stream(&self, s: Option<String>) -> ByteStream {
        match s {
            None => panic!("HHENOCKE"), // TODO error handling
            Some(s) => ByteStream::from(s.into_bytes()),
        }
    }

    fn s3_key(&self, filename: String) -> String {
        if self.object_prefix != "" {
            return format!("{}/{}", self.object_prefix, filename);
        }
        filename
    }
}

pub struct SqsExtendedClientBuilder {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    bucket_name: String,
    message_size_threshold: usize,
    batch_message_size_threshold: usize,
    always_s3: bool,
    pointer_class: String,
    reserved_attributes: Vec<String>,
    object_prefix: String,
}

impl SqsExtendedClientBuilder {
    pub fn new(
        s3_client: aws_sdk_s3::Client,
        sqs_client: aws_sdk_sqs::Client,
    ) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder {
            s3_client,
            sqs_client,
            bucket_name: "".to_string(),
            message_size_threshold: MAX_MESSAGE_SIZE_IN_BYTES,
            batch_message_size_threshold: MAX_MESSAGE_SIZE_IN_BYTES,
            always_s3: false,
            pointer_class: DEFAULT_POINTER_CLASS.to_string(),
            reserved_attributes: vec![
                "ExtendedPayloadSize".to_string(),
                LEGACY_RESERVED_ATTRIBUTE_NAME.to_string(),
            ],
            object_prefix: "".to_string(),
        }
    }

    pub fn with_logger(self) -> SqsExtendedClientBuilder {
        panic!("NOT IMPLEMENTED");
    }

    pub fn with_s3_bucket_name(mut self, bucket_name: String) -> SqsExtendedClientBuilder {
        self.bucket_name = bucket_name;
        self
    }

    pub fn with_message_size_threshold(mut self, msg_size: usize) -> SqsExtendedClientBuilder {
        self.message_size_threshold = msg_size;
        self
    }

    pub fn with_batch_message_size_threshold(
        mut self,
        batch_msg_size: usize,
    ) -> SqsExtendedClientBuilder {
        self.batch_message_size_threshold = batch_msg_size;
        self
    }

    pub fn with_always_through_s3(mut self, always_s3: bool) -> SqsExtendedClientBuilder {
        self.always_s3 = always_s3;
        self
    }

    pub fn with_reserved_attribute_names(
        mut self,
        reserved_attribute_names: Vec<String>,
    ) -> SqsExtendedClientBuilder {
        self.reserved_attributes = reserved_attribute_names;
        self
    }

    pub fn with_pointer_class(mut self, pointer_class: String) -> SqsExtendedClientBuilder {
        self.pointer_class = pointer_class;
        self
    }

    pub fn with_object_prefix(mut self, prefix: String) -> SqsExtendedClientBuilder {
        self.object_prefix = prefix;
        self
    }

    pub fn build(self) -> SqsExtendedClient {
        let ptr: S3Pointer = S3Pointer {
            s3_bucket_name: String::from(""), // bucket name is blank here
            s3_key: Uuid::new_v4().to_string(),
            class: self.pointer_class.clone(),
        };

        let base_attribute_size: usize =
            self.reserved_attributes[0].len() + "Number".to_string().len();

        SqsExtendedClient {
            s3_client: self.s3_client,
            sqs_client: self.sqs_client,
            bucket_name: self.bucket_name,
            message_size_threshold: self.message_size_threshold,
            batch_messages_size_threshold: self.batch_message_size_threshold,
            always_through_s3: self.always_s3,
            pointer_class: self.pointer_class,
            reserved_attributes: self.reserved_attributes,
            object_prefix: self.object_prefix,
            base_s3_pointer_size: ptr.marshall_json().len(),
            base_attribute_size: base_attribute_size,
        }
    }
}

//------------------------------------------------------------------------------

struct S3Pointer {
    s3_bucket_name: String,
    s3_key: String,
    class: String,
}

impl S3Pointer {
    fn marshall_json(self) -> String {
        String::from(format!(
            "[\"{}\",{{\"s3BucketName\":\"{}\",\"s3Key\":\"{}\"}}]",
            self.class, self.s3_bucket_name, self.s3_key
        ))
    }
}

//------------------------------------------------------------------------------

struct MessageSize {
    body_size: usize,
    attribute_size: usize,
}

impl MessageSize {
    fn total(self) -> usize {
        self.body_size + self.attribute_size
    }
}

//------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;

    use super::*;

    fn make_test_credentials() -> aws_sdk_s3::config::Credentials {
        aws_sdk_s3::config::Credentials::new(
            "TEST_ACCESS_KEY_ID",
            "TEST_SECRET_ACCESS_KEY",
            Some("TEST_SESSION_TOKEN".to_string()),
            None,
            "",
        )
    }

    fn make_test_s3_client() -> aws_sdk_s3::client::Client {
        aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(make_test_credentials())
                .build(),
        )
    }

    fn make_test_sqs_client() -> aws_sdk_sqs::client::Client {
        aws_sdk_sqs::Client::from_conf(
            aws_sdk_sqs::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(make_test_credentials())
                .build(),
        )
    }

    #[test]
    fn test_builder_fns() {
        let sqs_extended_client: SqsExtendedClient =
            SqsExtendedClientBuilder::new(make_test_s3_client(), make_test_sqs_client())
                .with_s3_bucket_name("bucket-name".to_string())
                .with_message_size_threshold(9999)
                .with_batch_message_size_threshold(1000)
                .with_always_through_s3(true)
                .with_reserved_attribute_names(vec!["attr_one".to_string(), "attr_two".to_string()])
                .with_pointer_class("pointer-class".to_string())
                .with_object_prefix("object-prefix".to_string())
                .build();

        assert_eq!("bucket-name", sqs_extended_client.bucket_name);
        assert_eq!(9999, sqs_extended_client.message_size_threshold);
        assert_eq!(1000, sqs_extended_client.batch_messages_size_threshold);
        assert_eq!(true, sqs_extended_client.always_through_s3);
        assert_eq!(
            vec!["attr_one".to_string(), "attr_two".to_string()],
            sqs_extended_client.reserved_attributes
        );
        assert_eq!("pointer-class", sqs_extended_client.pointer_class);
        assert_eq!("object-prefix", sqs_extended_client.object_prefix);
        assert_eq!(84, sqs_extended_client.base_s3_pointer_size);
        assert_eq!(14, sqs_extended_client.base_attribute_size);
    }

    #[test]
    fn test_builder_defaults() {
        let sqs_extended_client: SqsExtendedClient =
            SqsExtendedClientBuilder::new(make_test_s3_client(), make_test_sqs_client()).build();

        assert_eq!("", sqs_extended_client.bucket_name);
        assert_eq!(
            MAX_MESSAGE_SIZE_IN_BYTES,
            sqs_extended_client.message_size_threshold
        );
        assert_eq!(
            MAX_MESSAGE_SIZE_IN_BYTES,
            sqs_extended_client.batch_messages_size_threshold
        );
        assert_eq!(false, sqs_extended_client.always_through_s3);
        assert_eq!(
            vec![
                "ExtendedPayloadSize".to_string(),
                LEGACY_RESERVED_ATTRIBUTE_NAME.to_string(),
            ],
            sqs_extended_client.reserved_attributes
        );
        assert_eq!(DEFAULT_POINTER_CLASS, sqs_extended_client.pointer_class);
        assert_eq!("", sqs_extended_client.object_prefix);
        assert_eq!(121, sqs_extended_client.base_s3_pointer_size);
        assert_eq!(25, sqs_extended_client.base_attribute_size);
    }

    #[test]
    fn test_calc_attribute_size() {
        let sqs_extended_client: SqsExtendedClient =
            SqsExtendedClientBuilder::new(make_test_s3_client(), make_test_sqs_client()).build();

        let reserved_attribute = MessageAttributeValue::builder()
            .data_type(String::from("String"))
            .string_value(String::from("some string"))
            .build()
            .expect("Failed to build MessageAttributeValue");

        let mut hm: HashMap<String, MessageAttributeValue> = HashMap::new();
        hm.insert(String::from("testing_strings"), reserved_attribute);

        assert_eq!(32, sqs_extended_client.calc_attribute_size(&hm))
    }

    #[test]
    fn test_s3_key() {
        let sqs_extended_client_no_prefix: SqsExtendedClient =
            SqsExtendedClientBuilder::new(make_test_s3_client(), make_test_sqs_client()).build();

        assert_eq!(
            String::from("00000000-0000-0000-0000-000000000000"),
            sqs_extended_client_no_prefix
                .s3_key(String::from("00000000-0000-0000-0000-000000000000"))
        );

        let sqs_extended_client_with_prefix: SqsExtendedClient =
            SqsExtendedClientBuilder::new(make_test_s3_client(), make_test_sqs_client())
                .with_object_prefix(String::from("some_prefix"))
                .build();

        assert_eq!(
            String::from("some_prefix/00000000-0000-0000-0000-000000000000"),
            sqs_extended_client_with_prefix
                .s3_key(String::from("00000000-0000-0000-0000-000000000000"))
        )
    }
}
