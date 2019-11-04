use crate::lib::api_client::{call, request_status, QueryResponseReply, ReadResponse};
use crate::lib::env::ClientEnv;
use crate::lib::error::{DfxError, DfxResult};
use crate::lib::message::UserMessage;
use crate::util::clap::validators;
use crate::util::print_idl_blob;
use clap::{App, Arg, ArgMatches, SubCommand};
use ic_http_agent::{Blob, CanisterId};
use serde_idl::{Encode, IDLArgs};
use tokio::runtime::Runtime;

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("call")
        .about(UserMessage::CallCanister.to_str())
        .arg(
            Arg::with_name("deployment_id")
                .takes_value(true)
                .help(UserMessage::DeploymentId.to_str())
                .required(true)
                .validator(validators::is_canister_id),
        )
        .arg(
            Arg::with_name("method_name")
                .help(UserMessage::MethodName.to_str())
                .required(true),
        )
        .arg(
            Arg::with_name("async")
                .help(UserMessage::AsyncResult.to_str())
                .long("async")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("type")
                .help(UserMessage::ArgumentType.to_str())
                .long("type")
                .takes_value(true)
                .requires("argument")
                .possible_values(&["string", "number", "idl"]),
        )
        .arg(
            Arg::with_name("argument")
                .help(UserMessage::ArgumentValue.to_str())
                .takes_value(true),
        )
}

pub fn exec<T>(env: &T, args: &ArgMatches<'_>) -> DfxResult
where
    T: ClientEnv,
{
    // Read the config.
    let canister_id = args
        .value_of("deployment_id")
        .unwrap()
        .parse::<CanisterId>()
        .map_err(|e| DfxError::InvalidArgument(format!("Invalid deployment ID: {}", e)))?;
    let method_name = args.value_of("method_name").unwrap();
    let arguments: Option<&str> = args.value_of("argument");
    let arg_type: Option<&str> = args.value_of("type");

    // Get the argument, get the type, convert the argument to the type and return
    // an error if any of it doesn't work.
    let arg_value = if let Some(a) = arguments {
        Some(Blob::from(match arg_type {
            Some("string") => Ok(Encode!(&a)),
            Some("number") => Ok(Encode!(&a.parse::<u64>().map_err(|e| {
                DfxError::InvalidArgument(format!(
                    "Argument is not a valid 64-bit unsigned integer: {}",
                    e
                ))
            })?)),
            Some("idl") | None => {
                let args: IDLArgs = a
                    .parse()
                    .map_err(|e| DfxError::InvalidArgument(format!("Invalid IDL: {}", e)))?;
                Ok(args.to_bytes().map_err(|e| {
                    DfxError::InvalidData(format!("Unable to convert IDL to bytes: {}", e))
                })?)
            }
            Some(v) => Err(DfxError::Unknown(format!("Invalid type: {}", v))),
        }?))
    } else {
        None
    };

    let client = env.get_client();
    let call_future = call(client, canister_id, method_name.to_owned(), arg_value);

    let mut runtime = Runtime::new().expect("Unable to create a runtime");
    let request_id = runtime.block_on(call_future)?;

    if args.is_present("async") {
        println!("0x{}", String::from(request_id));
        Ok(())
    } else {
        let request_status = request_status(env.get_client(), request_id);
        let mut runtime = Runtime::new().expect("Unable to create a runtime");
        match runtime.block_on(request_status) {
            Ok(ReadResponse::Pending) => {
                eprintln!("Pending");
                println!("0x{}", String::from(request_id));
                Ok(())
            }
            Ok(ReadResponse::Replied { reply }) => {
                if let Some(QueryResponseReply { arg: blob }) = reply {
                    print_idl_blob(&blob)
                        .map_err(|e| DfxError::InvalidData(format!("Invalid IDL blob: {}", e)))?;
                }
                Ok(())
            }
            Ok(ReadResponse::Rejected {
                reject_code,
                reject_message,
            }) => Err(DfxError::ClientError(reject_code, reject_message)),
            // TODO(SDK-446): remove this when moving api_client to ic_http_agent.
            Ok(ReadResponse::Unknown) => Err(DfxError::Unknown("Unknown response".to_owned())),
            Err(x) => Err(x),
        }
    }
}
