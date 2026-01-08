use crate::proto::proto_asset_service_client::ProtoAssetServiceClient;
use crate::{AssetError, AssetResult};
use euro_auth::AuthedChannel;
use uuid::Uuid;

pub struct AssetManager {
    client: ProtoAssetServiceClient<AuthedChannel>,
}

impl AssetManager {
    pub async fn new() -> AssetResult<Self> {
        let channel = euro_auth::get_authed_channel().await;
        let client = ProtoAssetServiceClient::new(channel);

        Ok(Self { client })
    }

    pub async fn create_asset(&self, content: Vec<u8>, activity_id: Uuid) -> AssetResult<Uuid> {
        todo!()
        // let request = crate::proto::CreateAssetRequest {
        //     content,
        //     activity_id: activity_id.to_string(),
        // };

        // let response = self.client.create_asset(request).await?;

        // Ok(Uuid::parse_str(&response.id)?)
    }
}
