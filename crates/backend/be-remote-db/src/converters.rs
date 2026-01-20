use crate::error::DbError;
use crate::types::Conversation;
use chrono::DateTime;
use prost_types::Timestamp;
use uuid::Uuid;

// #[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
// pub struct Conversation {
//     pub id: Uuid,
//     pub user_id: Uuid,
//     pub title: Option<String>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
// }

impl TryFrom<proto_gen::conversation::Conversation> for Conversation {
    type Error = DbError;

    fn try_from(value: proto_gen::conversation::Conversation) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&value.id).map_err(|e| DbError::Internal(e.to_string()))?;
        let user_id =
            Uuid::parse_str(&value.user_id).map_err(|e| DbError::Internal(e.to_string()))?;
        let created_at = value.created_at.expect("Missing created_at");
        let created_at = DateTime::from_timestamp(created_at.seconds, created_at.nanos as u32)
            .expect("Invalid timestamp");

        let updated_at = value.updated_at.expect("Missing updated_at");
        let updated_at = DateTime::from_timestamp(updated_at.seconds, updated_at.nanos as u32)
            .expect("Invalid timestamp");

        Ok(Conversation {
            id,
            user_id,
            title: Some(value.title),
            created_at,
            updated_at,
        })
    }
}

impl TryInto<proto_gen::conversation::Conversation> for Conversation {
    type Error = DbError;

    fn try_into(self) -> Result<proto_gen::conversation::Conversation, Self::Error> {
        let id = self.id.to_string();
        let user_id = self.user_id.to_string();
        let title = self.title.unwrap_or_default();

        Ok(proto_gen::conversation::Conversation {
            id,
            user_id,
            title,
            created_at: Some(Timestamp {
                seconds: self.created_at.timestamp(),
                nanos: self.created_at.timestamp_subsec_nanos() as i32,
            }),
            updated_at: Some(Timestamp {
                seconds: self.updated_at.timestamp(),
                nanos: self.updated_at.timestamp_subsec_nanos() as i32,
            }),
        })
    }
}
