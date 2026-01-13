//! Server-side implementation for the Activity Service.
//!
//! This module contains the gRPC server implementation and is only
//! available when the `server` feature is enabled.

use std::sync::Arc;

use anyhow::Result;
use be_auth_grpc::Claims;
use chrono::{DateTime, Utc};
use euro_remote_db::DatabaseManager;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use activity_models::proto::{
    Activity, ActivityResponse, DeleteActivityRequest, GetActivitiesByTimeRangeRequest,
    GetActivityRequest, InsertActivityRequest, ListActivitiesRequest, ListActivitiesResponse,
    UpdateActivityEndTimeRequest, UpdateActivityRequest,
};

pub use activity_models::proto::proto_activity_service_server::{
    ProtoActivityService, ProtoActivityServiceServer,
};

/// The main activity service
#[derive(Debug)]
pub struct ActivityService {
    db: Arc<DatabaseManager>,
}

impl ActivityService {
    /// Create a new ActivityService instance
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        info!("Creating new ActivityService instance");
        Self { db }
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

#[tonic::async_trait]
impl ProtoActivityService for ActivityService {
    async fn list_activities(
        &self,
        request: Request<ListActivitiesRequest>,
    ) -> Result<Response<ListActivitiesResponse>, Status> {
        info!("ListActivities request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (activities, total_count) = self
            .db
            .list_activities(user_id, limit, req.offset)
            .await
            .map_err(|e| {
            error!("Failed to list activities: {}", e);
            Status::internal("Failed to list activities")
        })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let activity_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let activity = self
            .db
            .get_activity_for_user(activity_id, user_id)
            .await
            .map_err(|e| {
                warn!("Activity not found: {}", e);
                Status::not_found("Activity not found")
            })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let id = req
            .id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let started_at = req
            .started_at
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| Status::invalid_argument("started_at is required"))?;

        let ended_at = req.ended_at.as_ref().and_then(timestamp_to_datetime);

        // TODO: Handle icon upload - for now, pass None for icon_asset_id
        // The req.icon bytes would need to be uploaded to the asset service first
        // to obtain a valid icon_asset_id
        let activity = self
            .db
            .create_activity(
                id,
                user_id,
                &req.name,
                None, // icon_asset_id - requires proper asset upload implementation
                &req.process_name,
                &req.window_title,
                started_at,
                ended_at,
            )
            .await
            .map_err(|e| {
                error!("Failed to create activity: {}", e);
                Status::internal("Failed to create activity")
            })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let activity_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let icon_asset_id = req
            .icon_asset_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| Status::invalid_argument(format!("Invalid icon asset ID: {}", e)))?;

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
            .map_err(|e| {
                error!("Failed to update activity: {}", e);
                Status::internal("Failed to update activity")
            })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let activity_id = Uuid::parse_str(&req.activity_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let ended_at = req
            .ended_at
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| Status::invalid_argument("ended_at is required"))?;

        self.db
            .update_activity_end_time(activity_id, user_id, ended_at)
            .await
            .map_err(|e| {
                error!("Failed to update activity end time: {}", e);
                Status::internal("Failed to update activity end time")
            })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let activity = self
            .db
            .get_last_active_activity(user_id)
            .await
            .map_err(|e| {
                error!("Failed to get last active activity: {}", e);
                Status::internal("Failed to get last active activity")
            })?;

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

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let activity_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        self.db
            .delete_activity(activity_id, user_id)
            .await
            .map_err(|e| {
                error!("Failed to delete activity: {}", e);
                Status::internal("Failed to delete activity")
            })?;

        debug!("Deleted activity {} for user {}", activity_id, user_id);

        Ok(Response::new(()))
    }

    async fn get_activities_by_time_range(
        &self,
        request: Request<GetActivitiesByTimeRangeRequest>,
    ) -> Result<Response<ListActivitiesResponse>, Status> {
        info!("GetActivitiesByTimeRange request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let start_time = req
            .start_time
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| Status::invalid_argument("start_time is required"))?;

        let end_time = req
            .end_time
            .as_ref()
            .and_then(timestamp_to_datetime)
            .ok_or_else(|| Status::invalid_argument("end_time is required"))?;

        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (activities, total_count) = self
            .db
            .get_activities_by_time_range(user_id, start_time, end_time, limit, req.offset)
            .await
            .map_err(|e| {
                error!("Failed to get activities by time range: {}", e);
                Status::internal("Failed to get activities by time range")
            })?;

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
