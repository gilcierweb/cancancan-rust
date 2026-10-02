use cancancan_core::{DbValue, SubjectInstance};
use std::collections::HashMap;

pub struct User {
    pub id: i64,
    pub name: String,
}

impl SubjectInstance for User {
    fn subject_type(&self) -> &str {
        "User"
    }

    fn attribute(&self, name: &str) -> Option<DbValue> {
        match name {
            "id" => Some(DbValue::Int(self.id)),
            "name" => Some(DbValue::Str(self.name.clone())),
            _ => None,
        }
    }
}

pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub published: bool,
    pub title: String,
    pub author: Option<User>,
}

impl Post {
    pub fn owned(id: i64, user_id: i64) -> Self {
        Self {
            id,
            user_id,
            published: false,
            title: format!("post {id}"),
            author: None,
        }
    }
}

impl SubjectInstance for Post {
    fn subject_type(&self) -> &str {
        "Post"
    }

    fn attribute(&self, name: &str) -> Option<DbValue> {
        match name {
            "id" => Some(DbValue::Int(self.id)),
            "user_id" => Some(DbValue::Int(self.user_id)),
            "published" => Some(DbValue::Bool(self.published)),
            "title" => Some(DbValue::Str(self.title.clone())),
            _ => None,
        }
    }

    fn association(&self, name: &str) -> Option<&dyn SubjectInstance> {
        match name {
            "author" => self.author.as_ref().map(|user| user as &dyn SubjectInstance),
            _ => None,
        }
    }
}

pub struct MapSubject {
    pub type_name: String,
    pub fields: HashMap<String, DbValue>,
}

impl SubjectInstance for MapSubject {
    fn subject_type(&self) -> &str {
        &self.type_name
    }

    fn attribute(&self, name: &str) -> Option<DbValue> {
        self.fields.get(name).cloned()
    }
}
