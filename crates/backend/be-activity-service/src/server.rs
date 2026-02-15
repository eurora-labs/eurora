use std::sync::Arc;

use be_asset::AssetService;
use be_authz::{extract_claims, parse_user_id};
use be_remote_db::{
    DatabaseManager, ListActivities, NewActivity, PaginationParams, UpdateActivity,
};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use proto_gen::asset::CreateAssetRequest;
use tonic::{Request, Response, Status};
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{ActivityResult, ActivityServiceError};

use proto_gen::activity::{
    Activity, ActivityResponse, InsertActivityRequest, ListActivitiesRequest,
    ListActivitiesResponse,
};

pub use proto_gen::activity::proto_activity_service_server::{
    ProtoActivityService, ProtoActivityServiceServer,
};

#[derive(Debug)]
pub struct ActivityService {
    db: Arc<DatabaseManager>,
    asset_service: Arc<AssetService>,
}

impl ActivityService {
    pub fn new(db: Arc<DatabaseManager>, asset: Arc<AssetService>) -> Self {
        info!("Creating new ActivityService instance");
        Self {
            db,
            asset_service: asset,
        }
    }

    pub fn from_env(db: Arc<DatabaseManager>) -> ActivityResult<Self> {
        let asset = AssetService::from_env(db.clone()).map_err(ActivityServiceError::Asset)?;

        Ok(Self::new(db, Arc::new(asset)))
    }

    fn db_activity_to_proto(activity: &be_remote_db::Activity) -> Activity {
        Activity {
            id: activity.id.to_string(),
            name: activity.name.clone(),
            icon_asset_id: activity.icon_asset_id.map(|id| id.to_string()),
            process_name: Some(activity.process_name.clone()),
            window_title: Some(activity.window_title.clone()),
            started_at: Some(datetime_to_timestamp(activity.started_at)),
            ended_at: activity.ended_at.map(datetime_to_timestamp),
            created_at: Some(datetime_to_timestamp(activity.created_at)),
            updated_at: Some(datetime_to_timestamp(activity.updated_at)),
        }
    }
}

fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn timestamp_to_datetime(ts: &Timestamp) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
}

fn parse_optional_uuid(
    value: Option<&String>,
    field: &'static str,
) -> Result<Option<Uuid>, ActivityServiceError> {
    value
        .map(|s| Uuid::parse_str(s).map_err(|e| ActivityServiceError::invalid_uuid(field, e)))
        .transpose()
}

#[tonic::async_trait]
impl ProtoActivityService for ActivityService {
    async fn list_activities(
        &self,
        request: Request<ListActivitiesRequest>,
    ) -> Result<Response<ListActivitiesResponse>, Status> {
        info!("ListActivities request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let activities = self
            .db
            .list_activities(
                ListActivities { user_id },
                PaginationParams::new(req.offset, req.limit, "DESC".to_string()),
            )
            .await
            .map_err(ActivityServiceError::from)?;

        let proto_activities: Vec<Activity> =
            activities.iter().map(Self::db_activity_to_proto).collect();

        debug!(
            "Listed {} activities for user {}",
            proto_activities.len(),
            user_id
        );

        Ok(Response::new(ListActivitiesResponse {
            activities: proto_activities,
        }))
    }

    async fn insert_activity(
        &self,
        request: Request<InsertActivityRequest>,
    ) -> Result<Response<ActivityResponse>, Status> {
        info!("InsertActivity request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let id = parse_optional_uuid(req.id.as_ref(), "activity_id")?;

        let started_at = req
            .started_at
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| ActivityServiceError::invalid_timestamp("started_at"))?;

        let ended_at = req.ended_at.as_ref().and_then(timestamp_to_datetime);
        info!("Ended at: {:?}", ended_at);

        let activity = self
            .db
            .create_activity(NewActivity {
                id,
                user_id,
                name: req.name.clone(),
                icon_asset_id: None,
                process_name: req.process_name.clone(),
                window_title: req.window_title.clone(),
                started_at,
                ended_at,
            })
            .await
            .map_err(ActivityServiceError::from)?;
        info!("Created activity at: {:?}", activity.created_at);
        let icon_id = match req.icon {
            Some(icon) => {
                let icon_response = self
                    .asset_service
                    .create_asset(
                        CreateAssetRequest {
                            name: "icon".to_string(),
                            content: icon,
                            mime_type: "image/png".to_string(),
                            metadata: None,
                            activity_id: None,
                        },
                        user_id,
                    )
                    .await
                    .map_err(ActivityServiceError::Asset)?;

                match icon_response.asset {
                    Some(asset) => Some(Uuid::parse_str(&asset.id).unwrap()),
                    None => None,
                }
            }
            None => None,
        };

        self.db
            .update_activity(UpdateActivity {
                id: activity.id,
                user_id,
                icon_asset_id: icon_id,
                ..Default::default()
            })
            .await
            .map_err(ActivityServiceError::Database)?;

        debug!("Created activity {} for user {}", activity.id, user_id);

        Ok(Response::new(ActivityResponse {
            activity: Some(Self::db_activity_to_proto(&activity)),
        }))
    }
}
