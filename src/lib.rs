use aws_sdk_s3;
use aws_sdk_sqs;

const MAX_MESSAGE_SIZE_IN_BYTES: u64 = 262144; // TODO - why is this a u64 when it's a literal const of 262,144?
static DEFAULT_POINTER_CLASS: &str = "software.amazon.payloadoffloading.PayloadS3Pointer"; // TODO - should this be a static? 

pub struct SqsExtendedClient {
    s3_client: aws_sdk_s3::Client,   //
    sqs_client: aws_sdk_sqs::Client, //
    // logger here in Go implementation
    bucket_name: String,                //
    message_size_threshold: u64,        //
    batch_messages_size_threshold: u64, //
    always_through_s3: bool,            //
    pointer_class: String,              //
    reserved_attributes: Vec<String>,   //
    object_prefix: String,
    base_s3_pointer_size: i32, // TODO int -> what should the default size golang's int be?
    base_attribute_size: i32,  // TODO int -> what should the default size golang's int be?
}

impl SqsExtendedClient {
    pub fn builder(
        s3_client: aws_sdk_s3::Client,
        sqs_client: aws_sdk_sqs::Client,
    ) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
    }

    pub fn send_message(&self) -> String {
        // TODO implement
        "Sent Message!".to_string()
    }

    pub fn receive_message(&self) {
        panic!("NOT IMPLEMENTED")
    }
}

pub struct SqsExtendedClientBuilder {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    bucket_name: String,
    message_size_threshold: u64,
    batch_message_size_threshold: u64,
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
            reserved_attributes: Vec::new(),
            object_prefix: "".to_string(),
        }
    }

    pub fn with_logger(mut self) -> SqsExtendedClientBuilder {
        panic!("NOT IMPLEMENTED");
    }

    pub fn with_s3_bucket_name(mut self, bucket_name: String) -> SqsExtendedClientBuilder {
        self.bucket_name = bucket_name;
        self
    }

    pub fn with_message_size_threshold(mut self, msg_size: u64) -> SqsExtendedClientBuilder {
        self.message_size_threshold = msg_size;
        self
    }

    pub fn with_batch_message_size_threshold(
        mut self,
        batch_msg_size: u64,
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
        // TODO set base_s3_pointer_size & base_attribute_size

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
            base_s3_pointer_size: 0, // Go default/zero value here?
            base_attribute_size: 0,  // Go default/zero value here?
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_with_s3_bucket_name() {
//         let s3_client_mock
//         let sqs_client_mock
//         let sqs_extended_client: SqsExtendedClient = SqsExtendedClientBuilder::new()
//     }
// }
