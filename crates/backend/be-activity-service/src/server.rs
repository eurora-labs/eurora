//! Server-side implementation for the Activity Service.
//!
//! This module contains the gRPC server implementation and is only
//! available when the `server` feature is enabled.

use std::sync::Arc;

use be_auth_grpc::Claims;
use chrono::{DateTime, Utc};
use euro_remote_db::DatabaseManager;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{ActivityResult, ActivityServiceError};

use activity_models::proto::{
    Activity, ActivityResponse, DeleteActivityRequest, GetActivitiesByTimeRangeRequest,
    GetActivityRequest, InsertActivityRequest, ListActivitiesRequest, ListActivitiesResponse,
    UpdateActivityEndTimeRequest, UpdateActivityRequest,
};

pub use activity_models::proto::proto_activity_service_server::{
    ProtoActivityService, ProtoActivityServiceServer,
};

use be_storage::StorageService;

/// The main activity service
#[derive(Debug)]
pub struct ActivityService {
    db: Arc<DatabaseManager>,
    storage: Arc<StorageService>,
}

impl ActivityService {
    /// Create a new ActivityService instance
    pub fn new(db: Arc<DatabaseManager>, storage: Arc<StorageService>) -> Self {
        info!("Creating new ActivityService instance");
        Self { db, storage }
    }

    /// Create a new ActivityService from environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ActivityServiceError::Storage`] if the storage service
    /// cannot be initialized from environment variables.
    pub fn from_env(db: Arc<DatabaseManager>) -> ActivityResult<Self> {
        let storage = StorageService::from_env()?;
        Ok(Self::new(db, Arc::new(storage)))
    }

    /// Convert a database Activity to a proto Activity
    fn db_activity_to_proto(activity: &euro_remote_db::Activity) -> Activity {
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

/// Convert DateTime<Utc> to prost_types::Timestamp
fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// Convert prost_types::Timestamp to DateTime<Utc>
fn timestamp_to_datetime(ts: &Timestamp) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
}

/// Extract and validate claims from a gRPC request.
fn extract_claims<T>(request: &Request<T>) -> Result<&Claims, ActivityServiceError> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| ActivityServiceError::unauthenticated("Missing claims"))
}

/// Parse a user ID from claims.
fn parse_user_id(claims: &Claims) -> Result<Uuid, ActivityServiceError> {
    Uuid::parse_str(&claims.sub).map_err(|e| ActivityServiceError::invalid_uuid("user_id", e))
}

/// Parse an activity ID from a string.
fn parse_activity_id(id: &str) -> Result<Uuid, ActivityServiceError> {
    Uuid::parse_str(id).map_err(|e| ActivityServiceError::invalid_uuid("activity_id", e))
}

/// Parse an optional UUID from a string.
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
        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (activities, total_count) = self
            .db
            .list_activities(user_id, limit, req.offset)
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
            total_count,
        }))
    }

    async fn get_activity(
        &self,
        request: Request<GetActivityRequest>,
    ) -> Result<Response<ActivityResponse>, Status> {
        info!("GetActivity request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();
        let activity_id = parse_activity_id(&req.id)?;

        let activity = self
            .db
            .get_activity_for_user(activity_id, user_id)
            .await
            .map_err(ActivityServiceError::from)?;

        debug!("Retrieved activity {} for user {}", activity_id, user_id);

        Ok(Response::new(ActivityResponse {
            activity: Some(Self::db_activity_to_proto(&activity)),
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

        // Upload icon to storage if provided
        let icon_id = match req.icon {
            Some(icon) => {
                let id = Uuid::now_v7();

                self.storage
                    .upload(&user_id, &id, &icon, "image/png")
                    .await
                    .map_err(ActivityServiceError::from)?;
                Some(id)
            }
            None => None,
        };

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
            .create_activity(
                id,
                user_id,
                &req.name,
                icon_id,
                &req.process_name,
                &req.window_title,
                started_at,
                ended_at,
            )
            .await
            .map_err(ActivityServiceError::from)?;
        info!("Created activity at: {:?}", activity.created_at);

        debug!("Created activity {} for user {}", activity.id, user_id);

        Ok(Response::new(ActivityResponse {
            activity: Some(Self::db_activity_to_proto(&activity)),
        }))
    }

    async fn update_activity(
        &self,
        request: Request<UpdateActivityRequest>,
    ) -> Result<Response<ActivityResponse>, Status> {
        info!("UpdateActivity request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let activity_id = parse_activity_id(&req.id)?;
        let icon_asset_id = parse_optional_uuid(req.icon_asset_id.as_ref(), "icon_asset_id")?;

        let started_at = req.started_at.as_ref().and_then(timestamp_to_datetime);
        let ended_at = req.ended_at.as_ref().and_then(timestamp_to_datetime);

        let activity = self
            .db
            .update_activity(
                activity_id,
                user_id,
                req.name.as_deref(),
                icon_asset_id,
                req.process_name.as_deref(),
                req.window_title.as_deref(),
                started_at,
                ended_at,
            )
            .await
            .map_err(ActivityServiceError::from)?;

        debug!("Updated activity {} for user {}", activity_id, user_id);

        Ok(Response::new(ActivityResponse {
            activity: Some(Self::db_activity_to_proto(&activity)),
        }))
    }

    async fn update_activity_end_time(
        &self,
        request: Request<UpdateActivityEndTimeRequest>,
    ) -> Result<Response<()>, Status> {
        info!("UpdateActivityEndTime request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let activity_id = parse_activity_id(&req.activity_id)?;

        let ended_at = req
            .ended_at
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| ActivityServiceError::invalid_timestamp("ended_at"))?;

        self.db
            .update_activity_end_time(activity_id, user_id, ended_at)
            .await
            .map_err(ActivityServiceError::from)?;

        debug!(
            "Updated end time for activity {} for user {}",
            activity_id, user_id
        );

        Ok(Response::new(()))
    }

    async fn get_last_active_activity(
        &self,
        request: Request<()>,
    ) -> Result<Response<ActivityResponse>, Status> {
        info!("GetLastActiveActivity request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let activity = self
            .db
            .get_last_active_activity(user_id)
            .await
            .map_err(ActivityServiceError::from)?;

        debug!("Retrieved last active activity for user {}", user_id);

        Ok(Response::new(ActivityResponse {
            activity: activity.as_ref().map(Self::db_activity_to_proto),
        }))
    }

    async fn delete_activity(
        &self,
        request: Request<DeleteActivityRequest>,
    ) -> Result<Response<()>, Status> {
        info!("DeleteActivity request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let activity_id = parse_activity_id(&req.id)?;

        self.db
            .delete_activity(activity_id, user_id)
            .await
            .map_err(ActivityServiceError::from)?;

        debug!("Deleted activity {} for user {}", activity_id, user_id);

        Ok(Response::new(()))
    }

    async fn get_activities_by_time_range(
        &self,
        request: Request<GetActivitiesByTimeRangeRequest>,
    ) -> Result<Response<ListActivitiesResponse>, Status> {
        info!("GetActivitiesByTimeRange request received");

        let claims = extract_claims(&request)?;
        let user_id = parse_user_id(claims)?;

        let req = request.into_inner();

        let start_time = req
            .start_time
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| ActivityServiceError::invalid_timestamp("start_time"))?;

        let end_time = req
            .end_time
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| ActivityServiceError::invalid_timestamp("end_time"))?;

        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (activities, total_count) = self
            .db
            .get_activities_by_time_range(user_id, start_time, end_time, limit, req.offset)
            .await
            .map_err(ActivityServiceError::from)?;

        let proto_activities: Vec<Activity> =
            activities.iter().map(Self::db_activity_to_proto).collect();

        debug!(
            "Listed {} activities in time range for user {}",
            proto_activities.len(),
            user_id
        );

        Ok(Response::new(ListActivitiesResponse {
            activities: proto_activities,
            total_count,
        }))
    }
}
