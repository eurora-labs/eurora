use crate::proto::proto_asset_service_client::ProtoAssetServiceClient;
use crate::{AssetError, AssetResult};
use euro_auth::AuthedChannel;

pub struct AssetManager {
    client: ProtoAssetServiceClient<AuthedChannel>,
}

impl AssetManager {
    pub async fn new() -> AssetResult<Self> {
        let channel = euro_auth::get_authed_channel().await;
        let client = ProtoAssetServiceClient::new(channel);

        Ok(Self { client })
    }
}
