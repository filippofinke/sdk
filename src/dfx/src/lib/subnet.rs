use crate::lib::error::DfxResult;
use anyhow::anyhow;
use backoff::exponential::ExponentialBackoff;
use backoff::future::retry;
use backoff::SystemClock;
use candid::{CandidType, Deserialize, Principal};
use ic_agent::{Agent, AgentError};
use ic_utils::Canister;
use phantom_newtype::Id;
// type GetSubnetForCanisterRequest = record { "principal" : opt principal };
// type GetSubnetForCanisterResponse = record { subnet_id : opt principal };
// get_subnet_for_canister : (GetSubnetForCanisterRequest) -> (Result_4) query;
use ic_utils::call::SyncCall;

pub struct SubnetTag {}
/// A type representing a subnet's [`Principal`].
pub type SubnetId = Id<SubnetTag, Principal>;

pub const MAINNET_REGISTRY_CANISTER_ID: Principal =
    Principal::from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01]);

#[derive(CandidType)]
pub struct GetSubnetForCanisterRequest {
    pub principal: Option<Principal>,
}

#[derive(CandidType, Deserialize, Debug, Clone)]
pub struct GetSubnetForCanisterResponse {
    pub subnet_id: Option<Principal>,
}

pub async fn get_subnet_for_canister(agent: &Agent, canister_id: Principal) -> DfxResult<SubnetId> {
    let registry_canister = Canister::builder()
        .with_agent(agent)
        .with_canister_id(MAINNET_REGISTRY_CANISTER_ID)
        .build()?;

    let retry_policy: ExponentialBackoff<SystemClock> = ExponentialBackoff::default();

    retry(retry_policy, || async {
        let arg = GetSubnetForCanisterRequest {
            principal: Some(canister_id),
        };
        let result: Result<Result<GetSubnetForCanisterResponse, String>, AgentError> =
            registry_canister
                .query("get_subnet_for_canister")
                .with_arg(arg)
                .build()
                .call()
                .await
                .map(|(result,)| result);
        match result {
            Ok(Ok(GetSubnetForCanisterResponse {
                subnet_id: Some(subnet_id),
            })) => Ok(SubnetId::from(subnet_id)),
            Ok(Ok(GetSubnetForCanisterResponse { subnet_id: None })) => Err(
                backoff::Error::permanent(anyhow!("no subnet found for canister {}", &canister_id)),
            ),
            Ok(Err(text)) => Err(backoff::Error::permanent(anyhow!(
                "unable to determine subnet: {}",
                text
            ))),
            Err(agent_err) if retryable(&agent_err) => {
                Err(backoff::Error::transient(anyhow!(agent_err)))
            }
            Err(agent_err) => Err(backoff::Error::permanent(anyhow!(agent_err))),
        }
    })
    .await
}
fn retryable(agent_error: &AgentError) -> bool {
    matches!(
        agent_error,
        AgentError::TimeoutWaitingForResponse() | AgentError::TransportError(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_canister_id() {
        assert_eq!(
            MAINNET_REGISTRY_CANISTER_ID,
            Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap()
        );
    }
}
