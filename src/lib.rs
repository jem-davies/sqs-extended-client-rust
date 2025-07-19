use aws_sdk_s3;
use aws_sdk_sqs;

pub struct SqsExtendedClient {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    always_s3: bool
}

impl SqsExtendedClient {
    // pub fn new(s3_client: aws_sdk_s3::Client, sqs_client: aws_sdk_sqs::Client, always_s3: bool) -> SqsExtendedClient {
    //     SqsExtendedClient{
    //         s3_client,
    //         sqs_client,
    //         always_s3,
    //     }
    // }

    pub fn builder(s3_client: aws_sdk_s3::Client, sqs_client: aws_sdk_sqs::Client) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder::new(s3_client, sqs_client)
    }

    pub fn send_message(&self) -> String {
        "Sent Message!".to_string()
    }

    pub fn always_s3_message(&self) -> String {
        "Always S3 is: ".to_string() + &self.always_s3.to_string()
    }
}

pub struct SqsExtendedClientBuilder {
    s3_client: aws_sdk_s3::Client,
    sqs_client: aws_sdk_sqs::Client,
    always_s3: bool
}

impl SqsExtendedClientBuilder {
    pub fn new(s3_client: aws_sdk_s3::Client, sqs_client: aws_sdk_sqs::Client) -> SqsExtendedClientBuilder {
        SqsExtendedClientBuilder{
            s3_client,
            sqs_client,
            always_s3: false
        }
    }

    pub fn always_s3(mut self) -> SqsExtendedClientBuilder {
        self.always_s3 = true;
        self
    }

    pub fn build(self) -> SqsExtendedClient {
        SqsExtendedClient {s3_client: self.s3_client, sqs_client: self.sqs_client, always_s3: self.always_s3}
    }
}