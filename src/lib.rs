use aws_sdk_s3;
use aws_sdk_sqs;

const MAX_MESSAGE_SIZE_IN_BYTES: u64 = 262144; // TODO - why is this a u64 when it's a literal const of 262,144?
static DEFAULT_POINTER_CLASS: &str = "software.amazon.payloadoffloading.PayloadS3Pointer";

pub struct SqsExtendedClient {
    s3_client: aws_sdk_s3::Client,      //
    sqs_client: aws_sdk_sqs::Client,    //
    bucket_name: String,                //
    message_size_threshold: u64,        //
    batch_messages_size_threshold: u64, //
    always_via_s3: bool,                //
    pointer_class: String,              //
    //reserved_attributes: []String, // TODO
    object_prefix: String,
    //base_s3_pointer_size: i32, // TODO int -> what should the default size golang's int be?
    //base_attribute_size: i32,  // TODO int -> what should the default size golang's int be?
}

impl SqsExtendedClient {
    pub fn builder(
        s3_client: aws_sdk_s3::Client,
        sqs_client: aws_sdk_sqs::Client,
        bucket_name: String,
    ) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder::new(s3_client, sqs_client, bucket_name)
    }

    pub fn send_message(&self) -> String {
        // TODO implement
        "Sent Message!".to_string()
    }

    pub fn always_s3_message(&self) -> String {
        // TODO delete
        "Always S3 is: ".to_string() + &self.always_via_s3.to_string()
    }
}

pub struct SqsExtendedClientBuilder {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    bucket_name: String,
    always_s3: bool,
    object_prefix: String,
}

impl SqsExtendedClientBuilder {
    pub fn new(
        s3_client: aws_sdk_s3::Client,
        sqs_client: aws_sdk_sqs::Client,
        bucket_name: String,
    ) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder {
            s3_client,
            sqs_client,
            bucket_name,
            always_s3: false,
            object_prefix: "".to_string(),
        }
    }

    pub fn always_via_s3(mut self) -> SqsExtendedClientBuilder {
        self.always_s3 = true;
        self
    }

    pub fn build(self) -> SqsExtendedClient {
        SqsExtendedClient {
            s3_client: self.s3_client,
            sqs_client: self.sqs_client,
            bucket_name: self.bucket_name,
            message_size_threshold: MAX_MESSAGE_SIZE_IN_BYTES,
            batch_messages_size_threshold: MAX_MESSAGE_SIZE_IN_BYTES,
            always_via_s3: self.always_s3,
            pointer_class: DEFAULT_POINTER_CLASS.to_string(),
            object_prefix: self.object_prefix,
        }
    }
}
