use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub login: String,
    #[serde(skip_serializing)]
    pub(super) access_token: RefCell<Option<Sensitive<String>>>,
}
