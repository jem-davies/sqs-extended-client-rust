# sqs-extended-client-rust

[![codecov](https://codecov.io/gh/jem-davies/sqs-extended-client-rust/graph/badge.svg?token=81XM8DODBX)](https://codecov.io/gh/jem-davies/sqs-extended-client-rust)
![cratesio](https://img.shields.io/crates/v/sqs-extended-client.svg)
[![docs](https://docs.rs/sqs-extended-client/badge.svg)](https://docs.rs/sqs-extended-client/)

# THIS REPO IS A WIP

--------------------------------------------------------------------------------

`sqs-extended-client-rust` is an extension to the Amazon SQS Client that enables
sending and receiving messages up to 2GB via Amazon S3. It is very similar to the
[SQS Extended Client for Java](https://github.com/awslabs/amazon-sqs-java-extended-client-lib), but has an adjusted API to be more Rust friendly.

## Installation

```sh
cargo add sqs-extended-client
```

## Usage

```rust
use aws_config::BehaviorVersion;
use aws_sdk_sqs::operation::{receive_message::{builders::ReceiveMessageFluentBuilder, ReceiveMessageOutput}, send_message::builders::SendMessageFluentBuilder};
use sqs_extended_client::{SqsExtendedClient, SqsExtendedClientBuilder, SqsExtendedClientError};

#[::tokio::main]
async fn main() -> Result<(), SqsExtendedClientError> {
    // get a sqs_queue_url & s3_bucket_name
    let sqs_queue_url: String = "rusty-extended-queue".to_string();
    let s3_bucket_name: String = "rusty-bucket".to_string();

    // create an aws_config
    let config: aws_config::SdkConfig =
        aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;

    // create an s3 client
    let s3_client: aws_sdk_s3::Client = aws_sdk_s3::Client::new(&config);

    // create an sqs client
    let sqs_client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&config);

    // use the SqsExtendedClientBuilder to create a SqsExtendedClient
    let sqs_extended_client: SqsExtendedClient =
        SqsExtendedClientBuilder::new(s3_client, sqs_client.clone())
            .with_s3_bucket_name(s3_bucket_name)
            .build();

    // Create a large message of size 12MiB
    let large_message: String = "X".repeat(12 * 1024 * 1024);

    // Create a "send message request" 
    let msg_input: SendMessageFluentBuilder = sqs_client
        .send_message()
        .queue_url(&sqs_queue_url)
        .message_body(large_message);
    
    // Sqs Extended Client Send message
    sqs_extended_client.send_message(msg_input).await?;

    // Create a "receive message request" 
    let receive_msg: ReceiveMessageFluentBuilder = sqs_client.receive_message().queue_url(sqs_queue_url);

    // Sqs Extended Client Receive message
    let rcv:ReceiveMessageOutput = sqs_extended_client.receive_message(receive_msg).await?;

    // Print the large message:
    for message in rcv.messages.unwrap_or_default() {
        println!("Got the message: {:#?}", message);
    }

    // There is also: 

    // sqs_extended_client.delete_message
    // sqs_extended_client.change_message_visibility

    // and many options in the SqsExtendedClientBuilder

    Ok(())
}
```

--------------------------------------------------------------------------------

## Road to Release 1.0.0

- IMPLEMENTATIONS
    - client_builder                ✅
    - send_message                  ✅
    - receive_message               ✅
    - delete_message                ✅
    - change_message_visibility     ✅
- TODOs                               
    - match -> let-else             ✅
    - 100% error handling           ❌
    - review rcv_msg() looping etc. ❌
- UNIT TESTS
    - for private/pure fns          ✅
    - 2 small TODOs                 ✅
    - mocked public/non-pure fns    ❌
- LOCAL STACK INT. TESTS            
    - send_message                  ✅
    - receive_message               ✅
    - delete_message                ❌
    - change_message_visibility     ❌
    - send_and_receive_multiple     ❌
- MVP DEV EXP. 
    - github workflow linting       ❌
    - github workflow tests         ❌
    - github workflow release       ❌
    - github workflow converage     ✅ -> Doesn't work bogus line count 
    - docs                          ❌
    - delete main.rs -> README.md   ✅
- UPDATE DEPS
    - increase limit to 1MiB        ❌

## Future

- Add Batch functions
- Deal with SQS & Lambda events
- `.to_string()` & `.clone()`
- marshall_json -> serde
- async handling of rcv_msg
- msg attr names = "ALL"