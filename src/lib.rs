use core::panic;
use std::collections::HashMap;
use std::{fmt, rc};

use aws_sdk_s3;
use aws_sdk_s3::operation::put_object::{PutObjectError, PutObjectOutput};
use aws_sdk_sqs;
use aws_sdk_sqs::operation::receive_message::ReceiveMessageError;
use aws_sdk_sqs::operation::send_message::builders::SendMessageFluentBuilder;
use aws_sdk_sqs::operation::send_message::{SendMessageError, SendMessageOutput};
use aws_sdk_sqs::types::MessageAttributeValue;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use aws_smithy_runtime_api::client::result::SdkError;
use aws_smithy_types::byte_stream::ByteStream;
use aws_smithy_types::error::operation::BuildError;
use uuid::Uuid;

const MAX_MESSAGE_SIZE_IN_BYTES: usize = 262144;
static DEFAULT_POINTER_CLASS: &str = "software.amazon.payloadoffloading.PayloadS3Pointer";
static LEGACY_RESERVED_ATTRIBUTE_NAME: &str = "SQSLargePayloadSize";

pub struct SqsExtendedClient {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    // logger here in Go implementation
    bucket_name: Option<String>,
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
    ) -> Result<SendMessageOutput, SqsExtendedClientError> {
        let bucket_name: String;
        match &self.bucket_name { // replace with a let-else statement
            None => return Err(SqsExtendedClientError::NoBucketName),
            Some(bn) => bucket_name = bn.to_string(),
        }

        let message_body: &str;
        match msg_input.get_message_body() { // replace with a let-else statement
            None => return Err(SqsExtendedClientError::NoMessageBody),
            Some(msg_bdy) => message_body = msg_bdy,
        }

        let result = if self.always_through_s3
            || self.message_exceeds_threshold(message_body, &msg_input.get_message_attributes())
        {
            let s3_key: String = self.s3_key(Uuid::new_v4().to_string());

            let s3_result: Result<PutObjectOutput, SdkError<PutObjectError, HttpResponse>> = self
                .s3_client
                .put_object()
                .bucket(&bucket_name)
                .key(&s3_key)
                .body(self.convert_string_byte_stream(message_body))
                .send()
                .await;

            if let Err(s3_error) = s3_result {
                return Err(SqsExtendedClientError::S3Upload(s3_error));
            }

            let new_msg: S3Pointer = S3Pointer {
                s3_bucket_name: bucket_name,
                s3_key: s3_key,
                class: self.pointer_class.clone(),
            };

            let message_body_size: usize = message_body.len();

            let reserved_attribute = MessageAttributeValue::builder()
                .data_type("Number")
                .string_value(message_body_size.to_string())
                .build()?;

            msg_input
                .message_body(new_msg.marshall_json())
                .message_attributes(self.reserved_attributes[0].clone(), reserved_attribute)
                .send()
                .await
        } else {
            msg_input.send().await
        };

        result.map_err(|sqs_error| SqsExtendedClientError::SqsSendMessage(sqs_error))
    }

    pub async fn receive_message(&self, queue_url: &String) -> Result<(), SqsExtendedClientError> {
        let rcv_message_output = self.sqs_client.receive_message().queue_url(queue_url).send().await;
        
        let foo = match rcv_message_output {
            Ok(result) => result,
            Err(err) => return Err(SqsExtendedClientError::SqsReceiveMessage(err))
        };

        println!("Messages from queue with url: {}", queue_url);

        for message in foo.messages.unwrap_or_default() {
            println!("Got the message: {:#?}", message);
            while let Some(attr) = message.clone().attributes {
                println!("here");
                println!("Attr {:#?}", attr);
            }
        }
    
        Ok(())
    }

    pub async fn delete_message(&self) -> Result<(), SqsExtendedClientError> {
        panic!("NOT IMPLEMENTED");
    }

    fn message_exceeds_threshold(
        &self,
        body: &str,
        attributes: &Option<HashMap<String, MessageAttributeValue>>,
    ) -> bool {
        self.message_size(body, attributes).total() > self.message_size_threshold
    }

    fn message_size(
        &self,
        body: &str,
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

    fn convert_string_byte_stream(&self, s: &str) -> ByteStream {
        ByteStream::from(s.as_bytes().to_vec())
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
    bucket_name: Option<String>,
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
            bucket_name: None,
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
        self.bucket_name = Some(bucket_name);
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
            s3_bucket_name: String::from(""),
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

#[derive(Debug)]
pub enum SqsExtendedClientError {
    S3Upload(SdkError<PutObjectError, HttpResponse>),
    SqsSendMessage(SdkError<SendMessageError, HttpResponse>),
    SqsReceiveMessage(SdkError<ReceiveMessageError, HttpResponse>),
    SqsBuildMessageAttribute(BuildError),
    NoBucketName,
    NoMessageBody,
}

impl fmt::Display for SqsExtendedClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::S3Upload(err) => write!(f, "S3 upload failed: {}", err),
            Self::SqsSendMessage(err) => write!(f, "SQS operation failed: {}", err),
            Self::SqsReceiveMessage(err) => write!(f, "SQS operation failed: {}", err),
            Self::SqsBuildMessageAttribute(err ) => write!(f, "SQS build message attribute failed: {}", err),
            Self::NoBucketName => write!(f, "No bucket name configured"),
            Self::NoMessageBody => write!(f, "No message body provided"),
        }
    }
}

impl From<aws_sdk_s3::error::BuildError> for SqsExtendedClientError {
    fn from(err: aws_sdk_s3::error::BuildError) -> Self {
        Self::SqsBuildMessageAttribute(err)
    }
}

impl std::error::Error for SqsExtendedClientError {}
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

        let bucket_name: String;
        match sqs_extended_client.bucket_name {
            None => {
                bucket_name = String::from("");
            }
            Some(bn) => {
                bucket_name = bn;
            }
        }

        assert_eq!("bucket-name", bucket_name);
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

        let bucket_name: String;
        match sqs_extended_client.bucket_name {
            None => {
                bucket_name = String::from("");
            }
            Some(bn) => {
                bucket_name = bn;
            }
        }

        assert_eq!("", bucket_name);
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
